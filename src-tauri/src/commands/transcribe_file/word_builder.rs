//! Word-from-segment construction + timestamp sanitization (extracted from mod.rs).

use log::{info, warn};
use transcribe_rs::TranscriptionSegment;

use super::{WordAlignmentMeta, SAMPLE_RATE_HZ};
use crate::audio_toolkit::timing::{round_f64_to_i64, seconds_to_us as timing_seconds_to_us};
use crate::managers::editor::Word;

/// Build word-level timestamps from transcription segments.
///
/// Primary alignment is the DP-based forced aligner in
/// [`crate::audio_toolkit::forced_alignment`]: for each ASR segment, the
/// interior boundaries are placed at the frames that minimize the sum of
/// local acoustic energy and deviation from their char-proportional expected
/// position. The segment endpoints themselves come from the ASR engine and
/// are treated as authoritative.
///
/// **Fallback to char-proportional split.** When the aligner declines a
/// segment (returns `None`) — e.g. the slice is too short to produce enough
/// energy frames, or no valid interior search window exists — we fall back
/// to the legacy character-proportional distribution. This keeps behavior
/// defined for degenerate segments and for future engines whose adapter
/// reports `word_timestamps_authoritative == false` but whose audio slice
/// is too small to align. The downstream safety-net passes
/// (`correct_short_word_boundaries`, `refine_word_boundaries`,
/// `align_onset_boundaries`, and `realign_suspicious_spans` at the call
/// site) run regardless and may refine both paths further.
pub(super) fn build_words_from_segments(
    full_text: &str,
    segments: &[TranscriptionSegment],
    samples: &[f32],
) -> (Vec<Word>, Vec<WordAlignmentMeta>) {
    let mut words = Vec::new();
    let mut meta = Vec::new();

    // The filtered text may differ from segment text (due to filler filtering,
    // custom word correction). We'll use the final text's words and match them
    // against segment boundaries for the best timestamp assignment.
    let final_words: Vec<&str> = full_text.split_whitespace().collect();

    if final_words.is_empty() || segments.is_empty() {
        return (words, meta);
    }

    // Build a flat list of (word, start_us, end_us) from segments first.
    // For each segment we prefer the DP forced aligner; if it declines we
    // fall through to the legacy char-proportional split.
    let mut segment_words: Vec<(String, i64, i64)> = Vec::new();
    for seg in segments {
        let seg_text = seg.text.trim();
        if seg_text.is_empty() {
            continue;
        }
        // Half-open convention: a segment covers [seg_start_us, seg_end_us).
        // Both ends are rounded with the same nearest-integer policy so the
        // duration `end - start` is not biased by mixing floor+ceil.
        let seg_start_us = timing_seconds_to_us(seg.start as f64);
        let seg_end_us = timing_seconds_to_us(seg.end as f64);
        let seg_duration_us = seg_end_us - seg_start_us;

        let seg_words: Vec<&str> = seg_text.split_whitespace().collect();
        if seg_words.is_empty() {
            continue;
        }

        // Primary path: DP forced alignment against frame-level RMS.
        if let Some(aligned) = crate::audio_toolkit::forced_alignment::align_words_in_segment(
            &seg_words,
            seg_start_us,
            seg_end_us,
            samples,
            SAMPLE_RATE_HZ,
        ) {
            for (sw, (ws, we)) in seg_words.iter().zip(aligned.into_iter()) {
                segment_words.push(((*sw).to_string(), ws, we));
            }
            continue;
        }

        // Fallback path: char-proportional split. Fires when the aligner
        // cannot run (segment too short, too few frames, or slice outside
        // the sample buffer). Kept so degenerate segments still produce
        // *some* ordered output; downstream refinement may fix the worst
        // of it.
        const MIN_WORD_CHAR_WEIGHT: usize = 1;
        let total_chars: usize = seg_words
            .iter()
            .map(|w| w.len().max(MIN_WORD_CHAR_WEIGHT))
            .sum();

        let mut cursor_us = seg_start_us;
        for (j, sw) in seg_words.iter().enumerate() {
            let char_fraction = sw.len().max(MIN_WORD_CHAR_WEIGHT) as f64 / total_chars as f64;
            let word_duration_us = round_f64_to_i64(seg_duration_us as f64 * char_fraction);

            let word_start = cursor_us;
            let word_end = if j == seg_words.len() - 1 {
                seg_end_us // last word gets the remainder to avoid gaps
            } else {
                cursor_us + word_duration_us
            };

            segment_words.push((sw.to_string(), word_start, word_end));
            cursor_us = word_end;
        }
    }

    // Now match filtered final_words against segment_words.
    // The final text may have had filler words removed or words corrected,
    // so we do a greedy forward match. If a final word matches a segment word,
    // use that segment word's timestamps. If not, interpolate.
    let mut seg_idx = 0;
    let mut interpolated_count = 0usize;
    let mut interpolation_examples: Vec<String> = Vec::new();
    for fw in &final_words {
        let fw_lower = fw.to_lowercase();

        // Try to find a matching segment word from current position forward.
        // Use a large lookahead (20 words) to tolerate filler removal, stutters,
        // and word corrections that can shift alignment significantly.
        let mut found = false;
        let search_limit = (seg_idx + 20).min(segment_words.len());
        for (k, seg_word) in segment_words
            .iter()
            .enumerate()
            .skip(seg_idx)
            .take(search_limit.saturating_sub(seg_idx))
        {
            let seg_word_lower = seg_word.0.to_lowercase();
            // Fuzzy match: segment text might have punctuation attached
            if seg_word_lower == fw_lower
                || seg_word_lower.starts_with(&fw_lower)
                || fw_lower.starts_with(&seg_word_lower)
                || seg_word_lower.trim_matches(|c: char| !c.is_alphanumeric()) == fw_lower
            {
                words.push(Word {
                    text: fw.to_string(),
                    start_us: seg_word.1,
                    end_us: seg_word.2,
                    deleted: false,
                    silenced: false,
                    confidence: -1.0,
                    speaker_id: -1,
                });
                meta.push(WordAlignmentMeta {
                    interpolated: false,
                });
                seg_idx = k + 1;
                found = true;
                break;
            }
        }

        if !found {
            // No match found — interpolate from nearest segment word and advance
            // the pointer so subsequent words don't all pile up at the same position
            let (start, end) = if seg_idx < segment_words.len() {
                let ts = (segment_words[seg_idx].1, segment_words[seg_idx].2);
                seg_idx += 1; // advance past this word to prevent repeated timestamps
                ts
            } else if let Some(last) = segment_words.last() {
                (last.1, last.2)
            } else {
                (0, 0)
            };
            interpolated_count += 1;
            if interpolation_examples.len() < 5 {
                interpolation_examples.push((*fw).to_string());
            }
            words.push(Word {
                text: fw.to_string(),
                start_us: start,
                end_us: end,
                deleted: false,
                silenced: false,
                confidence: -1.0,
                speaker_id: -1,
            });
            meta.push(WordAlignmentMeta { interpolated: true });
        }
    }

    if interpolated_count > 0 {
        let ratio = interpolated_count as f64 / final_words.len() as f64;
        let sample_words = interpolation_examples.join(", ");
        if ratio >= 0.20 {
            warn!(
                "build_words_from_segments: high interpolation rate {}/{} ({:.1}%). examples: [{}]",
                interpolated_count,
                final_words.len(),
                ratio * 100.0,
                sample_words
            );
        } else {
            info!(
                "build_words_from_segments: interpolated {}/{} words ({:.1}%). examples: [{}]",
                interpolated_count,
                final_words.len(),
                ratio * 100.0,
                sample_words
            );
        }
    }

    // Pre-correction for short-word proportional boundaries
    correct_short_word_boundaries(&mut words, samples);

    // Refine word boundaries by snapping to silence points in the audio
    refine_word_boundaries(&mut words, samples);

    // Align segment-leading word starts to true speech onset
    align_onset_boundaries(&mut words, samples);

    (words, meta)
}

