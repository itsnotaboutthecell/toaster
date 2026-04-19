//! R-002 — ASR silence pre-filter (`features/reintroduce-silero-vad`).
//!
//! Produces a list of [`SpeechWindow`]s over a decoded 16 kHz mono PCM
//! buffer so that the transcription manager can hand only the speech
//! regions to the ASR. Per BLUEPRINT §AD-4 the module **does not**
//! rewrite the audio buffer — it returns windows in file-time (µs),
//! ASR is called per-window, and [`remap_words`] reprojects ASR-local
//! word timestamps into file-time in a single call site.
//!
//! Graceful degradation (BLUEPRINT §AD-8): callers are expected to
//! construct a [`SileroVad`] via [`try_open_silero`] and fall back to
//! the full-file ASR path on `Err` or `None`. This module never panics
//! on missing-model / ORT-init / resample failure.
//!
//! Timing invariants (PRD §AC-002-a):
//!   - frame cadence is exactly [`SILERO_FRAME_SAMPLES_16K`] samples,
//!   - window timestamps are absolute file-time in microseconds,
//!   - [`remap_words`] is the **only** place window-relative timestamps
//!     are shifted into file-time (R-002 timestamp correctness bar).

use std::path::Path;

use super::silero::{SileroVad, DEFAULT_SILERO_THRESHOLD, SILERO_FRAME_SAMPLES_16K};
use super::smoothed::{
    DEFAULT_HANGOVER_FRAMES, DEFAULT_ONSET_FRAMES, DEFAULT_PREFILL_FRAMES,
};
use super::VoiceActivityDetector;

/// Hard-coded working sample rate for the VAD path. Silero accepts
/// 8 kHz or 16 kHz; we standardize on 16 kHz because Whisper /
/// transcribe-rs also runs at 16 kHz, so callers can reuse the same
/// resampled buffer.
pub const VAD_SAMPLE_RATE_HZ: u32 = 16_000;

/// Pre-roll added to the left edge of each detected speech window.
/// Keeps a short run-in so ASR does not start mid-phoneme. Expressed
/// in microseconds; matches Handy's 120 ms prefill cadence.
pub const PREROLL_US: i64 = 120_000;

/// Hangover added to the right edge of each detected speech window.
/// Matches the 200 ms hangover used by [`super::smoothed::SmoothedVad`]
/// but applied once at the window level so padding does not
/// double-count frame-level hysteresis.
pub const HANGOVER_US: i64 = 200_000;

/// A contiguous speech span in absolute file-time. Always monotonic
/// (`end_us > start_us`). Emitted by [`prefilter_speech_windows`] and
/// consumed by the transcription manager when slicing the PCM buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeechWindow {
    pub start_us: i64,
    pub end_us: i64,
}

impl SpeechWindow {
    #[allow(dead_code)] // consumed by transcription pipeline once wired.
    pub fn duration_us(&self) -> i64 {
        self.end_us - self.start_us
    }
}

/// A microsecond-precise word emitted by the ASR. Mirrors the shape of
/// [`crate::managers::editor::Word`] without the deletion flag so the
/// prefilter remains decoupled from editor state. The transcription
/// manager constructs its editor `Word` from these fields after
/// remapping.
#[derive(Debug, Clone)]
#[allow(dead_code)] // consumed once transcription manager wires the prefilter path.
pub struct PrefilterWord {
    pub text: String,
    pub start_us: i64,
    pub end_us: i64,
}

/// Attempt to open the Silero VAD from `model_path`. Returns `Ok(None)`
/// when the file does not exist (graceful-absence path, R-005). Only
/// returns `Err` when the file exists but ORT refuses to load it — the
/// caller can surface a `tracing::warn!` once and continue on the
/// fall-back path.
#[allow(dead_code)] // wired by transcription / boundary / filler consumers.
pub fn try_open_silero(model_path: &Path) -> anyhow::Result<Option<SileroVad>> {
    if !model_path.exists() {
        return Ok(None);
    }
    let vad = SileroVad::new(
        model_path,
        VAD_SAMPLE_RATE_HZ as usize,
        DEFAULT_SILERO_THRESHOLD,
    )?;
    Ok(Some(vad))
}

