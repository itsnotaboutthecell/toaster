//! Canonical transcription adapter layer.
//!
//! Per-engine adapters normalize the heterogeneous `transcribe_rs::TranscriptionResult`
//! shape into a single `NormalizedTranscriptionResult` that the rest of the app
//! (editor, exports, precision eval) can consume without reasoning about engine
//! quirks. This operationalizes the Coupling Map / Proposed Canonical Schema
//! findings from the fleet audit (todo `p1-adapter-trait`).
//!
//! Responsibilities encapsulated here:
//!   * Strip engine-specific hallucinations and non-speech tokens.
//!   * Report whether the engine produced authoritative per-word timestamps.
//!   * Declare per-engine capabilities (prompt injection, fuzzy correction,
//!     pre-speech padding, native input sample rate) in one place — no more
//!     `is_whisper` bool tests scattered across `transcribe()`.
//!   * Normalize BCP-47 language codes per-engine (e.g. `zh-Hans` -> `zh` for
//!     Whisper/Cohere, dropped for Moonshine).
//!
//! Explicit non-goals (see todo list):
//!   * No forced alignment / stable-ts second pass — `p1-authoritative-flag-actionable`.
//!   * No refactor of `transcribe_file/mod.rs`'s refinement passes.
//!
//! # Contract invariant: no equal-duration synthesis
//!
//! Adapters MUST NOT produce a [`NormalizedTranscriptionResult`] whose word
//! timings are synthesized by distributing total audio duration evenly across
//! token count. This violates the precision/UX guardrail in AGENTS.md
//! ("Preserve precise transcription timing; never synthesize equal-duration
//! timestamps").
//!
//! If the underlying engine returns only a blob of text with no segments
//! (i.e. `TranscriptionResult::segments` is `None` or empty while `text` is
//! non-empty), the adapter MUST return `Err` with a message naming the
//! engine. The correct long-term fix for such an engine is forced alignment
//! inside the adapter (see `p1-authoritative-flag-actionable`), never the
//! reintroduction of equal-duration synthesis downstream.
//!
//! Char-proportional split within a **real** engine-provided segment span is
//! permitted — that's a distribution *within* authoritative segment timing,
//! not synthesis of the timing itself.
//!
//! # Per-engine timing source
//!
//! | Engine              | Adapter              | Timing source                                     | Authoritative? |
//! |---------------------|----------------------|---------------------------------------------------|----------------|
//! | Whisper             | `WhisperAdapter`     | segment-level; char-proportional split per seg    | no             |
//! | Parakeet            | `ParakeetAdapter`    | word-level when `TimestampGranularity::Word` set; char-split fallback for mixed segments | yes (word-level path) |
//! | Moonshine / streaming | `MoonshineAdapter` | segment-level; char-proportional split per seg    | no             |
//! | SenseVoice          | `SenseVoiceAdapter`  | segment-level; char-proportional split per seg    | no             |
//! | GigaAM              | `GigaAmAdapter`      | segment-level; char-proportional split per seg    | no             |
//! | Canary              | `CanaryAdapter`      | segment-level; char-proportional split per seg    | no             |
//! | Cohere              | `CohereAdapter`      | segment-level; char-proportional split per seg    | no             |
//! | Mock (CI)           | `MockAdapter`        | whatever the fixture / `fixed_text` produced; `fixed_text` is equal-duration and is test-only (see `transcription_mock.rs`) | word-level if fixture is |
//!
//! None of the production adapters contain an equal-duration synthesis
//! branch: when their engine truly produces no segments, they return `Err`
//! via [`require_segments`]. The legacy even-distribution fallback in
//! `commands::transcribe_file` has been removed (`p3-abandon-even-dist-fallback`).

use crate::managers::model::EngineType;
use anyhow::{anyhow, Result};
use log::warn;
use transcribe_rs::{TranscriptionResult, TranscriptionSegment};

use super::adapter_normalize::{
    make_normalized, segments_are_word_level, words_from_segments_native,
    words_from_segments_proportional,
};
#[cfg(test)]
use super::adapter_normalize::is_non_speech_token;

/// Fallback input sample rate when an adapter doesn't declare one — matches
/// what every current ASR engine in transcribe-rs actually accepts.
pub const ASR_INPUT_SAMPLE_RATE_HZ_DEFAULT: u32 = 16_000;

