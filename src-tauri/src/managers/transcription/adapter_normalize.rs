//! Shared normalization helpers extracted from `adapter.rs` to keep the
//! per-engine adapter file under the 800-line cap.
//!
//! Per-engine [`super::TranscriptionModelAdapter`] impls compose these
//! helpers to produce a [`super::NormalizedTranscriptionResult`]. The
//! invariants enforced here back the
//! [transcription-adapter-contract](../../../../.github/skills/transcription-adapter-contract/SKILL.md)
//! gate: monotonic non-overlapping word spans, no zero-duration words, and
//! stripped non-speech tokens. No equal-duration synthesis happens here
//! either — the contract invariant documented in `adapter.rs` still holds.

use anyhow::Result;
use log::debug;
use transcribe_rs::{TranscriptionResult, TranscriptionSegment};

use super::adapter::{AudioInfo, CanonicalWord, NormalizedTranscriptionResult};

/// Patterns treated as non-speech / hallucination by every adapter that
/// uses [`is_non_speech_token`]. Intentionally conservative; precise filler
/// filtering lives downstream in `filter_transcription_output`.
const NON_SPEECH_MARKERS: &[&str] = &[
    "[MUSIC]",
    "[Music]",
    "[music]",
    "[APPLAUSE]",
    "[Applause]",
    "[applause]",
    "[LAUGHTER]",
    "[Laughter]",
    "[laughter]",
    "[SILENCE]",
    "[silence]",
    "[INAUDIBLE]",
    "[inaudible]",
    "(music)",
    "(applause)",
    "<|nospeech|>",
    "<|silence|>",
    "<unk>",
];

/// Returns `true` for tokens the adapter should strip. Matches bracketed
/// markers, Whisper special tokens, and common music-note hallucinations
/// (`♪`, `♫`). Whole-token match only — text like "the music" is not
/// filtered here.
pub(super) fn is_non_speech_token(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    if NON_SPEECH_MARKERS
        .iter()
        .any(|m| m.eq_ignore_ascii_case(trimmed))
    {
        return true;
    }
    // Music-note hallucinations: a run of ♪/♫ (optionally with whitespace).
    if trimmed.chars().all(|c| matches!(c, '♪' | '♫' | ' ' | '\t')) {
        return true;
    }
    // Runs of 4+ identical punctuation (.... / ---- / ==== etc.) — common
    // Whisper hallucination on silence.
    if trimmed.chars().count() >= 4 {
        let first = trimmed.chars().next().unwrap();
        if !first.is_alphanumeric() && trimmed.chars().all(|c| c == first) {
            return true;
        }
    }
    false
}

/// Seconds (from `TranscriptionSegment`) -> microseconds. Uses
/// nearest-integer rounding to match `audio_toolkit::timing::seconds_to_us`.
fn seconds_to_us(s: f32) -> i64 {
    (s as f64 * 1_000_000.0).round() as i64
}

/// Char-proportional split of a segment's text across `[start_us, end_us)`.
/// Used by engines whose segment times are authoritative but whose word
/// boundaries aren't (Whisper, Moonshine, SenseVoice, GigaAM, Canary,
/// Cohere). See `build_words_from_segments` in `transcribe_file/mod.rs` for
/// the richer downstream refinement pass — the adapter only produces the
/// proportional baseline so the invariants hold.
fn split_segment_by_chars(seg_text: &str, start_us: i64, end_us: i64) -> Vec<(String, i64, i64)> {
    const MIN_WORD_CHAR_WEIGHT: usize = 1;
    let words: Vec<&str> = seg_text.split_whitespace().collect();
    if words.is_empty() || end_us <= start_us {
        return Vec::new();
    }
    let total: usize = words
        .iter()
        .map(|w| w.len().max(MIN_WORD_CHAR_WEIGHT))
        .sum();
    let duration_us = end_us - start_us;
    let mut out = Vec::with_capacity(words.len());
    let mut cursor = start_us;
    for (i, w) in words.iter().enumerate() {
        let share = (w.len().max(MIN_WORD_CHAR_WEIGHT) as f64 / total as f64 * duration_us as f64)
            .round() as i64;
        let word_end = if i == words.len() - 1 {
            end_us
        } else {
            cursor + share
        };
        out.push((w.to_string(), cursor, word_end));
        cursor = word_end;
    }
    out
}

