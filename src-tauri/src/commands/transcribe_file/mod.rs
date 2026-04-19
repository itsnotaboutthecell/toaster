use log::info;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

use crate::commands::editor::EditorStore;
use crate::managers::editor::Word;
use crate::managers::transcription::TranscriptionManager;

mod extract;
use extract::{extract_audio_to_wav_at_rate, is_wav_file};

pub(super) const SAMPLE_RATE_HZ: f64 = 16000.0;
/// Maximum transcription duration in seconds (4 hours).
/// At 16kHz mono float32 this is ~921 MB of WAV sample data.
const MAX_TRANSCRIPTION_DURATION_SECS: u64 = 14400;
/// Bytes per sample for 16kHz mono PCM (4 bytes for f32 / pcm_s16le raw estimate).
const BYTES_PER_SAMPLE: u64 = 4;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct WordAlignmentMeta {
    pub(super) interpolated: bool,
}

mod alignment;
mod word_builder;
use alignment::realign_suspicious_spans;
use word_builder::{build_words_from_segments, sanitize_word_timestamps};

/// Transcribe any audio or video file and populate the editor with word-level results.
///
/// For WAV files, reads samples directly. For all other formats (MP4, MP3, etc.),
/// uses FFmpeg to extract audio to a temporary 16kHz mono WAV first.
#[tauri::command]
#[specta::specta]
pub async fn transcribe_media_file(
    app: AppHandle,
    editor_store: State<'_, EditorStore>,
    path: String,
) -> Result<Vec<Word>, String> {
    let file_path = std::path::Path::new(&path);

    if !file_path.exists() {
        return Err(format!("File not found: {}", path));
    }

    // Look up the model's declared native input sample rate. Falls back to
    // the default (16 kHz) when the model isn't known or no manager exists;
    // matches the old hardcoded `-ar 16000`.
    let asr_sample_rate_hz: u32 = {
        let settings = crate::settings::get_settings(&app);
        app.try_state::<Arc<crate::managers::model::ModelManager>>()
            .and_then(|mm| mm.get_model_info(&settings.selected_model))
            .map(|info| info.input_sample_rate_hz())
            .unwrap_or(crate::audio_toolkit::constants::ASR_INPUT_SAMPLE_RATE_HZ_DEFAULT)
    };

    // For non-WAV files, extract audio via FFmpeg first
    let (wav_path, is_temp) = if is_wav_file(file_path) {
        (file_path.to_path_buf(), false)
    } else {
        (
            extract_audio_to_wav_at_rate(file_path, asr_sample_rate_hz)?,
            true,
        )
    };

    // Guard: check file size / estimated duration before loading into memory
    let wav_file_size = std::fs::metadata(&wav_path).map(|m| m.len()).unwrap_or(0);
    let estimated_duration_secs = wav_file_size / (16000 * BYTES_PER_SAMPLE);
    if estimated_duration_secs > MAX_TRANSCRIPTION_DURATION_SECS {
        if is_temp {
            let _ = std::fs::remove_file(&wav_path);
        }
        let est_hours = estimated_duration_secs as f64 / 3600.0;
        let max_hours = MAX_TRANSCRIPTION_DURATION_SECS as f64 / 3600.0;
        return Err(format!(
            "Audio too long for transcription ({:.1} hours). Maximum is {:.0} hours.",
            est_hours, max_hours
        ));
    }

    // Read audio samples from WAV file
    let samples = crate::audio_toolkit::read_wav_samples(&wav_path).map_err(|e| {
        if is_temp {
            let _ = std::fs::remove_file(&wav_path);
        }
        format!("Failed to read audio: {}", e)
    })?;

    // Clean up temp file
    if is_temp {
        let _ = std::fs::remove_file(&wav_path);
    }

    if samples.is_empty() {
        return Err("Audio file contains no samples".to_string());
    }

    // Get the transcription manager
    let tm = app
        .try_state::<Arc<TranscriptionManager>>()
        .ok_or_else(|| "Transcription manager not available".to_string())?;

    info!("Transcribing {} samples...", samples.len());

    // Ensure model is loaded before transcribing
    if !tm.is_model_loaded() {
        info!("Model not loaded — initiating auto-load...");
        tm.initiate_model_load();
        // Wait for model to finish loading (initiate_model_load is async)
        // The transcribe() call below will wait on the loading condvar
    }

    // Transcribe — now returns a NormalizedTranscriptionResult with the
    // engine's raw segment timings preserved alongside the filtered text.
    let normalized = tm
        .transcribe(samples.clone())
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not loaded") {
                "No transcription model loaded. Go to Settings → Models, download a model, then try again.".to_string()
            } else {
                format!("Transcription failed: {}", msg)
            }
        })?;
    let text = normalized.text;
    let segments = normalized.segments;

    if text.is_empty() {
        return Err("Transcription produced no text".to_string());
    }

    // Build words with real timestamps from transcription segments.
    // Primary: DP forced alignment inside each engine-reported segment
    // (see `audio_toolkit::forced_alignment`). Interior boundaries are
    // placed at the frames that minimize local RMS energy plus a quadratic
    // deviation penalty from their char-proportional expected position,
    // replacing the legacy char-proportional synthesis as the primary source
    // of per-word timing for engines whose adapter reports
    // `word_timestamps_authoritative = false` (todo
    // `p1-authoritative-flag-actionable`).
    //
    // The adapter layer (`managers::transcription::adapter`) is the single
    // contract enforcement point: every engine MUST produce at least
    // segment-level timestamps. If an engine genuinely can't, the fix is
    // forced alignment *in the adapter*, not equal-duration synthesis
    // downstream. See todos `p1-adapter-trait` and
    // `p3-abandon-even-dist-fallback`, and the "equal-duration timestamp
    // synthesis" prohibition in AGENTS.md.
    let sample_rate = 16000.0_f64;
    let total_duration_us = crate::audio_toolkit::timing::sample_to_us(samples.len(), sample_rate);

    let segs = segments.as_ref().ok_or_else(|| {
        "Transcription engine produced no segment-level timestamps; adapter must always \
         return at least segment-level timing. See p1-adapter-trait."
            .to_string()
    })?;
    if segs.is_empty() {
        return Err(
            "Transcription engine returned an empty segment list; adapter must always \
             return at least segment-level timing. See p1-adapter-trait."
                .to_string(),
        );
    }

    let (mut words, align_meta_vec) = build_words_from_segments(&text, segs, &samples);
    let align_meta: Option<Vec<WordAlignmentMeta>> = Some(align_meta_vec);

    // Sanitize timestamps: clamp to audio duration, enforce monotonic
    // non-overlapping progression, and ensure minimal non-zero durations.
    sanitize_word_timestamps(&mut words, total_duration_us);
    realign_suspicious_spans(&mut words, &samples, align_meta.as_deref());
    sanitize_word_timestamps(&mut words, total_duration_us);

    if words.is_empty() {
        return Err("No words in transcription".to_string());
    }

    // Populate the editor
    let mut state = crate::lock_recovery::try_lock(editor_store.0.lock()).map_err(|e| e.to_string())?;
    state.set_words(words.clone());

    Ok(state.get_words().to_vec())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// Precision benchmark suite for the Toaster edit pipeline.
