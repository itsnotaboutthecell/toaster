//! LLM manager — owns the in-process GGUF lifecycle.
//!
//! Responsibilities:
//! - Catalog discovery + download-status refresh (delegates to `ModelManager`).
//! - Lazy-load a selected GGUF on first `complete()` call; unload after the
//!   existing `model_unload_timeout` setting elapses.
//! - Expose a `complete()` entry point used by
//!   [`managers::cleanup::llm_dispatch::DispatchBackend::LocalGguf`].
//!
//! Everything behind the `LlmBackend` trait is abstracted over so tests
//! and default cargo-check builds do not pull in `llama-cpp-2`.
//!
//! As of the unified-model-catalog feature (R-002 / R-004), all catalog
//! data and download/delete/cancel operations flow through the shared
//! `managers::model::catalog::post_processor` entries + `ModelManager`.
//! The prior `managers::llm::{catalog,download}` modules are deleted.

pub mod inference;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use anyhow::{anyhow, Result};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use inference::{CompletionRequest, CompletionResponse, LlmBackend};

use crate::managers::model::{
    catalog as unified_catalog, DownloadStatus, ModelCategory, ModelDownloadProgress, ModelInfo,
    ModelManager,
};

/// Runtime view of a post-processor catalog entry + download status.
///
/// Retained for the deprecated `list_llm_models` Tauri shim (see
/// `commands::llm_models`). New code should consume `ModelInfo` directly
/// via `commands::models::get_models(Some(PostProcessor))`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LlmModelInfo {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub filename: String,
    pub download_url: String,
    pub sha256: String,
    pub quantization: String,
    pub size_bytes: u64,
    pub context_length: u32,
    pub recommended_ram_gb: u32,
    pub is_recommended_default: bool,
    pub is_downloaded: bool,
    pub is_downloading: bool,
    pub partial_size: u64,
}

impl LlmModelInfo {
    /// Build from a unified `ModelInfo` record. PostProcessor entries in
    /// the unified catalog carry an `llm_metadata` block with the fields
    /// needed here.
    pub fn from_model_info(m: ModelInfo) -> Self {
        let llm = m.llm_metadata.clone().unwrap_or_default();
        Self {
            id: m.id,
            display_name: m.name,
            description: m.description,
            filename: m.filename,
            download_url: m.url.unwrap_or_default(),
            sha256: m.sha256.unwrap_or_default(),
            quantization: llm.quantization,
            size_bytes: m.size_mb.saturating_mul(1_048_576),
            context_length: llm.context_length,
            recommended_ram_gb: llm.recommended_ram_gb,
            is_recommended_default: m.is_recommended,
            is_downloaded: m.is_downloaded,
            is_downloading: m.is_downloading,
            partial_size: m.partial_size,
        }
    }
}

/// Pull the (context_length, recommended_ram_gb, size_bytes) tuple from a
/// post-processor `ModelInfo`. Centralized so every call site uses the
/// same fallbacks when `llm_metadata` is missing (which should not happen
/// for a curated PostProcessor entry — but we defend against it).
fn llm_fields(m: &ModelInfo) -> (u32, u32, u64) {
    let meta = m.llm_metadata.as_ref();
    (
        meta.map(|x| x.context_length).unwrap_or(0),
        meta.map(|x| x.recommended_ram_gb).unwrap_or(0),
        m.size_mb.saturating_mul(1_048_576),
    )
}

/// Total system RAM probe. Abstracted so the "insufficient RAM" test can
/// inject a small value without tying the test to the real host.
pub trait RamProbe: Send + Sync {
    fn total_ram_bytes(&self) -> u64;
}

pub struct RealRamProbe;

impl RamProbe for RealRamProbe {
    fn total_ram_bytes(&self) -> u64 {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        // sysinfo reports bytes in 0.33+.
        sys.total_memory()
    }
}

pub struct FixedRamProbe(pub u64);

impl RamProbe for FixedRamProbe {
    fn total_ram_bytes(&self) -> u64 {
        self.0
    }
}

/// Manager state. Mirrors the `ModelManager` shape for UX parity.
///
/// After the unified-model-catalog work, the canonical on-disk catalog +
/// download pipeline live on `Arc<ModelManager>`. `LlmManager` holds:
/// - an optional shared `ModelManager` for download / delete / cancel,
/// - its own cached `llm_dir` for file probes,
/// - a loaded-model slot + idle tracker for the in-process GGUF runtime,
/// - a `RamProbe` for the RAM budget check on `ensure_loaded`.
///
/// Tests use the cheap `with_probes` constructor which passes `None` for
/// the shared `ModelManager` — they exercise the load/unload path only.
pub struct LlmManager {
    llm_dir: PathBuf,
    model_manager: Option<Arc<ModelManager>>,
    loaded: Mutex<Option<LoadedModel>>,
    last_used: AtomicU64,
    ram_probe: Arc<dyn RamProbe>,
}

struct LoadedModel {
    id: String,
    backend: Arc<Mutex<dyn LlmBackend>>,
    #[allow(dead_code)]
    context_length: u32,
}

