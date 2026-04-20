use crate::settings::{get_settings, write_settings};
use anyhow::Result;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

pub mod catalog;
mod download;
mod hash;
pub mod hardware_profile;
pub mod recommendation;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
pub enum EngineType {
    #[default]
    Whisper,
    Parakeet,
    Moonshine,
    MoonshineStreaming,
    SenseVoice,
    GigaAM,
    Canary,
    Cohere,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq, Default)]
pub enum ModelCategory {
    #[default]
    Transcription,
    /// File-based analyzer models that are not transcription engines —
    /// currently only the Silero VAD ONNX consumed by
    /// `managers::splice::boundaries` and `managers::filler`.
    /// Filtered out of the ASR model picker.
    VoiceActivityDetection,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
pub struct TranscriptionMetadata {
    pub engine_type: EngineType,
    pub accuracy_score: f32,
    pub speed_score: f32,
    pub supports_translation: bool,
    pub supports_language_selection: bool,
    pub supported_languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub filename: String,
    pub url: Option<String>,
    pub sha256: Option<String>,
    pub size_mb: u64,
    pub is_downloaded: bool,
    pub is_downloading: bool,
    pub partial_size: u64,
    pub is_directory: bool,
    pub engine_type: EngineType,
    pub accuracy_score: f32,
    pub speed_score: f32,
    pub supports_translation: bool,
    pub is_recommended: bool,
    pub supported_languages: Vec<String>,
    pub supports_language_selection: bool,
    pub is_custom: bool,
    #[serde(default)]
    pub category: ModelCategory,
    /// Optional transcription-specific metadata block.
    /// Legacy JSON without this field deserializes as None.
    #[serde(default)]
    pub transcription_metadata: Option<TranscriptionMetadata>,
}

impl ModelInfo {
    /// Native input sample rate (Hz) for this model. Derived from the engine
    /// type rather than stored, because every engine in `transcribe-rs` today
    /// accepts 16 kHz — and adding a per-literal field would churn every
    /// `ModelInfo` builder. When adapters start declaring non-16 kHz rates
    /// (see `ModelCapabilities::native_input_sample_rate_hz`), this method
    /// should route through the adapter and eventually become a stored field.
    ///
    /// Falls back to `ASR_INPUT_SAMPLE_RATE_HZ_DEFAULT` (16 kHz) for
    /// engine types without a declared rate.
    pub fn input_sample_rate_hz(&self) -> u32 {
        use crate::audio_toolkit::constants::ASR_INPUT_SAMPLE_RATE_HZ_DEFAULT;
        match self.engine_type {
            EngineType::Whisper
            | EngineType::Parakeet
            | EngineType::Moonshine
            | EngineType::MoonshineStreaming
            | EngineType::SenseVoice
            | EngineType::GigaAM
            | EngineType::Canary
            | EngineType::Cohere => 16_000,
            #[allow(unreachable_patterns)]
            _ => ASR_INPUT_SAMPLE_RATE_HZ_DEFAULT,
        }
    }
}

/// Unified per-model download progress event. Emitted on the
/// `model-download-progress` channel for both transcription and
/// post-processor downloads (PRD R-005). The frontend keys progress
/// state by `id`; `category` lets consumers scope to a specific
/// catalog segment without a round-trip lookup.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ModelDownloadProgress {
    pub id: String,
    pub category: ModelCategory,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f64,
    pub status: DownloadStatus,
}

/// Lifecycle status for a download. Mirrors the progress-bar reducer
/// states the UI already tracks for Whisper models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    Started,
    Progress,
    Completed,
    Cancelled,
    Failed,
}

/// RAII guard that cleans up download state (`is_downloading` flag and cancel flag)
/// when dropped, unless explicitly disarmed. This ensures consistent cleanup on
/// every error path without requiring manual cleanup at each `?` or `return Err`.
struct DownloadCleanup<'a> {
    available_models: &'a Mutex<HashMap<String, ModelInfo>>,
    cancel_flags: &'a Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    model_id: String,
    disarmed: bool,
}

impl<'a> Drop for DownloadCleanup<'a> {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }
        {
            let mut models = crate::lock_recovery::recover_lock(self.available_models.lock());
            if let Some(model) = models.get_mut(self.model_id.as_str()) {
                model.is_downloading = false;
            }
        }
        crate::lock_recovery::recover_lock(self.cancel_flags.lock()).remove(&self.model_id);
    }
}