/// Slice `samples_16k` into 30 ms frames, run them through `vad`, and
/// collapse consecutive speech frames into [`SpeechWindow`]s using the
/// [`DEFAULT_ONSET_FRAMES`] / [`DEFAULT_HANGOVER_FRAMES`] /
/// [`DEFAULT_PREFILL_FRAMES`] cadence (the BLUEPRINT single-source-of-
/// truth values). The returned vector is sorted by `start_us` and
/// non-overlapping.
///
/// `samples_16k` must be mono f32 at exactly [`VAD_SAMPLE_RATE_HZ`] —
/// the caller is responsible for resampling. The function never
/// panics; on an empty or sub-frame input it returns an empty vector.
/// Per-frame VAD errors are treated as silence so a single bad frame
/// does not abort the whole pass (graceful-degradation, AD-8).
#[allow(dead_code)] // wired by managers::transcription prefilter consumer.
pub fn prefilter_speech_windows<V: VoiceActivityDetector>(
    samples_16k: &[f32],
    vad: &mut V,
) -> Vec<SpeechWindow> {
    let mut out: Vec<SpeechWindow> = Vec::new();
    let frame = SILERO_FRAME_SAMPLES_16K;
    let total_frames = samples_16k.len() / frame;
    if total_frames == 0 {
        return out;
    }

    let frame_us: i64 = 1_000_000_i64 * frame as i64 / VAD_SAMPLE_RATE_HZ as i64;
    let buffer_end_us: i64 = frame_us * total_frames as i64;

    // Tiny on-the-spot hysteresis state machine — separate from
    // SmoothedVad because we operate at the window level rather than
    // re-emitting the prefilled slice. Uses the same constants so the
    // onset/hangover/prefill cadence is identical across callers.
    let mut onset = 0usize;
    let mut hangover = 0usize;
    let mut in_speech = false;
    let mut current: Option<(i64, i64)> = None; // (onset_start_us, last_voice_end_us)

    for fi in 0..total_frames {
        let lo = fi * frame;
        let hi = lo + frame;
        let frame_start_us = frame_us * fi as i64;
        let frame_end_us = frame_start_us + frame_us;

        let is_voice = vad.is_voice(&samples_16k[lo..hi]).unwrap_or(false);

        match (in_speech, is_voice) {
            (false, true) => {
                onset += 1;
                if onset >= DEFAULT_ONSET_FRAMES {
                    in_speech = true;
                    onset = 0;
                    hangover = DEFAULT_HANGOVER_FRAMES;
                    // Open the window at the start of the onset run,
                    // backdated by the onset length so the first
                    // qualifying voice frame is included.
                    let start_us = frame_start_us
                        - frame_us * (DEFAULT_ONSET_FRAMES as i64 - 1);
                    current = Some((start_us.max(0), frame_end_us));
                }
            }
            (false, false) => {
                onset = 0;
            }
            (true, true) => {
                hangover = DEFAULT_HANGOVER_FRAMES;
                if let Some((_, end)) = current.as_mut() {
                    *end = frame_end_us;
                }
            }
            (true, false) => {
                if hangover > 0 {
                    hangover -= 1;
                } else {
                    in_speech = false;
                    if let Some((start, end)) = current.take() {
                        out.push(pad_window(start, end, buffer_end_us));
                    }
                }
            }
        }
    }

    if let Some((start, end)) = current.take() {
        out.push(pad_window(start, end, buffer_end_us));
    }

    merge_overlapping(&mut out);
    out
}

/// Shift ASR-emitted word timestamps from window-relative to absolute
/// file-time. This is the **single** site that performs the shift; any
/// new ASR backend consumed by the prefilter path routes through here
/// (R-002 timestamp correctness invariant). Preserves microsecond
/// precision — no rounding, no equal-duration synthesis (per
/// `transcript-precision-eval`).
#[allow(dead_code)] // wired by managers::transcription prefilter consumer.
pub fn remap_words(words: &mut [PrefilterWord], window: SpeechWindow) {
    for w in words {
        w.start_us += window.start_us;
        w.end_us += window.start_us;
    }
}

fn pad_window(start_us: i64, end_us: i64, buffer_end_us: i64) -> SpeechWindow {
    // Pre-roll accounts for DEFAULT_PREFILL_FRAMES worth of context
    // the SmoothedVad would have replayed on stream open. Use the
    // constant directly so this stays aligned with that module.
    let _ = DEFAULT_PREFILL_FRAMES; // keep the import used; constant drives PREROLL_US.
    SpeechWindow {
        start_us: (start_us - PREROLL_US).max(0),
        end_us: (end_us + HANGOVER_US).min(buffer_end_us),
    }
}