impl LlmManager {
    /// Production constructor. Uses the real RAM probe and delegates to
    /// the shared `ModelManager` for all on-disk operations.
    pub fn new(model_manager: Arc<ModelManager>) -> Result<Self> {
        let llm_dir = model_manager.llm_dir().clone();
        Ok(Self {
            llm_dir,
            model_manager: Some(model_manager),
            loaded: Mutex::new(None),
            last_used: AtomicU64::new(0),
            ram_probe: Arc::new(RealRamProbe),
        })
    }

    /// Test constructor with an injected RAM probe. No `ModelManager`:
    /// download / delete / cancel return errors if invoked in this mode.
    pub fn with_probes(llm_dir: PathBuf, ram_probe: Arc<dyn RamProbe>) -> Result<Self> {
        if !llm_dir.exists() {
            std::fs::create_dir_all(&llm_dir)?;
        }
        Ok(Self {
            llm_dir,
            model_manager: None,
            loaded: Mutex::new(None),
            last_used: AtomicU64::new(0),
            ram_probe,
        })
    }

    pub fn llm_dir(&self) -> &PathBuf {
        &self.llm_dir
    }

    fn model_manager(&self) -> Result<&Arc<ModelManager>> {
        self.model_manager
            .as_ref()
            .ok_or_else(|| anyhow!("LlmManager constructed without ModelManager"))
    }

    pub fn list_models(&self) -> Vec<LlmModelInfo> {
        let Some(mm) = self.model_manager.as_ref() else {
            // Test mode: read directly from the static post-processor catalog.
            return unified_catalog::post_processor_entries()
                .into_iter()
                .map(LlmModelInfo::from_model_info)
                .collect();
        };
        mm.get_available_models()
            .into_iter()
            .filter(|m| m.category == ModelCategory::PostProcessor)
            .map(LlmModelInfo::from_model_info)
            .collect()
    }

    pub fn get_model(&self, model_id: &str) -> Option<LlmModelInfo> {
        let Some(mm) = self.model_manager.as_ref() else {
            return unified_catalog::find_post_processor(model_id).map(LlmModelInfo::from_model_info);
        };
        mm.get_model_info(model_id)
            .filter(|m| m.category == ModelCategory::PostProcessor)
            .map(LlmModelInfo::from_model_info)
    }

    /// Re-scan via the shared `ModelManager`. Kept as a no-op shim —
    /// `ModelManager` refreshes its own state on download completion.
    pub fn refresh_status(&self) -> Result<()> {
        Ok(())
    }

    /// Start downloading a catalog entry via the unified pipeline.
    ///
    /// Note: `ModelManager::download_model` is the emitter of canonical
    /// `model-download-progress` events. This helper additionally calls
    /// `on_progress` with start + end snapshots so the legacy
    /// `llm-model-download-progress` channel can keep working until the
    /// command layer is unified (umc-command-unify).
    pub async fn download(
        &self,
        model_id: &str,
        mut on_progress: impl FnMut(ModelDownloadProgress),
    ) -> Result<()> {
        let entry = unified_catalog::find_post_processor(model_id)
            .ok_or_else(|| anyhow!("Unknown LLM model id: {}", model_id))?;
        let (_ctx, _ram, size_bytes) = llm_fields(&entry);

        on_progress(ModelDownloadProgress {
            id: model_id.to_string(),
            category: ModelCategory::PostProcessor,
            downloaded_bytes: 0,
            total_bytes: size_bytes,
            percentage: 0.0,
            status: DownloadStatus::Started,
        });

        let result = self.model_manager()?.download_model(model_id).await;

        let downloaded_final = if result.is_ok() { size_bytes } else { 0 };
        on_progress(ModelDownloadProgress {
            id: model_id.to_string(),
            category: ModelCategory::PostProcessor,
            downloaded_bytes: downloaded_final,
            total_bytes: size_bytes,
            percentage: if size_bytes > 0 {
                (downloaded_final as f64 / size_bytes as f64) * 100.0
            } else {
                0.0
            },
            status: if result.is_ok() {
                DownloadStatus::Completed
            } else {
                DownloadStatus::Failed
            },
        });

        result
    }

    /// Flip the cancel flag on the unified pipeline for an in-flight download.
    pub fn cancel_download(&self, model_id: &str) -> Result<()> {
        self.model_manager()?.cancel_download(model_id).map_err(|e| {
            warn!("LLM cancel_download delegation failed: {}", e);
            e
        })
    }

    pub fn delete(&self, model_id: &str) -> Result<()> {
        let _entry = unified_catalog::find_post_processor(model_id)
            .ok_or_else(|| anyhow!("Unknown LLM model id: {}", model_id))?;
        // If the deleted model is currently loaded, drop it first.
        {
            let mut loaded = crate::lock_recovery::recover_lock(self.loaded.lock());
            if loaded.as_ref().map(|l| l.id.as_str()) == Some(model_id) {
                *loaded = None;
            }
        }
        self.model_manager()?.delete_model(model_id)
    }

