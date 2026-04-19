//! ModelManager::download_model implementation (extracted from mod.rs).

use anyhow::Result;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use log::{info, warn};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tar::Archive;
use tauri::Emitter;

use super::{DownloadCleanup, DownloadProgress, ModelManager};

impl ModelManager {
    pub async fn download_model(&self, model_id: &str) -> Result<()> {
        let model_info = {
            let models = crate::lock_recovery::recover_lock(self.available_models.lock());
            models.get(model_id).cloned()
        };

        let model_info =
            model_info.ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        let url = model_info
            .url
            .ok_or_else(|| anyhow::anyhow!("No download URL for model"))?;
        let model_path = self.models_dir.join(&model_info.filename);
        let partial_path = self
            .models_dir
            .join(format!("{}.partial", &model_info.filename));

        // Don't download if complete version already exists
        if model_path.exists() {
            // Clean up any partial file that might exist
            if partial_path.exists() {
                let _ = fs::remove_file(&partial_path);
            }
            self.update_download_status()?;
            return Ok(());
        }

        // Check if we have a partial download to resume
        let mut resume_from = if partial_path.exists() {
            let size = partial_path.metadata()?.len();
            info!("Resuming download of model {} from byte {}", model_id, size);
            size
        } else {
            info!("Starting fresh download of model {} from {}", model_id, url);
            0
        };

        // Mark as downloading
        {
            let mut models = crate::lock_recovery::recover_lock(self.available_models.lock());
            if let Some(model) = models.get_mut(model_id) {
                model.is_downloading = true;
            }
        }

        // Create cancellation flag for this download
        let cancel_flag = Arc::new(AtomicBool::new(false));
        {
            let mut flags = crate::lock_recovery::recover_lock(self.cancel_flags.lock());
            flags.insert(model_id.to_string(), cancel_flag.clone());
        }

        // Guard ensures is_downloading and cancel_flags are cleaned up on every
        // error path. Disarmed only on success (which sets is_downloaded = true).
        let mut cleanup = DownloadCleanup {
            available_models: &self.available_models,
            cancel_flags: &self.cancel_flags,
            model_id: model_id.to_string(),
            disarmed: false,
        };

        // Create HTTP client with range request for resuming
        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if resume_from > 0 {
            request = request.header("Range", format!("bytes={}-", resume_from));
        }

        let mut response = request.send().await?;

        // If we tried to resume but server returned 200 (not 206 Partial Content),
        // the server doesn't support range requests. Delete partial file and restart
        // fresh to avoid file corruption (appending full file to partial).
        if resume_from > 0 && response.status() == reqwest::StatusCode::OK {
            warn!(
                "Server doesn't support range requests for model {}, restarting download",
                model_id
            );
            drop(response);
            let _ = fs::remove_file(&partial_path);

            // Reset resume_from since we're starting fresh
            resume_from = 0;

            // Restart download without range header
            response = client.get(&url).send().await?;
        }

        // Check for success or partial content status
        if !response.status().is_success()
            && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
        {
            return Err(anyhow::anyhow!(
                "Failed to download model: HTTP {}",
                response.status()
            ));
        }

        let total_size = if resume_from > 0 {
            // For resumed downloads, add the resume point to content length
            resume_from + response.content_length().unwrap_or(0)
        } else {
            response.content_length().unwrap_or(0)
        };

        let mut downloaded = resume_from;
        let mut stream = response.bytes_stream();

        // Open file for appending if resuming, or create new if starting fresh
        let mut file = if resume_from > 0 {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&partial_path)?
        } else {
            std::fs::File::create(&partial_path)?
        };

