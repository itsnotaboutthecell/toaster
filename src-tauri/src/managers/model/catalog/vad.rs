//! Silero Voice Activity Detector — model catalog entry (R-005).
//!
//! The Silero ONNX is **not** a transcription engine; it is a
//! file-based analyzer consumed by
//! [`managers::transcription::prefilter`] (R-002),
//! [`managers::splice::boundaries`] (R-003), and
//! [`managers::filler`] (R-004). Phase 4 wires it through the same
//! `ModelManager` download / SHA-256 / cancel / delete pipeline used
//! for ASR models, differentiated by
//! [`ModelCategory::VoiceActivityDetection`] so the ASR model picker
//! filters it out.
//!
//! All three consumers must call [`silero_vad_model_path`] to discover
//! the ONNX location, and must treat absence as a graceful-fallback
//! signal per BLUEPRINT AD-8.

use std::path::{Path, PathBuf};

use super::super::{EngineType, ModelCategory, ModelInfo};

/// On-disk filename for the Silero VAD ONNX. Kept short and
/// versionless so future model swaps are a one-line catalog change.
pub const SILERO_VAD_FILENAME: &str = "silero_vad.onnx";

/// Human-readable identifier used by the download surface. Exposed
/// here (rather than inlined at the call site) so tests can grep for
/// it under `managers/model/catalog` without depending on UI strings.
pub const SILERO_VAD_MODEL_ID: &str = "silero-vad";

/// Approximate on-disk size. Used only for UI progress hints; the
/// downloader verifies by SHA-256, not size.
pub const SILERO_VAD_APPROX_SIZE_BYTES: u64 = 1_807_522;

/// Upstream download URL for the pinned Silero v4 ONNX.
pub const SILERO_VAD_URL: &str =
    "https://github.com/snakers4/silero-vad/raw/v4.0/files/silero_vad.onnx";

/// Expected SHA-256 of [`SILERO_VAD_URL`]'s payload. Pinned to the
/// snakers4/silero-vad v4.0 release tag (1,807,522 bytes); verified
/// 2026-04-19 via `Invoke-WebRequest` + `Get-FileHash -Algorithm SHA256`.
pub const SILERO_VAD_SHA256: &str =
    "a35ebf52fd3ce5f1469b2a36158dba761bc47b973ea3382b3186ca15b1f5af28";

/// Resolve the on-disk location of the Silero VAD ONNX inside
/// `models_dir`. The file may or may not exist — callers must
/// `Path::exists()` before handing to `SileroVad::new`. This is the
/// single source of truth for the filesystem path; do not duplicate
/// the `models_dir.join(SILERO_VAD_FILENAME)` expression elsewhere.
pub fn silero_vad_model_path(models_dir: &Path) -> PathBuf {
    models_dir.join(SILERO_VAD_FILENAME)
}

/// ModelInfo entries exposed to the catalog aggregator.
/// Currently a single Silero entry; future VAD backends would append here.
pub(super) fn entries() -> Vec<ModelInfo> {
    vec![ModelInfo {
        id: SILERO_VAD_MODEL_ID.to_string(),
        name: "Silero VAD".to_string(),
        description: "Voice activity detector used to pre-filter silence before transcription \
                      and to refine splice boundaries. ~1.8 MB, runs locally."
            .to_string(),
        filename: SILERO_VAD_FILENAME.to_string(),
        url: Some(SILERO_VAD_URL.to_string()),
        sha256: Some(SILERO_VAD_SHA256.to_string()),
        // `size_mb` is UI-display only. Silero is < 2 MB; round up so the
        // download card reads "2 MB" rather than "0 MB" (integer division).
        size_mb: SILERO_VAD_APPROX_SIZE_BYTES.div_ceil(1024 * 1024),
        is_downloaded: false,
        is_downloading: false,
        partial_size: 0,
        is_directory: false,
        engine_type: EngineType::default(),
        accuracy_score: 0.0,
        speed_score: 0.0,
        supports_translation: false,
        is_recommended: false,
        supported_languages: vec![],
        supports_language_selection: false,
        is_custom: false,
        category: ModelCategory::VoiceActivityDetection,
        transcription_metadata: None,
    }]
}