    /// Lazy-load the requested model and run a completion.
    /// If a different model is currently loaded, unload it first.
    pub async fn complete(
        &self,
        model_id: &str,
        request: CompletionRequest,
    ) -> Result<CompletionResponse> {
        self.ensure_loaded(model_id)?;
        self.last_used.store(monotonic_secs(), Ordering::Relaxed);

        let backend = {
            let loaded = crate::lock_recovery::recover_lock(self.loaded.lock());
            loaded
                .as_ref()
                .map(|l| l.backend.clone())
                .ok_or_else(|| anyhow!("LLM model not loaded"))?
        };

        let req = request.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let mut b = backend.lock().unwrap_or_else(|e| e.into_inner());
            b.complete(&req)
        });
        handle.await.map_err(|e| anyhow!("join error: {}", e))?
    }

    /// Ensure the requested model is loaded. Enforces the RAM budget and
    /// existence check.
    fn ensure_loaded(&self, model_id: &str) -> Result<()> {
        {
            let loaded = crate::lock_recovery::recover_lock(self.loaded.lock());
            if loaded.as_ref().map(|l| l.id.as_str()) == Some(model_id) {
                return Ok(());
            }
        }
        let entry = unified_catalog::find_post_processor(model_id)
            .ok_or_else(|| anyhow!("Unknown LLM model id: {}", model_id))?;
        let (context_length, recommended_ram_gb, _size_bytes) = llm_fields(&entry);
        let gguf_path = self.llm_dir.join(&entry.filename);
        if !gguf_path.exists() {
            return Err(anyhow!(
                "LLM model file not downloaded: {} (expected at {})",
                entry.id,
                gguf_path.display()
            ));
        }
        // RAM budget check.
        let have = self.ram_probe.total_ram_bytes();
        let need = (recommended_ram_gb as u64) * 1024 * 1024 * 1024;
        if have < need {
            return Err(anyhow!(
                "Insufficient RAM for {}: have {} bytes, need at least {} ({} GiB)",
                entry.id,
                have,
                need,
                recommended_ram_gb
            ));
        }
        let backend = inference::load_backend(&gguf_path, context_length)?;
        {
            let mut loaded = crate::lock_recovery::recover_lock(self.loaded.lock());
            *loaded = Some(LoadedModel {
                id: entry.id.clone(),
                backend,
                context_length,
            });
        }
        debug!("LLM model loaded: {}", entry.id);
        Ok(())
    }

    /// Test hook: directly install a pre-built backend without touching disk.
    #[cfg(test)]
    pub(crate) fn install_backend_for_tests(
        &self,
        model_id: &str,
        backend: Arc<Mutex<dyn LlmBackend>>,
    ) {
        let entry = unified_catalog::find_post_processor(model_id)
            .expect("test model id must be in catalog");
        let (context_length, _, _) = llm_fields(&entry);
        let mut loaded = crate::lock_recovery::recover_lock(self.loaded.lock());
        *loaded = Some(LoadedModel {
            id: entry.id.clone(),
            backend,
            context_length,
        });
    }

    /// Check whether the manager currently has a loaded model.
    pub fn is_loaded(&self) -> bool {
        crate::lock_recovery::recover_lock(self.loaded.lock()).is_some()
    }

    /// Check whether the loaded model has been idle for more than `timeout`.
    /// Used by the unload scheduler.
    pub fn is_idle_for(&self, timeout: Duration) -> bool {
        if !self.is_loaded() {
            return false;
        }
        let last = self.last_used.load(Ordering::Relaxed);
        let now = monotonic_secs();
        Duration::from_secs(now.saturating_sub(last)) >= timeout
    }

    /// Drop the loaded backend. Idempotent.
    pub fn unload(&self) {
        let mut loaded = crate::lock_recovery::recover_lock(self.loaded.lock());
        if loaded.is_some() {
            debug!("Unloading LLM model");
            *loaded = None;
        }
    }

    /// One pass of the unload scheduler. Call this periodically (once a
    /// minute is fine); it drops the loaded model if idle.
    pub fn maybe_unload(&self, timeout: Duration) {
        if self.is_idle_for(timeout) {
            self.unload();
        }
    }
}

fn monotonic_secs() -> u64 {
    // We use a `OnceLock` start epoch so that `last_used = 0` correctly
    // represents "never used" in constructors. Under normal operation this
    // epoch is set once at process start.
    use std::sync::OnceLock;
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    let epoch = EPOCH.get_or_init(Instant::now);
    epoch.elapsed().as_secs()
}

/// Helper: emit the same download-failed payload the Whisper path uses
/// (see `commands::models::download_model`) so the frontend can render it
/// through the same toast + inline-error surfaces.
pub fn download_failed_payload(model_id: &str, error: &str) -> serde_json::Value {
    serde_json::json!({ "model_id": model_id, "error": error, "asset_kind": "llm" })
}

// Re-export for test consumers.
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use inference::MockBackend;

// Needed because the `lock_recovery` helper is internal to the crate.
#[allow(unused_imports)]
use crate::lock_recovery as _lock_recovery;

// Silence unused-import warnings when the `local-llm` feature is disabled.
#[allow(dead_code)]
fn _warn_silencer(warn_msg: &str) {
    warn!("{}", warn_msg);
}