/// Canonical per-word result after adapter normalization. Sentinel values
/// preserve the existing `Word` struct's convention (`-1.0` / `-1`).
#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalWord {
    pub text: String,
    pub start_us: i64,
    pub end_us: i64,
    /// Confidence in `[0.0, 1.0]`. Sentinel `-1.0` = not provided by engine.
    pub confidence: f32,
    /// Speaker index. Sentinel `-1` = not provided.
    pub speaker_id: i32,
    /// True for engine-emitted non-speech / silence markers (`[MUSIC]`,
    /// `<|nospeech|>`, `<unk>`, `♪♪`, etc.). The adapter strips these before
    /// returning — they must never leak into the editor surface. The flag
    /// exists so diagnostic tooling can intentionally preserve them.
    pub is_non_speech: bool,
}

/// Normalized, engine-agnostic transcription result.
///
/// Invariants (enforced by [`Self::validate`] and `debug_assert!`s in adapter
/// impls):
///   1. `words` is non-empty when transcription succeeded (empty audio is
///      expressed by `words == []` and should be handled by the caller).
///   2. Monotonic non-overlap: `words[i].end_us <= words[i+1].start_us`.
///   3. Gaps are allowed (silence is a gap, not a token).
///   4. No zero-duration words: `start_us < end_us`.
///   5. `is_non_speech` tokens are stripped before returning.
#[derive(Debug, Clone)]
pub struct NormalizedTranscriptionResult {
    pub words: Vec<CanonicalWord>,
    /// Final text blob — the engine's `TranscriptionResult::text` after the
    /// manager's text-level post-processing (fuzzy custom-word correction,
    /// filler/stutter cleanup). Carried alongside `words` so downstream
    /// callers that still operate on the text blob
    /// (`commands::transcribe_file::build_words_from_segments`) don't have
    /// to round-trip through `words` and lose punctuation/case.
    pub text: String,
    /// Raw engine-reported segment timings, preserved for downstream forced
    /// alignment (`commands::transcribe_file::build_words_from_segments`).
    /// `None` only when the engine genuinely returned silence — adapters
    /// reject "text but no segments" via `require_segments`.
    pub segments: Option<Vec<TranscriptionSegment>>,
    /// BCP-47 code, or `"und"` if the engine didn't report a language.
    pub language: String,
    /// `true` if per-word timestamps delivered downstream are authoritative
    /// — either because the engine emitted per-word times verbatim
    /// (Parakeet word-level), or because the pipeline guarantees forced
    /// alignment will run in `commands::transcribe_file::build_words_from_segments`
    /// before the words reach the editor (Whisper, after
    /// `p1-authoritative-flag-actionable`).
    ///
    /// `false` means neither applies: downstream code will receive the
    /// adapter's char-proportional seed unchanged and is free to refine.
    pub word_timestamps_authoritative: bool,
}