        // Emit initial progress
        let initial_progress = DownloadProgress {
            model_id: model_id.to_string(),
            downloaded,
            total: total_size,
            percentage: if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                0.0
            },
        };
        let _ = self
            .app_handle
            .emit("model-download-progress", &initial_progress);

        // Throttle progress events to max 10/sec (100ms intervals)
        let mut last_emit = Instant::now();
        let throttle_duration = Duration::from_millis(100);

        // Download with progress
        while let Some(chunk) = stream.next().await {
            // Check if download was cancelled
            if cancel_flag.load(Ordering::Relaxed) {
                drop(file);
                info!("Download cancelled for: {}", model_id);
                // Keep partial file for resume functionality.
                // Guard handles is_downloading + cancel_flags cleanup on drop.
                return Ok(());
            }

            let chunk = chunk?;

            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;

            let percentage = if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };

            // Emit progress event (throttled to avoid UI freeze)
            if last_emit.elapsed() >= throttle_duration {
                let progress = DownloadProgress {
                    model_id: model_id.to_string(),
                    downloaded,
                    total: total_size,
                    percentage,
                };
                let _ = self.app_handle.emit("model-download-progress", &progress);
                last_emit = Instant::now();
            }
        }

        // Emit final progress to ensure 100% is shown
        let final_progress = DownloadProgress {
            model_id: model_id.to_string(),
            downloaded,
            total: total_size,
            percentage: if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                100.0
            },
        };
        let _ = self
            .app_handle
            .emit("model-download-progress", &final_progress);

        file.flush()?;
        drop(file); // Ensure file is closed before moving

        // Verify downloaded file size matches expected size
        if total_size > 0 {
            let actual_size = partial_path.metadata()?.len();
            if actual_size != total_size {
                // Download is incomplete/corrupted - delete partial and return error
                let _ = fs::remove_file(&partial_path);
                return Err(anyhow::anyhow!(
                    "Download incomplete: expected {} bytes, got {} bytes",
                    total_size,
                    actual_size
                ));
            }
        }

        // Verify SHA256 checksum. Runs in a blocking thread so the async executor is not
        // stalled while hashing large model files (up to 1.6 GB). On failure the partial
        // is deleted inside verify_sha256 so the next attempt always starts fresh.
        let _ = self.app_handle.emit("model-verification-started", model_id);
        info!("Verifying SHA256 for model {}...", model_id);
        let verify_path = partial_path.clone();
        let verify_expected = model_info.sha256.clone();
        let verify_model_id = model_id.to_string();
        let verify_result = tokio::task::spawn_blocking(move || {
            super::catalog::verify_sha256(
                &verify_path,
                verify_expected.as_deref(),
                &verify_model_id,
            )
        })
        .await
        .map_err(|e| anyhow::anyhow!("SHA256 task panicked: {}", e))?;
        verify_result?;
        let _ = self
            .app_handle
            .emit("model-verification-completed", model_id);

        // Handle directory-based models (extract tar.gz) vs file-based models
        if model_info.is_directory {
            // Track that this model is being extracted
            {
                let mut extracting = crate::lock_recovery::recover_lock(self.extracting_models.lock());
                extracting.insert(model_id.to_string());
            }

            // Emit extraction started event
            let _ = self.app_handle.emit("model-extraction-started", model_id);
            info!("Extracting archive for directory-based model: {}", model_id);

            // Use a temporary extraction directory to ensure atomic operations
            let temp_extract_dir = self
                .models_dir
                .join(format!("{}.extracting", &model_info.filename));
            let final_model_dir = self.models_dir.join(&model_info.filename);

            // Clean up any previous incomplete extraction
            if temp_extract_dir.exists() {
                let _ = fs::remove_dir_all(&temp_extract_dir);
            }

            // Create temporary extraction directory
            fs::create_dir_all(&temp_extract_dir)?;

            // Open the downloaded tar.gz file
            let tar_gz = File::open(&partial_path)?;
            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);

            // Extract to the temporary directory first
            archive.unpack(&temp_extract_dir).map_err(|e| {
                let error_msg = format!("Failed to extract archive: {}", e);
                // Clean up failed extraction
                let _ = fs::remove_dir_all(&temp_extract_dir);
                // Delete the corrupt partial file so the next download attempt starts fresh
                // instead of resuming from a broken archive (issue #858).
                let _ = fs::remove_file(&partial_path);
                // Remove from extracting set
                {
                    let mut extracting = crate::lock_recovery::recover_lock(self.extracting_models.lock());
                    extracting.remove(model_id);
                }
                let _ = self.app_handle.emit(
                    "model-extraction-failed",
                    &serde_json::json!({
                        "model_id": model_id,
                        "error": error_msg
                    }),
                );
                anyhow::anyhow!(error_msg)
            })?;

            // Find the actual extracted directory (archive might have a nested structure)
            let extracted_dirs: Vec<_> = fs::read_dir(&temp_extract_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                .collect();

            if extracted_dirs.len() == 1 {
                // Single directory extracted, move it to the final location
                let source_dir = extracted_dirs[0].path();
                if final_model_dir.exists() {
                    fs::remove_dir_all(&final_model_dir)?;
                }
                fs::rename(&source_dir, &final_model_dir)?;
                // Clean up temp directory
                let _ = fs::remove_dir_all(&temp_extract_dir);
            } else {
                // Multiple items or no directories, rename the temp directory itself
                if final_model_dir.exists() {
                    fs::remove_dir_all(&final_model_dir)?;
                }
                fs::rename(&temp_extract_dir, &final_model_dir)?;
            }

            info!("Successfully extracted archive for model: {}", model_id);
            // Remove from extracting set
            {
                let mut extracting = crate::lock_recovery::recover_lock(self.extracting_models.lock());
                extracting.remove(model_id);
            }
            // Emit extraction completed event
            let _ = self.app_handle.emit("model-extraction-completed", model_id);

            // Remove the downloaded tar.gz file
            let _ = fs::remove_file(&partial_path);
        } else {
            // Move partial file to final location for file-based models
            fs::rename(&partial_path, &model_path)?;
        }

        // Disarm the guard — success path does its own cleanup because it
        // additionally sets is_downloaded = true.
        cleanup.disarmed = true;
        {
            let mut models = crate::lock_recovery::recover_lock(self.available_models.lock());
            if let Some(model) = models.get_mut(model_id) {
                model.is_downloading = false;
                model.is_downloaded = true;
                model.partial_size = 0;
            }
        }
        crate::lock_recovery::recover_lock(self.cancel_flags.lock()).remove(model_id);

        // Emit completion event
        let _ = self.app_handle.emit("model-download-complete", model_id);

        info!(
            "Successfully downloaded model {} to {:?}",
            model_id, model_path
        );

        Ok(())
    }
}
