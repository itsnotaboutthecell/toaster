//! LLM manager — owns the in-process GGUF lifecycle and the catalog state.
//!
//! Responsibilities:
//! - Catalog discovery + download-status refresh (mirrors `ModelManager`).
//! - Lazy-load a selected GGUF on first `complete()` call; unload after the
//!   existing `model_unload_timeout` setting elapses.
//! - Expose a `complete()` entry point used by
//!   [`managers::cleanup::llm_dispatch::DispatchBackend::LocalGguf`].
//!
//! Everything behind the `LlmBackend` trait is abstracted over so tests
//! and default cargo-check builds do not pull in `llama-cpp-2`.
//!
//! As of the unified-model-catalog feature (R-004), downloads, deletes,
//! status refresh, and cancel flow through `Arc<ModelManager>`. The legacy
//! `managers::llm::download` module is kept alive for its unit tests only
//! (deletion deferred to umc-delete-llm-catalog).

pub mod catalog;
pub mod download;
pub mod inference;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use anyhow::{anyhow, Result};
use log::{debug, warn};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use catalog::{LlmCatalogEntry, LlmModelInfo};
pub use download::{DiskSpaceCheck, FreeSpaceProbe, LlmDownloadProgress, RealFreeSpaceProbe};
pub use inference::{CompletionRequest, CompletionResponse, LlmBackend};

use crate::managers::model::{ModelCategory, ModelManager};

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
            // Test mode: synthesize from the static catalog so consumers
            // still see a non-empty list.
            return catalog::catalog()
                .into_iter()
                .map(|e| LlmModelInfo::from(&e))
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
            return catalog::find_entry(model_id).map(|e| LlmModelInfo::from(&e));
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
    pub async fn download(
        &self,
        model_id: &str,
        mut on_progress: impl FnMut(LlmDownloadProgress),
    ) -> Result<()> {
        let entry = catalog::find_entry(model_id)
            .ok_or_else(|| anyhow!("Unknown LLM model id: {}", model_id))?;

        let start = Instant::now();
        on_progress(LlmDownloadProgress {
            model_id: model_id.to_string(),
            downloaded: 0,
            total: entry.size_bytes,
            percentage: 0.0,
            speed_bps: 0,
        });

        let result = self.model_manager()?.download_model(model_id).await;

        let downloaded_final = if result.is_ok() { entry.size_bytes } else { 0 };
        let elapsed = start.elapsed().as_secs_f64().max(0.001);
        let speed_bps = (downloaded_final as f64 / elapsed) as u64;
        on_progress(LlmDownloadProgress {
            model_id: model_id.to_string(),
            downloaded: downloaded_final,
            total: entry.size_bytes,
            percentage: if entry.size_bytes > 0 {
                (downloaded_final as f64 / entry.size_bytes as f64) * 100.0
            } else {
                0.0
            },
            speed_bps,
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
        let _entry = catalog::find_entry(model_id)
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
        let entry = catalog::find_entry(model_id)
            .ok_or_else(|| anyhow!("Unknown LLM model id: {}", model_id))?;
        let gguf_path = self.llm_dir.join(entry.filename());
        if !gguf_path.exists() {
            return Err(anyhow!(
                "LLM model file not downloaded: {} (expected at {})",
                entry.id,
                gguf_path.display()
            ));
        }
        // RAM budget check.
        let have = self.ram_probe.total_ram_bytes();
        let need = (entry.recommended_ram_gb as u64) * 1024 * 1024 * 1024;
        if have < need {
            return Err(anyhow!(
                "Insufficient RAM for {}: have {} bytes, need at least {} ({} GiB)",
                entry.id,
                have,
                need,
                entry.recommended_ram_gb
            ));
        }
        let backend = inference::load_backend(&gguf_path, entry.context_length)?;
        {
            let mut loaded = crate::lock_recovery::recover_lock(self.loaded.lock());
            *loaded = Some(LoadedModel {
                id: entry.id.clone(),
                backend,
                context_length: entry.context_length,
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
        let entry = catalog::find_entry(model_id).expect("test model id must be in catalog");
        let mut loaded = crate::lock_recovery::recover_lock(self.loaded.lock());
        *loaded = Some(LoadedModel {
            id: entry.id.clone(),
            backend,
            context_length: entry.context_length,
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
pub(crate) use inference::MockBackend;

// Needed because the `lock_recovery` helper is internal to the crate.
#[allow(unused_imports)]
use crate::lock_recovery as _lock_recovery;

// Silence unused-import warnings when the `local-llm` feature is disabled.
#[allow(dead_code)]
fn _warn_silencer(warn_msg: &str) {
    warn!("{}", warn_msg);
}