impl NormalizedTranscriptionResult {
    /// Enforce invariants. Returns `Err` on violation, plus a human-readable
    /// message identifying the offending word index.
    pub fn validate(&self) -> Result<()> {
        for (i, w) in self.words.iter().enumerate() {
            if w.is_non_speech {
                return Err(anyhow!(
                    "validate: word {} ('{}') has is_non_speech=true; adapter must strip before returning",
                    i,
                    w.text
                ));
            }
            if w.start_us >= w.end_us {
                return Err(anyhow!(
                    "validate: word {} ('{}') has zero/negative duration: start_us={}, end_us={}",
                    i,
                    w.text,
                    w.start_us,
                    w.end_us
                ));
            }
            if i > 0 {
                let prev = &self.words[i - 1];
                if w.start_us < prev.end_us {
                    return Err(anyhow!(
                        "validate: word {} ('{}') overlaps previous: prev.end_us={}, start_us={}",
                        i,
                        w.text,
                        prev.end_us,
                        w.start_us
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Per-engine capability flags. Replaces the scattered `is_whisper` / engine
/// `match` arms that used to live in `TranscriptionManager::transcribe`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelCapabilities {
    /// Engine accepts an `initial_prompt` for biasing vocabulary (Whisper).
    /// When false, the manager falls back to post-hoc fuzzy word correction.
    pub supports_prompt_injection: bool,
    /// Engine output is eligible for the `apply_custom_words` fuzzy pass.
    /// Generally the inverse of `supports_prompt_injection` — Whisper already
    /// biased via prompt, so fuzzy correction is skipped for it.
    pub supports_fuzzy_word_correction: bool,
    /// Engine injects pre-speech padding (Parakeet). Waveform callers may opt
    /// into an outer-trim pass when this is set.
    pub has_pre_speech_padding: bool,
    /// Sample rate the engine expects for PCM input, in Hz.
    pub native_input_sample_rate_hz: u32,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            supports_prompt_injection: false,
            supports_fuzzy_word_correction: true,
            has_pre_speech_padding: false,
            native_input_sample_rate_hz: ASR_INPUT_SAMPLE_RATE_HZ_DEFAULT,
        }
    }
}

/// Metadata about the input PCM buffer handed to the adapter. Lets adapters
/// that don't rely on per-segment times (or need to clamp to real duration)
/// produce sane fallbacks.
#[derive(Debug, Clone, Copy)]
pub struct AudioInfo {
    pub duration_us: i64,
}

impl AudioInfo {
    pub fn from_samples(samples: usize, sample_rate_hz: u32, channels: u16) -> Self {
        // Sample count -> microseconds; use f64 math with rounding to avoid
        // bias toward earlier samples (see audio_toolkit::timing rounding
        // policy).
        let rate = sample_rate_hz.max(1) as f64;
        let frames = samples / channels.max(1) as usize;
        let duration_us = ((frames as f64 / rate) * 1_000_000.0).round() as i64;
        Self { duration_us }
    }
}

/// Engine-agnostic normalization surface. Each variant of `LoadedEngine` has
/// a corresponding static impl of this trait; the manager looks one up via
/// [`adapter_for_engine`] before and after dispatch.
pub trait TranscriptionModelAdapter: Send + Sync {
    /// Static capability flags. Called from both pre-dispatch (to decide
    /// `initial_prompt` vs `apply_custom_words`) and post-dispatch.
    fn capabilities(&self) -> ModelCapabilities;

    /// Normalize a BCP-47 code the UI hands us into the subset the engine
    /// accepts. `"auto"` MUST map to `None`. Engines that ignore language
    /// (Moonshine, GigaAM) return `None` for every input.
    fn normalize_language(&self, raw: &str) -> Option<String>;

    /// Normalize the raw `TranscriptionResult` into the canonical schema.
    /// Strips hallucinations, enforces invariants, and validates before
    /// returning.
    fn adapt(
        &self,
        raw: TranscriptionResult,
        audio_info: AudioInfo,
    ) -> Result<NormalizedTranscriptionResult>;
}

/// Helper: extract a segment slice from a `TranscriptionResult`, whose
/// `segments` field is `Option<Vec<_>>` (some engines return `None`).
fn segments_of(raw: &TranscriptionResult) -> &[TranscriptionSegment] {
    raw.segments.as_deref().unwrap_or(&[])
}

/// Enforce the "no text-without-timing" contract. Returns `Err` when the
/// engine emitted textual content but no segment-level timings — equal-
/// duration synthesis is forbidden, so the adapter cannot recover.
///
/// Empty-text + empty-segments (true silence) is Ok; the caller gets an
/// empty word list and handles it upstream.
fn require_segments(engine: &str, raw: &TranscriptionResult) -> Result<()> {
    let has_text = !raw.text.trim().is_empty();
    let has_segments = raw
        .segments
        .as_ref()
        .is_some_and(|s| s.iter().any(|seg| !seg.text.trim().is_empty()));
    if has_text && !has_segments {
        return Err(anyhow!(
            "{engine} adapter: engine returned text ({} chars) but no segment-level \
             timestamps; equal-duration synthesis is forbidden. Fix the engine \
             configuration to emit segments, or add forced alignment in the adapter \
             (see p1-authoritative-flag-actionable).",
            raw.text.len()
        ));
    }
    Ok(())
}

/// Return the adapter singleton for a given engine. Adapters are zero-state,
/// so we hand out `&'static` references.
pub fn adapter_for_engine(engine: &EngineType) -> &'static dyn TranscriptionModelAdapter {
    match engine {
        EngineType::Whisper => &WhisperAdapter,
        EngineType::Parakeet => &ParakeetAdapter,
        EngineType::Moonshine => &MoonshineAdapter,
        EngineType::MoonshineStreaming => &MoonshineAdapter,
        EngineType::SenseVoice => &SenseVoiceAdapter,
        EngineType::GigaAM => &GigaAmAdapter,
        EngineType::Canary => &CanaryAdapter,
        EngineType::Cohere => &CohereAdapter,
    }
}

// ── per-engine adapters ────────────────────────────────────────────────────

pub struct WhisperAdapter;

impl TranscriptionModelAdapter for WhisperAdapter {
    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            supports_prompt_injection: true,
            supports_fuzzy_word_correction: false,
            has_pre_speech_padding: false,
            native_input_sample_rate_hz: 16_000,
        }
    }

    fn normalize_language(&self, raw: &str) -> Option<String> {
        if raw == "auto" || raw.is_empty() {
            return None;
        }
        match raw {
            "zh-Hans" | "zh-Hant" => Some("zh".to_string()),
            other => Some(other.to_string()),
        }
    }

    fn adapt(
        &self,
        raw: TranscriptionResult,
        audio_info: AudioInfo,
    ) -> Result<NormalizedTranscriptionResult> {
        require_segments("Whisper", &raw)?;
        let words = words_from_segments_proportional(segments_of(&raw), audio_info);
        // Whisper-backed transcription is authoritative by pipeline
        // contract: `commands::transcribe_file::build_words_from_segments`
        // runs DP forced alignment (see
        // `audio_toolkit::forced_alignment::align_words_in_segment`) on
        // every Whisper segment before words reach the editor. The
        // char-proportional seed produced here is only used by tests and
        // by the degenerate-segment fallback inside that function.
        // Todo: `p1-authoritative-flag-actionable`.
        make_normalized(raw, words, true)
    }
}

pub struct ParakeetAdapter;

impl TranscriptionModelAdapter for ParakeetAdapter {
    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            supports_prompt_injection: false,
            supports_fuzzy_word_correction: true,
            has_pre_speech_padding: true,
            native_input_sample_rate_hz: 16_000,
        }
    }

    fn normalize_language(&self, _raw: &str) -> Option<String> {
        // Parakeet doesn't accept a language hint at the transcribe_rs layer.
        None
    }

    fn adapt(
        &self,
        raw: TranscriptionResult,
        audio_info: AudioInfo,
    ) -> Result<NormalizedTranscriptionResult> {
        require_segments("Parakeet", &raw)?;
        let segs = segments_of(&raw);
        let word_level = segments_are_word_level(segs);
        let words = if word_level {
            words_from_segments_native(segs, audio_info)
        } else {
            warn!(
                "ParakeetAdapter: {} segments did not look word-level, falling back to char-split",
                segs.len()
            );
            words_from_segments_proportional(segs, audio_info)
        };
        make_normalized(raw, words, word_level)
    }
}