pub struct ModelManager {
    app_handle: AppHandle,
    models_dir: PathBuf,
    available_models: Mutex<HashMap<String, ModelInfo>>,
    cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    extracting_models: Arc<Mutex<HashSet<String>>>,
    /// Cached hardware probe. Populated once in `new()`; read-only
    /// afterwards. See `hardware_profile::probe` — the probe is
    /// local-only and non-fatal, so this is always `Some` after
    /// `new()` even on weird hardware.
    hardware_profile: hardware_profile::HardwareProfile,
}

impl ModelManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Create models directory in app data
        let app_data = crate::portable::app_data_dir(app_handle)
            .map_err(|e| anyhow::anyhow!("Failed to get app data dir: {}", e))?;
        let models_dir = app_data.join("models");

        if !models_dir.exists() {
            fs::create_dir_all(&models_dir)?;
        }

        let mut available_models = catalog::build_static_catalog();
        // Auto-discover custom Whisper models(.bin files) in the models directory
        if let Err(e) = catalog::discover_custom_whisper_models(&models_dir, &mut available_models)
        {
            warn!("Failed to discover custom models: {}", e);
        }

        let manager = Self {
            app_handle: app_handle.clone(),
            models_dir: models_dir.clone(),
            available_models: Mutex::new(available_models),
            cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            extracting_models: Arc::new(Mutex::new(HashSet::new())),
            hardware_profile: hardware_profile::probe(&models_dir),
        };

        // Migrate any bundled models to user directory
        manager.migrate_bundled_models()?;

        // Migrate GigaAM from single-file to directory format
        manager.migrate_gigaam_to_directory()?;

        // Check which models are already downloaded
        manager.update_download_status()?;

        // Auto-select a model if none is currently selected
        manager.auto_select_model_if_needed()?;

        Ok(manager)
    }

    pub fn get_available_models(&self) -> Vec<ModelInfo> {
        let models = crate::lock_recovery::recover_lock(self.available_models.lock());
        models.values().cloned().collect()
    }

    pub fn get_model_info(&self, model_id: &str) -> Option<ModelInfo> {
        let models = crate::lock_recovery::recover_lock(self.available_models.lock());
        models.get(model_id).cloned()
    }

    /// Read-only view of the cached hardware probe. Computed once in
    /// `new()`; safe to call from any thread.
    pub fn hardware_profile(&self) -> &hardware_profile::HardwareProfile {
        &self.hardware_profile
    }

    /// Convenience wrapper: run the pure `recommend_model` function
    /// against the cached profile and the current catalog snapshot.
    pub fn recommend_model(&self) -> recommendation::ModelRecommendation {
        let catalog = self.get_available_models();
        recommendation::recommend_model(&self.hardware_profile, &catalog)
    }

    /// On-disk root for a category. Transcription models live under
    /// `<app-data>/models/`.
    pub fn resolve_dir(&self, _category: &ModelCategory) -> &PathBuf {
        &self.models_dir
    }

    fn migrate_bundled_models(&self) -> Result<()> {
        // Check for bundled models and copy them to user directory
        let bundled_models = ["ggml-small.bin"]; // Add other bundled models here if any

        for filename in &bundled_models {
            let bundled_path = self.app_handle.path().resolve(
                format!("resources/models/{}", filename),
                tauri::path::BaseDirectory::Resource,
            );

            if let Ok(bundled_path) = bundled_path {
                if bundled_path.exists() {
                    let user_path = self.models_dir.join(filename);

                    // Only copy if user doesn't already have the model
                    if !user_path.exists() {
                        info!("Migrating bundled model {} to user directory", filename);
                        fs::copy(&bundled_path, &user_path)?;
                        info!("Successfully migrated {}", filename);
                    }
                }
            }
        }

        Ok(())
    }

    /// Migrate GigaAM from the old single-file format (giga-am-v3.int8.onnx)
    /// to the new directory format (giga-am-v3-int8/model.int8.onnx + vocab.txt).
    /// This was required by the transcribe-rs 0.3.x upgrade.
    fn migrate_gigaam_to_directory(&self) -> Result<()> {
        let old_file = self.models_dir.join("giga-am-v3.int8.onnx");
        let new_dir = self.models_dir.join("giga-am-v3-int8");

        if !old_file.exists() || new_dir.exists() {
            return Ok(());
        }

        info!("Migrating GigaAM from single-file to directory format");

        let vocab_path = self
            .app_handle
            .path()
            .resolve(
                "resources/models/gigaam_vocab.txt",
                tauri::path::BaseDirectory::Resource,
            )
            .map_err(|e| anyhow::anyhow!("Failed to resolve GigaAM vocab path: {}", e))?;

        info!(
            "Resolved vocab path: {:?} (exists: {})",
            vocab_path,
            vocab_path.exists()
        );
        info!("Old file: {:?} (exists: {})", old_file, old_file.exists());
        info!("New dir: {:?} (exists: {})", new_dir, new_dir.exists());

        fs::create_dir_all(&new_dir)?;
        fs::rename(&old_file, new_dir.join("model.int8.onnx"))?;
        fs::copy(&vocab_path, new_dir.join("vocab.txt"))?;

        // Clean up old partial file if it exists
        let old_partial = self.models_dir.join("giga-am-v3.int8.onnx.partial");
        if old_partial.exists() {
            let _ = fs::remove_file(&old_partial);
        }

        info!("GigaAM migration complete");
        Ok(())
    }

    fn update_download_status(&self) -> Result<()> {
        let mut models = crate::lock_recovery::recover_lock(self.available_models.lock());

        for model in models.values_mut() {
            let dir: &PathBuf = &self.models_dir;
            if model.is_directory {
                // For directory-based models, check if the directory exists
                let model_path = dir.join(&model.filename);
                let partial_path = dir.join(format!("{}.partial", &model.filename));
                let extracting_path = dir.join(format!("{}.extracting", &model.filename));

                // Clean up any leftover .extracting directories from interrupted extractions
                // But only if this model is NOT currently being extracted
                let is_currently_extracting = {
                    let extracting = crate::lock_recovery::recover_lock(self.extracting_models.lock());
                    extracting.contains(&model.id)
                };
                if extracting_path.exists() && !is_currently_extracting {
                    warn!("Cleaning up interrupted extraction for model: {}", model.id);
                    let _ = fs::remove_dir_all(&extracting_path);
                }

                model.is_downloaded = model_path.exists() && model_path.is_dir();
                model.is_downloading = false;

                // Get partial file size if it exists (for the .tar.gz being downloaded)
                if partial_path.exists() {
                    model.partial_size = partial_path.metadata().map(|m| m.len()).unwrap_or(0);
                } else {
                    model.partial_size = 0;
                }
            } else {
                // For file-based models (existing logic)
                let model_path = dir.join(&model.filename);
                let partial_path = dir.join(format!("{}.partial", &model.filename));

                model.is_downloaded = model_path.exists();
                model.is_downloading = false;

                // Get partial file size if it exists
                if partial_path.exists() {
                    model.partial_size = partial_path.metadata().map(|m| m.len()).unwrap_or(0);
                } else {
                    model.partial_size = 0;
                }
            }
        }

        Ok(())
    }

    fn auto_select_model_if_needed(&self) -> Result<()> {
        let mut settings = get_settings(&self.app_handle);

        // Clear stale selection: selected model is set but doesn't exist
        // in available_models (e.g. deleted custom model file)
        if !settings.selected_model.is_empty() {
            let models = crate::lock_recovery::recover_lock(self.available_models.lock());
            let exists = models.contains_key(&settings.selected_model);
            drop(models);

            if !exists {
                info!(
                    "Selected model '{}' not found in available models, clearing selection",
                    settings.selected_model
                );
                settings.selected_model = String::new();
                write_settings(&self.app_handle, settings.clone());
            }
        }

        // If no model is selected, pick the first downloaded one
        if settings.selected_model.is_empty() {
            // Find the first available (downloaded) model
            let models = crate::lock_recovery::recover_lock(self.available_models.lock());
            if let Some(available_model) = models.values().find(|model| model.is_downloaded) {
                info!(
                    "Auto-selecting model: {} ({})",
                    available_model.id, available_model.name
                );

                // Update settings with the selected model
                let mut updated_settings = settings;
                updated_settings.selected_model = available_model.id.clone();
                write_settings(&self.app_handle, updated_settings);

                info!("Successfully auto-selected model: {}", available_model.id);
            }
        }

        Ok(())
    }

    /// Discover custom Whisper models (.bin files) in the models directory.
    /// Skips files that match predefined model filenames.
    pub fn delete_model(&self, model_id: &str) -> Result<()> {
        debug!("ModelManager: delete_model called for: {}", model_id);

        let model_info = {
            let models = crate::lock_recovery::recover_lock(self.available_models.lock());
            models.get(model_id).cloned()
        };

        let model_info =
            model_info.ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        debug!("ModelManager: Found model info: {:?}", model_info);

        let dir = self.resolve_dir(&model_info.category);
        let model_path = dir.join(&model_info.filename);
        let partial_path = dir.join(format!("{}.partial", &model_info.filename));
        debug!("ModelManager: Model path: {:?}", model_path);
        debug!("ModelManager: Partial path: {:?}", partial_path);

        let mut deleted_something = false;

        if model_info.is_directory {
            // Delete complete model directory if it exists
            if model_path.exists() && model_path.is_dir() {
                info!("Deleting model directory at: {:?}", model_path);
                fs::remove_dir_all(&model_path)?;
                info!("Model directory deleted successfully");
                deleted_something = true;
            }
        } else {
            // Delete complete model file if it exists
            if model_path.exists() {
                info!("Deleting model file at: {:?}", model_path);
                fs::remove_file(&model_path)?;
                info!("Model file deleted successfully");
                deleted_something = true;
            }
        }

        // Delete partial file if it exists (same for both types)
        if partial_path.exists() {
            info!("Deleting partial file at: {:?}", partial_path);
            fs::remove_file(&partial_path)?;
            info!("Partial file deleted successfully");
            deleted_something = true;
        }

        if !deleted_something {
            return Err(anyhow::anyhow!("No model files found to delete"));
        }

        // Custom models should be removed from the list entirely since they
        // have no download URL and can't be re-downloaded
        if model_info.is_custom {
            let mut models = crate::lock_recovery::recover_lock(self.available_models.lock());
            models.remove(model_id);
            debug!("ModelManager: removed custom model from available models");
        } else {
            // Update download status (marks predefined models as not downloaded)
            self.update_download_status()?;
            debug!("ModelManager: download status updated");
        }

        // Emit event to notify UI
        let _ = self.app_handle.emit("model-deleted", model_id);

        Ok(())
    }

    pub fn get_model_path(&self, model_id: &str) -> Result<PathBuf> {
        let model_info = self
            .get_model_info(model_id)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        if !model_info.is_downloaded {
            return Err(anyhow::anyhow!("Model not available: {}", model_id));
        }

        // Ensure we don't return partial files/directories
        if model_info.is_downloading {
            return Err(anyhow::anyhow!(
                "Model is currently downloading: {}",
                model_id
            ));
        }

        let dir = self.resolve_dir(&model_info.category);
        let model_path = dir.join(&model_info.filename);
        let partial_path = dir.join(format!("{}.partial", &model_info.filename));

        if model_info.is_directory {
            // For directory-based models, ensure the directory exists and is complete
            if model_path.exists() && model_path.is_dir() && !partial_path.exists() {
                Ok(model_path)
            } else {
                Err(anyhow::anyhow!(
                    "Complete model directory not found: {}",
                    model_id
                ))
            }
        } else {
            // For file-based models (existing logic)
            if model_path.exists() && !partial_path.exists() {
                Ok(model_path)
            } else {
                Err(anyhow::anyhow!(
                    "Complete model file not found: {}",
                    model_id
                ))
            }
        }
    }

    pub fn cancel_download(&self, model_id: &str) -> Result<()> {
        debug!("ModelManager: cancel_download called for: {}", model_id);

        // Set the cancellation flag to stop the download loop
        {
            let flags = crate::lock_recovery::recover_lock(self.cancel_flags.lock());
            if let Some(flag) = flags.get(model_id) {
                flag.store(true, Ordering::Relaxed);
                info!("Cancellation flag set for: {}", model_id);
            } else {
                warn!("No active download found for: {}", model_id);
            }
        }

        // Update state immediately for UI responsiveness
        {
            let mut models = crate::lock_recovery::recover_lock(self.available_models.lock());
            if let Some(model) = models.get_mut(model_id) {
                model.is_downloading = false;
            }
        }

        // Update download status to reflect current state
        self.update_download_status()?;

        // Emit cancellation event so all UI components can clear their state
        let _ = self.app_handle.emit("model-download-cancelled", model_id);

        info!("Download cancellation initiated for: {}", model_id);
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