/// Strip non-speech segments and enforce the canonical invariants (monotonic,
/// non-overlapping, non-zero-duration). Words flagged `is_non_speech` are
/// removed here so they never appear in the returned result.
fn finalize_words(mut words: Vec<CanonicalWord>, audio_info: AudioInfo) -> Vec<CanonicalWord> {
    // Strip non-speech tokens. We never emit them.
    words.retain(|w| !w.is_non_speech);

    if words.is_empty() {
        return words;
    }

    // Clamp to audio duration and enforce monotonic / non-zero-duration.
    let max_us = audio_info.duration_us.max(0);
    let mut cursor: i64 = 0;
    let mut out: Vec<CanonicalWord> = Vec::with_capacity(words.len());

    for mut w in words {
        // Clamp into [0, max_us] if we have a known duration; otherwise
        // trust the engine's times (max_us == 0 means "unknown").
        if max_us > 0 {
            w.start_us = w.start_us.clamp(0, max_us);
            w.end_us = w.end_us.clamp(0, max_us);
        }
        if w.start_us < cursor {
            w.start_us = cursor;
        }
        if w.end_us <= w.start_us {
            // Grant a 1 ms floor when audio budget allows; otherwise drop.
            let floor = w.start_us + 1_000;
            if max_us == 0 || floor <= max_us {
                w.end_us = floor;
            } else {
                continue;
            }
        }
        cursor = w.end_us;
        out.push(w);
    }
    out
}

/// Build `CanonicalWord`s from raw segments using char-proportional split.
/// Shared by every non-word-level adapter.
pub(super) fn words_from_segments_proportional(
    segments: &[TranscriptionSegment],
    audio_info: AudioInfo,
) -> Vec<CanonicalWord> {
    let mut words: Vec<CanonicalWord> = Vec::new();
    for seg in segments {
        let text = seg.text.trim();
        if text.is_empty() {
            continue;
        }
        if is_non_speech_token(text) {
            debug!("adapter: stripping non-speech segment: {:?}", text);
            continue;
        }
        let start_us = seconds_to_us(seg.start);
        let end_us = seconds_to_us(seg.end);
        for (word_text, ws, we) in split_segment_by_chars(text, start_us, end_us) {
            if is_non_speech_token(&word_text) {
                continue;
            }
            words.push(CanonicalWord {
                text: word_text,
                start_us: ws,
                end_us: we,
                confidence: -1.0,
                speaker_id: -1,
                is_non_speech: false,
            });
        }
    }
    finalize_words(words, audio_info)
}

/// Build `CanonicalWord`s from per-word segments, preserving native times.
/// Used when the adapter detects one-word-per-segment output (Parakeet with
/// `TimestampGranularity::Word`).
pub(super) fn words_from_segments_native(
    segments: &[TranscriptionSegment],
    audio_info: AudioInfo,
) -> Vec<CanonicalWord> {
    let mut words: Vec<CanonicalWord> = Vec::with_capacity(segments.len());
    for seg in segments {
        let text = seg.text.trim();
        if text.is_empty() || is_non_speech_token(text) {
            continue;
        }
        words.push(CanonicalWord {
            text: text.to_string(),
            start_us: seconds_to_us(seg.start),
            end_us: seconds_to_us(seg.end),
            confidence: -1.0,
            speaker_id: -1,
            is_non_speech: false,
        });
    }
    finalize_words(words, audio_info)
}

/// Heuristic used by `ParakeetAdapter` to decide between native per-word times
/// and the char-split fallback: if >=80% of segments contain exactly one
/// whitespace-separated token, treat segments as word-level.
pub(super) fn segments_are_word_level(segments: &[TranscriptionSegment]) -> bool {
    if segments.is_empty() {
        return false;
    }
    let single = segments
        .iter()
        .filter(|s| s.text.split_whitespace().count() == 1)
        .count();
    (single as f64) / (segments.len() as f64) >= 0.8
}

/// Build + validate a `NormalizedTranscriptionResult` from the parts each
/// adapter produces. Centralizing this removes 9 copies of the same struct
/// literal + `validate()?` pattern and is the single place that carries
/// `raw.text` / `raw.segments` onto the normalized result.
pub(super) fn make_normalized(
    raw: TranscriptionResult,
    words: Vec<CanonicalWord>,
    word_timestamps_authoritative: bool,
) -> Result<NormalizedTranscriptionResult> {
    let result = NormalizedTranscriptionResult {
        words,
        text: raw.text,
        segments: raw.segments,
        language: "und".to_string(),
        word_timestamps_authoritative,
    };
    result.validate()?;
    Ok(result)
}
