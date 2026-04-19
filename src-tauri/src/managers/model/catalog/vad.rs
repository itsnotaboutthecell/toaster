//! Silero Voice Activity Detector — model catalog entry (R-005).
//!
//! The Silero ONNX is **not** a transcription engine; it is a
//! file-based analyzer consumed by
//! [`managers::transcription::prefilter`] (R-002),
//! [`managers::splice::boundaries`] (R-003), and
//! [`managers::filler`] (R-004). It therefore does not go through the
//! `ModelInfo` / `EngineType::*` catalog surface that users see for
//! picking an ASR model — it has its own tiny metadata block here and
//! reuses the existing download / SHA-256 verification primitives in
//! the parent `managers::model` module.
//!
//! All three consumers must call [`silero_vad_model_path`] to discover
//! the ONNX location, and must treat absence as a graceful-fallback
//! signal per BLUEPRINT AD-8.
//!
//! The URL and SHA-256 placeholders below are deliberate and gate
//! STATE.md promotion to `implemented`. Populate them with the pinned
//! snakers4/silero-vad release that ships the v4 ONNX before merging.

use std::path::{Path, PathBuf};

/// On-disk filename for the Silero VAD ONNX. Kept short and
/// versionless so future model swaps are a one-line catalog change.
pub const SILERO_VAD_FILENAME: &str = "silero_vad.onnx";

/// Human-readable identifier used by the download surface. Exposed
/// here (rather than inlined at the call site) so tests can grep for
/// it under `managers/model/catalog` without depending on UI strings.
pub const SILERO_VAD_MODEL_ID: &str = "silero-vad";

/// Approximate on-disk size. Used only for UI progress hints; the
/// downloader verifies by SHA-256, not size.
pub const SILERO_VAD_APPROX_SIZE_BYTES: u64 = 2_200_000;

/// Upstream download URL for the pinned Silero v4 ONNX. `None` means
/// the catalog entry has not been populated yet — callers must treat
/// absence as "feature not downloadable in this build" and surface a
/// clear message rather than silently 404. Populated before STATE.md
/// advances to `implemented` per features/reintroduce-silero-vad.
#[allow(dead_code)] // wired by Phase 2 downloader plumbing.
pub const SILERO_VAD_URL: Option<&str> = None;

/// Expected SHA-256 of [`SILERO_VAD_URL`]'s payload. `None` until the
/// URL is pinned; the downloader refuses to activate prefilter /
/// boundary / filler features when the hash is missing so a
/// silently-bad model file cannot bypass verification.
#[allow(dead_code)] // wired by Phase 2 downloader plumbing.
pub const SILERO_VAD_SHA256: Option<&str> = None;

/// Resolve the on-disk location of the Silero VAD ONNX inside
/// `models_dir`. The file may or may not exist — callers must
/// `Path::exists()` before handing to `SileroVad::new`. This is the
/// single source of truth for the filesystem path; do not duplicate
/// the `models_dir.join(SILERO_VAD_FILENAME)` expression elsewhere.
#[allow(dead_code)] // wired by Phase 2 consumers (R-002 / R-003 / R-004).
pub fn silero_vad_model_path(models_dir: &Path) -> PathBuf {
    models_dir.join(SILERO_VAD_FILENAME)
}