/// TODO(audit): native word-timing support not yet verified. Currently
/// routes through the char-proportional split path. Applies to plain
/// Moonshine and Moonshine streaming.
pub struct MoonshineAdapter;

impl TranscriptionModelAdapter for MoonshineAdapter {
    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities::default()
    }

    fn normalize_language(&self, _raw: &str) -> Option<String> {
        None
    }

    fn adapt(
        &self,
        raw: TranscriptionResult,
        audio_info: AudioInfo,
    ) -> Result<NormalizedTranscriptionResult> {
        require_segments("Moonshine", &raw)?;
        let words = words_from_segments_proportional(segments_of(&raw), audio_info);
        make_normalized(raw, words, false)
    }
}

/// TODO(audit): native word-timing support not yet verified.
pub struct SenseVoiceAdapter;

impl TranscriptionModelAdapter for SenseVoiceAdapter {
    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities::default()
    }

    fn normalize_language(&self, raw: &str) -> Option<String> {
        match raw {
            "auto" | "" => None,
            "zh" | "zh-Hans" | "zh-Hant" => Some("zh".to_string()),
            "en" | "ja" | "ko" | "yue" => Some(raw.to_string()),
            _ => None,
        }
    }

    fn adapt(
        &self,
        raw: TranscriptionResult,
        audio_info: AudioInfo,
    ) -> Result<NormalizedTranscriptionResult> {
        require_segments("SenseVoice", &raw)?;
        let words = words_from_segments_proportional(segments_of(&raw), audio_info);
        make_normalized(raw, words, false)
    }
}

