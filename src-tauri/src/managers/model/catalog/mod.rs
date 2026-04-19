//! Unified model catalog — transcription entries only (R9 purge).
//!
//! Post-processor LLM catalog was removed in Round 9 alongside the LLM
//! runtime (`managers::llm`, `managers::cleanup`, and the
//! post-processing settings surface). The `[transcription]` submodule
//! remains the single source of curated entries.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::Result;
use log::{info, warn};

use super::{hash, EngineType, ModelInfo};

pub mod transcription;
pub mod vad;

/// Build the full catalog keyed by model id.
/// Called once at `ModelManager::new`.
pub(super) fn build_static_catalog() -> HashMap<String, ModelInfo> {
    let mut out: HashMap<String, ModelInfo> = HashMap::new();
    for entry in transcription::entries() {
        out.insert(entry.id.clone(), entry);
    }
    out
}

/// Flat view of every curated catalog entry.
#[allow(dead_code)]
pub fn all() -> Vec<ModelInfo> {
    transcription::entries()
}

pub(super) fn discover_custom_whisper_models(
    models_dir: &Path,
    available_models: &mut HashMap<String, ModelInfo>,
) -> Result<()> {
    if !models_dir.exists() {
        return Ok(());
    }

    let predefined_filenames: HashSet<String> = available_models
        .values()
        .filter(|m| matches!(m.engine_type, EngineType::Whisper) && !m.is_directory)
        .map(|m| m.filename.clone())
        .collect();

    for entry in fs::read_dir(models_dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|s| s.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if filename.starts_with('.') {
            continue;
        }

        if !filename.ends_with(".bin") {
            continue;
        }

        if predefined_filenames.contains(&filename) {
            continue;
        }

        let model_id = filename.trim_end_matches(".bin").to_string();

        if available_models.contains_key(&model_id) {
            continue;
        }

        let display_name = model_id
            .replace(['-', '_'], " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        let size_mb = match path.metadata() {
            Ok(meta) => meta.len() / (1024 * 1024),
            Err(e) => {
                warn!("Failed to get metadata for {}: {}", filename, e);
                0
            }
        };

        info!(
            "Discovered custom Whisper model: {} ({}, {} MB)",
            model_id, filename, size_mb
        );

        available_models.insert(
            model_id.clone(),
            ModelInfo {
                id: model_id,
                name: display_name,
                description: "Not officially supported".to_string(),
                filename,
                url: None,
                sha256: None,
                size_mb,
                is_downloaded: true,
                is_downloading: false,
                partial_size: 0,
                is_directory: false,
                engine_type: EngineType::Whisper,
                accuracy_score: 0.0,
                speed_score: 0.0,
                supports_translation: false,
                is_recommended: false,
                supported_languages: vec![],
                supports_language_selection: true,
                is_custom: true,
                category: super::ModelCategory::Transcription,
                transcription_metadata: None,
            },
        );
    }

    Ok(())
}

/// Verifies the SHA256 of `path` against `expected_sha256` (if provided).
pub(super) fn verify_sha256(
    path: &Path,
    expected_sha256: Option<&str>,
    model_id: &str,
) -> Result<()> {
    hash::verify_sha256(path, expected_sha256, model_id)
}