///
/// These tests assert explicit acceptance thresholds for boundary quality and
/// pipeline correctness. All tests are deterministic — no timing, no I/O, no
/// external dependencies.
///
/// Acceptance thresholds (in comments near each group):
///   - Monotonicity violations:  0 (hard invariant)
///   - Boundary drift budget:   ≤ 162 000 µs (search window 160 ms + ZC snap 2 ms)
///   - Sample↔µs roundtrip:    ≤ 1 sample error (≈62.5 µs at 16 kHz)
///   - Edit→source time drift:  0 µs (integer arithmetic, exact)
///
/// TODO[click-rate]: True per-seam click rate cannot be asserted in unit tests
/// without perceptual audio analysis. Surrogate checks to add once the export
/// pipeline is end-to-end testable:
///   1. `samples[result] * samples[result+1] <= 0.0` at every exported cut point.
///   2. RMS in a 2 ms window around each seam < 10 % of signal peak RMS.
///   3. Adjacent keep-segments share exactly one boundary sample (no gap/overlap).
///
/// Add a `seam_rms_at_boundary(samples, cut_sample, window_samples) -> f32`
/// helper and assert `seam_rms < 0.05 * peak_rms` for each cut.

#[cfg(test)]
#[path = "precision_benchmarks.rs"]
mod precision_benchmarks;