/// Sanitize word timestamps to guarantee monotonic, non-overlapping,
/// duration-positive ordering within [0, total_audio_duration_us].
///
/// Whisper segments (and proportional distribution within them) can
/// occasionally produce:
///   - start > end (inverted range)
///   - next.start < prev.end (overlap / rewind)
///   - values outside the actual audio duration
///
/// All of these break keep-segment calculation and cause playback jumps.
/// This function fixes them in a single forward pass without altering the
/// ordering of words.
pub(super) fn sanitize_word_timestamps(words: &mut [Word], total_duration_us: i64) {
    const MIN_WORD_DURATION_US: i64 = 1_000; // 1 ms minimum word duration

    let max_us = total_duration_us.max(0);
    let mut cursor_us: i64 = 0; // tracks the earliest start allowed for the next word

    for word in words.iter_mut() {
        // 1. Clamp both endpoints into [0, max_us]
        word.start_us = word.start_us.clamp(0, max_us);
        word.end_us = word.end_us.clamp(0, max_us);

        // 2. Enforce start <= end
        if word.start_us > word.end_us {
            word.end_us = word.start_us;
        }

        // 3. Enforce monotonic progression: start must be >= cursor
        if word.start_us < cursor_us {
            word.start_us = cursor_us;
            // Re-clamp start after shift
            word.start_us = word.start_us.min(max_us);
            // Ensure end is still >= start after shift
            if word.end_us < word.start_us {
                word.end_us = word.start_us;
            }
        }

        // 4. Ensure minimal non-zero duration where audio budget allows
        if word.end_us == word.start_us && word.start_us + MIN_WORD_DURATION_US <= max_us {
            word.end_us = word.start_us + MIN_WORD_DURATION_US;
        }

        cursor_us = word.end_us;
    }
}
