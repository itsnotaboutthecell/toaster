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
//! See `features/local-llm-model-catalog/BLUEPRINT.md` for the "Unload
//! policy" and "Dispatch integration" decisions.

pub mod catalog;
pub mod download;
pub mod inference;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use catalog::{LlmCatalogEntry, LlmModelInfo};
pub use download::{DiskSpaceCheck, FreeSpaceProbe, LlmDownloadProgress, RealFreeSpaceProbe};
pub use inference::{CompletionRequest, CompletionResponse, LlmBackend};

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
pub struct LlmManager {
    llm_dir: PathBuf,
    available_models: Mutex<HashMap<String, LlmModelInfo>>,
    cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    loaded: Mutex<Option<LoadedModel>>,
    last_used: AtomicU64,
    ram_probe: Arc<dyn RamProbe>,
    free_space_probe: Arc<dyn FreeSpaceProbe>,
}

struct LoadedModel {
    id: String,
    backend: Arc<Mutex<dyn LlmBackend>>,
    #[allow(dead_code)]
    context_length: u32,
}

impl LlmManager {
    /// Production constructor. Uses the real RAM + disk-space probes.
    pub fn new(llm_dir: PathBuf) -> Result<Self> {
        Self::with_probes(
            llm_dir,
            Arc::new(RealRamProbe),
            Arc::new(RealFreeSpaceProbe),
        )
    }

    /// Test constructor with injected probes.
    pub fn with_probes(
        llm_dir: PathBuf,
        ram_probe: Arc<dyn RamProbe>,
        free_space_probe: Arc<dyn FreeSpaceProbe>,
    ) -> Result<Self> {
        if !llm_dir.exists() {
            std::fs::create_dir_all(&llm_dir)?;
        }
        let mut models = HashMap::new();
        for entry in catalog::catalog() {
            models.insert(entry.id.clone(), LlmModelInfo::from(&entry));
        }
        let manager = Self {
            llm_dir,
            available_models: Mutex::new(models),
            cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            loaded: Mutex::new(None),
            last_used: AtomicU64::new(0),
            ram_probe,
            free_space_probe,
        };
        manager.refresh_status()?;
        Ok(manager)
    }

    pub fn llm_dir(&self) -> &Path {
        &self.llm_dir
    }

    pub fn list_models(&self) -> Vec<LlmModelInfo> {
        crate::lock_recovery::recover_lock(self.available_models.lock())
            .values()
            .cloned()
            .collect()
    }

    pub fn get_model(&self, model_id: &str) -> Option<LlmModelInfo> {
        crate::lock_recovery::recover_lock(self.available_models.lock())
            .get(model_id)
            .cloned()
    }

    /// Re-scan `llm_dir` to refresh `is_downloaded` and `partial_size`
    /// flags on each catalog entry.
    pub fn refresh_status(&self) -> Result<()> {
        let mut models = crate::lock_recovery::recover_lock(self.available_models.lock());
        for model in models.values_mut() {
            let final_p = self.llm_dir.join(&model.filename);
            let part_p = self
                .llm_dir
                .join(format!("{}.partial", model.filename));
            model.is_downloaded = final_p.exists();
            model.is_downloading = false;
            model.partial_size = part_p.metadata().map(|m| m.len()).unwrap_or(0);
        }
        Ok(())
    }

    /// Start downloading a catalog entry.
    pub async fn download(
        &self,
        model_id: &str,
        mut on_progress: impl FnMut(LlmDownloadProgress),
    ) -> Result<()> {
        let entry = catalog::find_entry(model_id)
            .ok_or_else(|| anyhow!("Unknown LLM model id: {}", model_id))?;

        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut flags = crate::lock_recovery::recover_lock(self.cancel_flags.lock());
            flags.insert(model_id.to_string(), cancel.clone());
        }
        {
            let mut models =
                crate::lock_recovery::recover_lock(self.available_models.lock());
            if let Some(m) = models.get_mut(model_id) {
                m.is_downloading = true;
            }
        }

        let result = download::download_entry(
            self.free_space_probe.as_ref(),
            &self.llm_dir,
            &entry,
            cancel,
            |p| on_progress(p),
        )
        .await;

        // Always clear is_downloading + cancel flag.
        {
            let mut models =
                crate::lock_recovery::recover_lock(self.available_models.lock());
            if let Some(m) = models.get_mut(model_id) {
                m.is_downloading = false;
            }
        }
        {
            let mut flags = crate::lock_recovery::recover_lock(self.cancel_flags.lock());
            flags.remove(model_id);
        }
        // Refresh status regardless of result — the download may have
        // cleanly cancelled after writing a partial file.
        let _ = self.refresh_status();
        result
    }

    /// Flip the cancel flag for an in-flight download.
    pub fn cancel_download(&self, model_id: &str) -> Result<()> {
        let flags = crate::lock_recovery::recover_lock(self.cancel_flags.lock());
        if let Some(flag) = flags.get(model_id) {
            flag.store(true, Ordering::Relaxed);
            info!("LLM download cancellation requested for {}", model_id);
            Ok(())
        } else {
            Err(anyhow!("No active download for {}", model_id))
        }
    }

    pub fn delete(&self, model_id: &str) -> Result<()> {
        let entry = catalog::find_entry(model_id)
            .ok_or_else(|| anyhow!("Unknown LLM model id: {}", model_id))?;
        // If the deleted model is currently loaded, drop it first.
        {
            let mut loaded = crate::lock_recovery::recover_lock(self.loaded.lock());
            if loaded.as_ref().map(|l| l.id.as_str()) == Some(model_id) {
                *loaded = None;
            }
        }
        download::delete_asset(&self.llm_dir, &entry)?;
        self.refresh_status()?;
        Ok(())
    }

    /// Lazy-load the requested model and run a completion.
    /// If a different model is currently loaded, unload it first.
    pub async fn complete(
        &self,
        model_id: &str,
        request: CompletionRequest,
    ) -> Result<CompletionResponse> {
        self.ensure_loaded(model_id)?;
        self.last_used
            .store(monotonic_secs(), Ordering::Relaxed);

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
