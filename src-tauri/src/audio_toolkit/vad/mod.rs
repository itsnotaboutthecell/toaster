//! File-based Voice Activity Detection for Toaster.
//!
//! Reintroduced after the Handy-era prune (see
//! `.github/skills/handy-legacy-pruning/SKILL.md` — "VAD reintroduced"
//! section) for three strictly file-based editor use cases specified
//! in `features/reintroduce-silero-vad/PRD.md`:
//!
//! * **R-002** — ASR silence pre-filter in
//!   `managers::transcription::prefilter`.
//! * **R-003** — P(speech)-aware splice-boundary refinement in
//!   `managers::splice::boundaries`.
//! * **R-004** — acoustic classification of long filler/pause gaps in
//!   `managers::filler`.
//!
//! The microphone path is **not** reintroduced — no push-to-talk, no
//! recorder, no overlay. Everything here operates on already-decoded
//! `f32` PCM from a file.
//!
//! Contract: callers push fixed-size frames via
//! [`VoiceActivityDetector::push_frame`]; the detector returns a
//! [`VadFrame`] that either carries the same slice (speech — possibly
//! aggregating prefill / hangover) or indicates non-speech. The
//! [`SmoothedVad`] wrapper layers pre-roll, onset debounce, and
//! hangover hysteresis around any raw detector so callers get
//! utterance-level spans rather than per-frame flicker.

use anyhow::Result;

pub enum VadFrame<'a> {
    /// Speech — may aggregate several frames (prefill + current + hangover).
    Speech(&'a [f32]),
    /// Non-speech (silence, noise). Down-stream code can ignore it.
    Noise,
}

impl<'a> VadFrame<'a> {
    #[inline]
    #[allow(dead_code)] // wired by R-002 / R-003 / R-004 consumers in Phase 2.
    pub fn is_speech(&self) -> bool {
        matches!(self, VadFrame::Speech(_))
    }
}

/// File-based voice activity detector.
///
/// Implementors process one fixed-size `f32` frame at a time and hold
/// any stateful context (LSTM hidden state for Silero, hysteresis
/// counters for [`SmoothedVad`]) across calls. `push_frame` is the
/// primary API; `is_voice` is a convenience that discards the Speech
/// slice.
pub trait VoiceActivityDetector: Send + Sync {
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>>;

    #[allow(dead_code)] // used by SmoothedVad and by filler-gap classifier (R-004).
    fn is_voice(&mut self, frame: &[f32]) -> Result<bool> {
        Ok(self.push_frame(frame)?.is_speech())
    }

    #[allow(dead_code)] // used when re-using a detector across analyses.
    fn reset(&mut self) {}
}

mod silero;
mod smoothed;
pub mod prefilter;

#[allow(unused_imports)] // consumed by Phase 2 callers per PRD R-002 / R-003 / R-004.
pub use silero::{
    SileroVad, DEFAULT_SILERO_THRESHOLD, SILERO_FRAME_MS, SILERO_FRAME_SAMPLES_16K,
};
#[allow(unused_imports)]
pub use smoothed::{SmoothedVad, DEFAULT_HANGOVER_FRAMES, DEFAULT_ONSET_FRAMES, DEFAULT_PREFILL_FRAMES};