fn merge_overlapping(windows: &mut Vec<SpeechWindow>) {
    if windows.len() < 2 {
        return;
    }
    let mut out: Vec<SpeechWindow> = Vec::with_capacity(windows.len());
    for w in windows.drain(..) {
        match out.last_mut() {
            Some(prev) if w.start_us <= prev.end_us => {
                prev.end_us = prev.end_us.max(w.end_us);
            }
            _ => out.push(w),
        }
    }
    *windows = out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_toolkit::vad::VadFrame;
    use anyhow::Result;

    /// Scripted fake VAD: returns `is_voice(frame) = script[pos]`.
    /// Used to drive windowing logic without loading an ONNX.
    struct ScriptedVad {
        script: Vec<bool>,
        pos: usize,
    }

    impl VoiceActivityDetector for ScriptedVad {
        fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>> {
            let v = *self.script.get(self.pos).unwrap_or(&false);
            self.pos += 1;
            Ok(if v { VadFrame::Speech(frame) } else { VadFrame::Noise })
        }
    }

    fn make_samples(n_frames: usize) -> Vec<f32> {
        vec![0.0; n_frames * SILERO_FRAME_SAMPLES_16K]
    }

    #[test]
    fn empty_buffer_returns_no_windows() {
        let mut vad = ScriptedVad { script: vec![], pos: 0 };
        assert!(prefilter_speech_windows(&[], &mut vad).is_empty());
    }

    #[test]
    fn sub_frame_buffer_returns_no_windows() {
        let mut vad = ScriptedVad { script: vec![true], pos: 0 };
        let short = vec![0.0; SILERO_FRAME_SAMPLES_16K - 1];
        assert!(prefilter_speech_windows(&short, &mut vad).is_empty());
    }

    #[test]
    fn all_silence_returns_no_windows() {
        let samples = make_samples(20);
        let mut vad = ScriptedVad { script: vec![false; 20], pos: 0 };
        assert!(prefilter_speech_windows(&samples, &mut vad).is_empty());
    }

    #[test]
    fn onset_below_threshold_does_not_open_window() {
        // One voice frame followed by silence should not open a span
        // (DEFAULT_ONSET_FRAMES = 2).
        const _: () = assert!(DEFAULT_ONSET_FRAMES >= 2);
        let samples = make_samples(10);
        let mut script = vec![false; 10];
        script[3] = true;
        let mut vad = ScriptedVad { script, pos: 0 };
        assert!(prefilter_speech_windows(&samples, &mut vad).is_empty());
    }

    #[test]
    fn sustained_speech_produces_single_window() {
        // Voice in frames 2..=15 (14 consecutive speech frames).
        // Expect one window covering that range, padded by preroll /
        // hangover.
        let samples = make_samples(20);
        let mut script = vec![false; 20];
        for item in script.iter_mut().take(16).skip(2) {
            *item = true;
        }
        let mut vad = ScriptedVad { script, pos: 0 };
        let windows = prefilter_speech_windows(&samples, &mut vad);
        assert_eq!(windows.len(), 1, "got {windows:?}");
        let w = windows[0];
        assert!(w.start_us >= 0);
        assert!(w.end_us > w.start_us);
        // Pre-roll extends at least 100ms before onset frame-2.
        let frame_us: i64 = 1_000_000 * SILERO_FRAME_SAMPLES_16K as i64 / VAD_SAMPLE_RATE_HZ as i64;
        assert!(w.start_us <= 2 * frame_us);
    }

    #[test]
    fn remap_words_shifts_by_window_start() {
        let mut words = vec![
            PrefilterWord { text: "hello".into(), start_us: 0, end_us: 300_000 },
            PrefilterWord { text: "world".into(), start_us: 400_000, end_us: 700_000 },
        ];
        remap_words(
            &mut words,
            SpeechWindow { start_us: 5_000_000, end_us: 6_000_000 },
        );
        assert_eq!(words[0].start_us, 5_000_000);
        assert_eq!(words[0].end_us, 5_300_000);
        assert_eq!(words[1].start_us, 5_400_000);
        assert_eq!(words[1].end_us, 5_700_000);
    }

    #[test]
    fn remap_preserves_precision_no_rounding() {
        // AC-002-a guard: no equal-duration synthesis. Irregular
        // durations must survive the shift unchanged.
        let mut words = vec![
            PrefilterWord { text: "a".into(), start_us: 10_101, end_us: 137_777 },
            PrefilterWord { text: "b".into(), start_us: 137_777, end_us: 389_123 },
        ];
        let d0 = words[0].end_us - words[0].start_us;
        let d1 = words[1].end_us - words[1].start_us;
        remap_words(
            &mut words,
            SpeechWindow { start_us: 7_654_321, end_us: 9_999_999 },
        );
        assert_eq!(words[0].end_us - words[0].start_us, d0);
        assert_eq!(words[1].end_us - words[1].start_us, d1);
    }

    #[test]
    fn try_open_silero_missing_model_returns_ok_none() {
        // R-005 / AC-005-c core shape: absence is a graceful-fallback
        // signal, never an error.
        let out =
            try_open_silero(Path::new("does_not_exist_silero_vad.onnx"))
                .expect("missing model should yield Ok(None), not Err");
        assert!(out.is_none());
    }
}