/// TODO(audit): native word-timing support not yet verified.
pub struct GigaAmAdapter;

impl TranscriptionModelAdapter for GigaAmAdapter {
    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities::default()
    }

    fn normalize_language(&self, _raw: &str) -> Option<String> {
        None
    }

    fn adapt(
        &self,
        raw: TranscriptionResult,
        audio_info: AudioInfo,
    ) -> Result<NormalizedTranscriptionResult> {
        require_segments("GigaAM", &raw)?;
        let words = words_from_segments_proportional(segments_of(&raw), audio_info);
        make_normalized(raw, words, false)
    }
}

/// TODO(audit): native word-timing support not yet verified.
pub struct CanaryAdapter;

impl TranscriptionModelAdapter for CanaryAdapter {
    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities::default()
    }

    fn normalize_language(&self, raw: &str) -> Option<String> {
        if raw == "auto" || raw.is_empty() {
            None
        } else {
            Some(raw.to_string())
        }
    }

    fn adapt(
        &self,
        raw: TranscriptionResult,
        audio_info: AudioInfo,
    ) -> Result<NormalizedTranscriptionResult> {
        require_segments("Canary", &raw)?;
        let words = words_from_segments_proportional(segments_of(&raw), audio_info);
        make_normalized(raw, words, false)
    }
}

/// TODO(audit): native word-timing support not yet verified.
pub struct CohereAdapter;

impl TranscriptionModelAdapter for CohereAdapter {
    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities::default()
    }

    fn normalize_language(&self, raw: &str) -> Option<String> {
        match raw {
            "auto" | "" => None,
            "zh-Hans" | "zh-Hant" => Some("zh".to_string()),
            other => Some(other.to_string()),
        }
    }

    fn adapt(
        &self,
        raw: TranscriptionResult,
        audio_info: AudioInfo,
    ) -> Result<NormalizedTranscriptionResult> {
        require_segments("Cohere", &raw)?;
        let words = words_from_segments_proportional(segments_of(&raw), audio_info);
        make_normalized(raw, words, false)
    }
}

/// Mock adapter used by CI tests. Fixtures are hand-curated, so the adapter
/// reports `word_timestamps_authoritative: true` when the fixture is already
/// word-level — it is honest about its timing quality. Exposed for use by
/// this module's own tests; gated to `#[cfg(test)]` since production code
/// never constructs it (CI uses the separate mock in `transcription_mock.rs`).
#[cfg(test)]
pub struct MockAdapter;

#[cfg(test)]
impl TranscriptionModelAdapter for MockAdapter {
    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            supports_prompt_injection: false,
            supports_fuzzy_word_correction: true,
            has_pre_speech_padding: false,
            native_input_sample_rate_hz: ASR_INPUT_SAMPLE_RATE_HZ_DEFAULT,
        }
    }

    fn normalize_language(&self, raw: &str) -> Option<String> {
        if raw == "auto" || raw.is_empty() {
            None
        } else {
            Some(raw.to_string())
        }
    }

    fn adapt(
        &self,
        raw: TranscriptionResult,
        audio_info: AudioInfo,
    ) -> Result<NormalizedTranscriptionResult> {
        // The mock adapter is exercised by CI tests; it still must obey the
        // "no equal-duration synthesis in the adapter" contract. The
        // `MockTranscription::FixedText` variant in `transcription_mock.rs`
        // synthesizes segments in the mock-manager layer — that's test
        // scaffolding, explicitly labelled as such, and lives above this
        // adapter. Here we enforce the same contract as production.
        require_segments("Mock", &raw)?;
        let segs = segments_of(&raw);
        let word_level = segments_are_word_level(segs);
        let words = if word_level {
            words_from_segments_native(segs, audio_info)
        } else {
            words_from_segments_proportional(segs, audio_info)
        };
        make_normalized(raw, words, word_level)
    }
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
