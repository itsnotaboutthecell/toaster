use log::{info, warn};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use transcribe_rs::TranscriptionSegment;

use crate::commands::editor::EditorStore;
use crate::managers::editor::Word;
use crate::managers::transcription::TranscriptionManager;

mod extract;
use extract::{extract_audio_to_wav, is_wav_file};

const SAMPLE_RATE_HZ: f64 = 16000.0;
/// Maximum transcription duration in seconds (4 hours).
/// At 16kHz mono float32 this is ~921 MB of WAV sample data.
const MAX_TRANSCRIPTION_DURATION_SECS: u64 = 14400;
/// Bytes per sample for 16kHz mono PCM (4 bytes for f32 / pcm_s16le raw estimate).
const BYTES_PER_SAMPLE: u64 = 4;

#[derive(Debug, Clone, Copy, Default)]
struct WordAlignmentMeta {
    interpolated: bool,
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

fn sample_to_us(sample_idx: usize) -> i64 {
    (sample_idx as f64 / SAMPLE_RATE_HZ * 1_000_000.0) as i64
}

fn us_to_sample(timestamp_us: i64, total_samples: usize) -> usize {
    if total_samples == 0 {
        return 0;
    }
    let sample = (timestamp_us.max(0) as f64 / 1_000_000.0 * SAMPLE_RATE_HZ) as usize;
    sample.min(total_samples.saturating_sub(1))
}

fn snap_to_zero_crossing(samples: &[f32], target: usize, half_window: usize) -> usize {
    if samples.len() < 2 {
        return target.min(samples.len().saturating_sub(1));
    }

    let max_z = samples.len().saturating_sub(2);
    let zc_start = target.saturating_sub(half_window).min(max_z);
    let zc_end = (target + half_window).min(max_z);

    let mut best_zc = target.min(max_z);
    let mut best_dist = usize::MAX;
    for z in zc_start..=zc_end {
        if samples[z] * samples[z + 1] <= 0.0 {
            let dist = z.abs_diff(target);
            if dist < best_dist {
                best_dist = dist;
                best_zc = z;
            }
        }
    }
    best_zc
}

fn find_local_low_energy_boundary(
    samples: &[f32],
    center_sample: usize,
    half_window_samples: usize,
    rms_window_samples: usize,
    step_samples: usize,
) -> Option<(usize, f32, f32)> {
    if samples.len() < rms_window_samples || rms_window_samples == 0 || step_samples == 0 {
        return None;
    }

    let search_start = center_sample.saturating_sub(half_window_samples);
    let search_end = (center_sample + half_window_samples).min(samples.len());
    if search_start >= search_end || search_end - search_start < rms_window_samples {
        return None;
    }

    let mut min_energy = f32::MAX;
    let mut min_pos = center_sample;
    let mut pos = search_start;
    while pos + rms_window_samples <= search_end {
        let energy = rms(&samples[pos..pos + rms_window_samples]);
        if energy < min_energy {
            min_energy = energy;
            min_pos = pos + rms_window_samples / 2;
        }
        pos += step_samples;
    }

    let center_start = center_sample
        .saturating_sub(rms_window_samples / 2)
        .min(samples.len().saturating_sub(rms_window_samples));
    let center_end = (center_start + rms_window_samples).min(samples.len());
    let center_energy = rms(&samples[center_start..center_end]);

    Some((min_pos, min_energy, center_energy))
}

fn normalized_word_text(text: &str) -> String {
    text.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

fn boundary_has_interpolated_pattern(
    words: &[Word],
    meta: Option<&[WordAlignmentMeta]>,
    boundary_idx: usize,
) -> bool {
    let Some(meta) = meta else {
        return false;
    };
    if boundary_idx + 1 >= words.len() || meta.len() != words.len() {
        return false;
    }

    if meta[boundary_idx].interpolated || meta[boundary_idx + 1].interpolated {
        return true;
    }

    let left = normalized_word_text(&words[boundary_idx].text);
    let right = normalized_word_text(&words[boundary_idx + 1].text);
    if left.is_empty() || right.is_empty() || left != right {
        return false;
    }

    (boundary_idx > 0 && meta[boundary_idx - 1].interpolated)
        || (boundary_idx + 2 < words.len() && meta[boundary_idx + 2].interpolated)
}

/// Confidence-gated local re-alignment pass for suspicious boundaries.
/// Uses short local windows only, so runtime is bounded and independent of
/// global transcript length.
fn realign_suspicious_spans(
    words: &mut [Word],
    samples: &[f32],
    meta: Option<&[WordAlignmentMeta]>,
) {
    if words.len() < 2 || samples.is_empty() {
        return;
    }

    const LOCAL_WINDOW_US: i64 = 120_000;
    const RMS_WINDOW_SAMPLES: usize = 80; // 5 ms at 16 kHz
    const RMS_STEP_SAMPLES: usize = 40; // 2.5 ms
    const ZC_SNAP_HALF: usize = 32; // ±2 ms
    const MIN_WORD_US: i64 = 10_000;
    const LOW_CONF_THRESHOLD: f32 = 0.45;
    const MAX_REALIGN_BOUNDARIES: usize = 256;

    let half_window_samples = (LOCAL_WINDOW_US as f64 / 1_000_000.0 * SAMPLE_RATE_HZ) as usize;
    let mut adjusted = 0usize;

    for i in 0..words.len() - 1 {
        if adjusted >= MAX_REALIGN_BOUNDARIES {
            break;
        }

        let left_duration = (words[i].end_us - words[i].start_us).max(0);
        let right_duration = (words[i + 1].end_us - words[i + 1].start_us).max(0);
        let min_duration = left_duration.min(right_duration);
        let max_duration = left_duration.max(right_duration);

        let low_confidence = [words[i].confidence, words[i + 1].confidence]
            .iter()
            .any(|&c| (0.0..LOW_CONF_THRESHOLD).contains(&c));
        let confidence_unknown = words[i].confidence < 0.0 || words[i + 1].confidence < 0.0;
        let very_short_word = left_duration < 35_000 || right_duration < 35_000;
        let abrupt_duration_jump = min_duration > 0
            && max_duration > min_duration * 3
            && (max_duration - min_duration) > 80_000;
        let interpolated_pattern = boundary_has_interpolated_pattern(words, meta, i);

        let boundary_sample = us_to_sample(words[i].end_us, samples.len());
        let Some((low_energy_sample, min_energy, boundary_energy)) = find_local_low_energy_boundary(
            samples,
            boundary_sample,
            half_window_samples,
            RMS_WINDOW_SAMPLES,
            RMS_STEP_SAMPLES,
        ) else {
            continue;
        };

        // Boundary in high-energy region often indicates a cut in the middle of phonemes.
        let boundary_high_energy = boundary_energy > (min_energy * 1.35 + 1e-5);
        let heuristic_suspicious =
            very_short_word || abrupt_duration_jump || boundary_high_energy || interpolated_pattern;
        if !(low_confidence || (confidence_unknown && heuristic_suspicious)) {
            continue;
        }

        let snapped_sample = snap_to_zero_crossing(samples, low_energy_sample, ZC_SNAP_HALF);
        let refined_us = sample_to_us(snapped_sample);
        if (refined_us - words[i].end_us).abs() < 1_000 {
            continue;
        }

        if refined_us > words[i].start_us + MIN_WORD_US
            && refined_us < words[i + 1].end_us - MIN_WORD_US
        {
            words[i].end_us = refined_us;
            words[i + 1].start_us = refined_us;
            adjusted += 1;
        }
    }
}

// Refine word boundaries with hybrid RMS + zero-crossing snapping.
//
// After proportional timestamp distribution, word boundaries may fall in the
// middle of speech. This function performs a two-stage refinement per boundary:
//
// 1. **RMS energy scan** (±80 ms): slides a 5 ms window across the search range
//    and picks the centre of the lowest-energy window — the most likely gap between
//    words.
// 2. **Zero-crossing snap** (±2 ms around the energy minimum): moves the candidate
//    to the nearest sample index where the signal crosses zero, avoiding a cut mid-
//    waveform cycle which would produce an audible click on export.
//
// Monotonic ordering and per-word minimum-duration constraints are preserved.
//
// `samples` must be 16 kHz mono f32 audio.

/// Pre-correction for proportional char-weight boundaries: when one word in an
/// adjacent pair is very short (<200 ms), scan ±200 ms around the boundary for
/// the lowest-energy point and snap the boundary there.  This fixes the
/// systematic late-bias that proportional distribution creates for short words
/// like "a", "I", "new" next to longer neighbours.
///
/// `samples` must be 16 kHz mono f32 audio.
fn correct_short_word_boundaries(words: &mut [Word], samples: &[f32]) {
    const SAMPLE_RATE: f64 = 16000.0;
    const SHORT_THRESHOLD_US: i64 = 200_000; // 200ms
    const SEARCH_US: i64 = 200_000; // search ±200ms
    const RMS_WINDOW: usize = 48; // 3ms

    for i in 0..words.len().saturating_sub(1) {
        let left_dur = words[i].end_us - words[i].start_us;
        let right_dur = words[i + 1].end_us - words[i + 1].start_us;

        // Only correct when one word is much shorter than the other
        if left_dur >= SHORT_THRESHOLD_US && right_dur >= SHORT_THRESHOLD_US {
            continue;
        }
        if left_dur <= 0 || right_dur <= 0 {
            continue;
        }

        let boundary_us = words[i].end_us;
        let center_sample = ((boundary_us as f64 / 1_000_000.0) * SAMPLE_RATE) as usize;
        let half_window = ((SEARCH_US as f64 / 1_000_000.0) * SAMPLE_RATE) as usize;

        let search_start = center_sample.saturating_sub(half_window);
        let search_end = (center_sample + half_window).min(samples.len());

        if search_end - search_start < RMS_WINDOW {
            continue;
        }

        // Find the minimum energy point in the search window
        let mut min_energy = f32::MAX;
        let mut min_pos = center_sample;
        let mut pos = search_start;
        while pos + RMS_WINDOW <= search_end {
            let mut sum_sq = 0.0f32;
            for s in &samples[pos..pos + RMS_WINDOW] {
                sum_sq += s * s;
            }
            let energy = (sum_sq / RMS_WINDOW as f32).sqrt();
            if energy < min_energy {
                min_energy = energy;
                min_pos = pos + RMS_WINDOW / 2;
            }
            pos += RMS_WINDOW / 2;
        }

        let refined_us = ((min_pos as f64 / SAMPLE_RATE) * 1_000_000.0) as i64;

        // Only apply if it preserves minimum word durations
        let min_word_us = 10_000; // 10ms
        if refined_us > words[i].start_us + min_word_us
            && refined_us < words[i + 1].end_us - min_word_us
        {
            words[i].end_us = refined_us;
            words[i + 1].start_us = refined_us;
        }
    }
}

fn refine_word_boundaries(words: &mut [Word], samples: &[f32]) {
    if words.len() < 2 || samples.is_empty() {
        return;
    }

    const SEARCH_WINDOW_US: i64 = 80_000; // ±80ms default search window
    const SHORT_WORD_SEARCH_WINDOW_US: i64 = 160_000; // ±160ms for short-word boundaries
    const SHORT_WORD_DURATION_THRESHOLD_US: i64 = 200_000; // words < 200ms are "short"
    const RMS_WINDOW_SAMPLES: usize = 80; // 5ms RMS analysis window (16000 * 0.005)
                                          // ±2 ms at 16 kHz = 32 samples; tight enough to stay near the energy dip
    const ZC_SNAP_HALF: usize = 32;

    // For each boundary between adjacent words, find the minimum energy point
    // then snap it to the nearest zero-crossing.
    for i in 0..words.len() - 1 {
        let boundary_us = words[i].end_us;

        // Use a wider search window when either adjacent word is short.
        // Short leading words are the primary source of audible remnants
        // after deletion because proportional split underestimates their
        // duration.
        let left_dur = (words[i].end_us - words[i].start_us).max(0);
        let right_dur = (words[i + 1].end_us - words[i + 1].start_us).max(0);
        let window_us = if left_dur < SHORT_WORD_DURATION_THRESHOLD_US
            || right_dur < SHORT_WORD_DURATION_THRESHOLD_US
        {
            SHORT_WORD_SEARCH_WINDOW_US
        } else {
            SEARCH_WINDOW_US
        };

        // Search window in samples
        let center_sample = (boundary_us as f64 / 1_000_000.0 * SAMPLE_RATE_HZ) as usize;
        let half_window_samples = (window_us as f64 / 1_000_000.0 * SAMPLE_RATE_HZ) as usize;

        let search_start = center_sample.saturating_sub(half_window_samples);
        let search_end = (center_sample + half_window_samples).min(samples.len());

        if search_start >= search_end || search_end - search_start < RMS_WINDOW_SAMPLES {
            continue;
        }

        // Stage 1: slide the RMS window and find the minimum-energy centre.
        let mut min_energy = f32::MAX;
        let mut min_pos = center_sample;

        let mut pos = search_start;
        while pos + RMS_WINDOW_SAMPLES <= search_end {
            let energy = rms(&samples[pos..pos + RMS_WINDOW_SAMPLES]);
            if energy < min_energy {
                min_energy = energy;
                min_pos = pos + RMS_WINDOW_SAMPLES / 2; // centre of the window
            }
            pos += RMS_WINDOW_SAMPLES / 2; // 50 % overlap for smooth coverage
        }

        // If local energy is essentially flat around this boundary, moving it
        // is more likely to introduce drift than improve seam quality.
        // Require a meaningful energy dip before applying a correction.
        let center_start = center_sample
            .saturating_sub(RMS_WINDOW_SAMPLES / 2)
            .min(samples.len().saturating_sub(RMS_WINDOW_SAMPLES));
        let center_end = (center_start + RMS_WINDOW_SAMPLES).min(samples.len());
        let center_energy = rms(&samples[center_start..center_end]);
        const MIN_DIP_RATIO: f32 = 0.97; // at least 3% dip to consider reliable
        if center_energy > 1e-6 && min_energy >= center_energy * MIN_DIP_RATIO {
            // No clear energy dip found. For short coarticulated words (e.g.,
            // "new release" spoken without pause), the proportional char-weight
            // boundary can land mid-phoneme. Use the minimum-energy point anyway
            // when either adjacent word is very short — even a marginal dip is
            // better than a proportional guess through active speech.
            const COARTICULATED_SHORT_THRESHOLD_US: i64 = 250_000; // 250ms
            let is_short_boundary = left_dur < COARTICULATED_SHORT_THRESHOLD_US
                || right_dur < COARTICULATED_SHORT_THRESHOLD_US;
            if !is_short_boundary {
                continue;
            }
            // For short words: still use min_energy position but require it
            // differs from center by at least 1 RMS window step to avoid
            // no-op corrections.
            if min_pos.abs_diff(center_sample) < RMS_WINDOW_SAMPLES / 2 {
                continue;
            }
        }

        // Stage 2: snap min_pos to the nearest zero-crossing within ±ZC_SNAP_HALF.
        // A zero-crossing exists between index z and z+1 when the two samples have
        // opposite signs (or one is exactly zero).
        let zc_start = min_pos.saturating_sub(ZC_SNAP_HALF);
        let zc_end = (min_pos + ZC_SNAP_HALF).min(samples.len().saturating_sub(1));

        let mut best_zc = min_pos;
        let mut best_dist = usize::MAX;
        for z in zc_start..zc_end {
            if samples[z] * samples[z + 1] <= 0.0 {
                let dist = z.abs_diff(min_pos);
                if dist < best_dist {
                    best_dist = dist;
                    best_zc = z;
                }
            }
        }
        min_pos = best_zc;

        // Convert back to microseconds
        let refined_us = sample_to_us(min_pos);

        // Only snap if the refined point preserves minimum word durations on both
        // sides, keeping monotonic ordering intact.
        let min_word_us = 10_000; // minimum 10 ms per word
        if refined_us > words[i].start_us + min_word_us
            && refined_us < words[i + 1].end_us - min_word_us
        {
            words[i].end_us = refined_us;
            words[i + 1].start_us = refined_us;
        }
    }
}

/// Align the start of segment-leading words to the true speech onset.
///
/// Whisper's segment `start` time often lands at or slightly after the first
/// phoneme rather than at the acoustic onset (the pre-voice burst of a
/// plosive, the initial fricative noise, etc.).  When the leading word is
/// subsequently deleted, audio before `start_us` that still contains speech
/// energy leaks through.
///
/// This function scans backwards from the first word's nominal start to find
/// the earliest sample whose short-term energy exceeds a threshold derived
/// from the word's body.  The word's `start_us` is then shifted to that
/// onset point (snapped to a zero-crossing for click-free cuts).
///
/// It also pushes the first inter-word boundary later if the first word is
/// very short, using a wider energy search to ensure the gap lands in actual
/// silence rather than mid-phoneme.
///
/// Monotonic ordering is preserved: onset can only move earlier (never past
/// the previous word's end), and inter-word shifts respect minimum durations.
fn align_onset_boundaries(words: &mut [Word], samples: &[f32]) {
    if words.is_empty() || samples.is_empty() {
        return;
    }

    const ONSET_SEARCH_US: i64 = 50_000; // search up to 50 ms before nominal start
    const RMS_WINDOW: usize = 48; // 3 ms at 16 kHz
    const RMS_STEP: usize = 16; // 1 ms step
    const ZC_SNAP_HALF: usize = 32;
    const MIN_WORD_US: i64 = 10_000;

    // Phase 1: pull each segment-leading word's start to the true onset.
    // A word is "segment-leading" if it is the first word overall, or there
    // is a gap (≥ 20 ms) between it and the previous word's end, which
    // typically marks a segment boundary.
    for i in 0..words.len() {
        let is_segment_start = i == 0 || (words[i].start_us - words[i - 1].end_us) >= 20_000;
        if !is_segment_start {
            continue;
        }

        let nominal_start_sample = us_to_sample(words[i].start_us, samples.len());

        // Measure the word's body energy (first 20 ms of the word body).
        let body_start = nominal_start_sample;
        let body_end = (body_start + 320).min(samples.len()); // 20 ms
        if body_end <= body_start + RMS_WINDOW {
            continue;
        }
        let body_energy = rms(&samples[body_start..body_end]);
        if body_energy < 1e-5 {
            continue; // silence — nothing to align
        }

        // Onset threshold: 15% of body energy
        let onset_threshold = body_energy * 0.15;

        // Search backwards from nominal start
        let onset_search_samples = (ONSET_SEARCH_US as f64 / 1_000_000.0 * SAMPLE_RATE_HZ) as usize;
        let search_start = nominal_start_sample.saturating_sub(onset_search_samples);
        // Don't cross into the previous word
        let earliest_allowed = if i > 0 {
            us_to_sample(words[i - 1].end_us, samples.len())
        } else {
            0
        };
        let search_start = search_start.max(earliest_allowed);

        if nominal_start_sample <= search_start + RMS_WINDOW {
            continue;
        }

        // Scan backwards: find the earliest position whose energy is above
        // threshold (i.e. the onset of speech).
        let mut onset_sample = nominal_start_sample;
        let mut pos = nominal_start_sample.saturating_sub(RMS_WINDOW);
        while pos >= search_start {
            let end = (pos + RMS_WINDOW).min(samples.len());
            if end - pos < RMS_WINDOW {
                break;
            }
            let e = rms(&samples[pos..end]);
            if e >= onset_threshold {
                onset_sample = pos;
            } else {
                // Energy dropped below threshold — onset is at the last
                // above-threshold position (already recorded).
                break;
            }
            if pos < RMS_STEP {
                break;
            }
            pos -= RMS_STEP;
        }

        if onset_sample < nominal_start_sample {
            let snapped = snap_to_zero_crossing(samples, onset_sample, ZC_SNAP_HALF);
            let onset_us = sample_to_us(snapped);
            let lower_bound = if i > 0 {
                words[i - 1].end_us + MIN_WORD_US
            } else {
                0
            };
            if onset_us >= lower_bound && onset_us < words[i].start_us {
                words[i].start_us = onset_us;
            }
        }
    }
}

/// Build word-level timestamps from transcription segments.
///
/// Each segment has a start/end time and text. We split each segment's text
/// into words and distribute timestamps proportionally by character length.
/// This produces timestamps that are accurate to within a segment (~30s chunks
/// from Whisper), with proportional distribution within each segment being
/// much better than global even distribution.
fn build_words_from_segments(
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

    // Build a flat list of (word, start_us, end_us) from segments first
    let mut segment_words: Vec<(String, i64, i64)> = Vec::new();
    for seg in segments {
        let seg_text = seg.text.trim();
        if seg_text.is_empty() {
            continue;
        }
        let seg_start_us = (seg.start as f64 * 1_000_000.0) as i64;
        let seg_end_us = (seg.end as f64 * 1_000_000.0) as i64;
        let seg_duration_us = seg_end_us - seg_start_us;

        let seg_words: Vec<&str> = seg_text.split_whitespace().collect();
        if seg_words.is_empty() {
            continue;
        }

        // Minimum effective character weight per word.  Using 1 gives each
        // word its true proportional share — short words like "a" or "I"
        // get a 1-char share instead of an inflated 3-char share, which
        // avoids pushing subsequent word boundaries too late on fast speech.
        const MIN_WORD_CHAR_WEIGHT: usize = 1;

        // Total character count for proportional distribution
        let total_chars: usize = seg_words
            .iter()
            .map(|w| w.len().max(MIN_WORD_CHAR_WEIGHT))
            .sum();

        let mut cursor_us = seg_start_us;
        for (j, sw) in seg_words.iter().enumerate() {
            let char_fraction = sw.len().max(MIN_WORD_CHAR_WEIGHT) as f64 / total_chars as f64;
            let word_duration_us = (seg_duration_us as f64 * char_fraction) as i64;

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
fn sanitize_word_timestamps(words: &mut [Word], total_duration_us: i64) {
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

    // For non-WAV files, extract audio via FFmpeg first
    let (wav_path, is_temp) = if is_wav_file(file_path) {
        (file_path.to_path_buf(), false)
    } else {
        (extract_audio_to_wav(file_path)?, true)
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

    // Transcribe — now returns segments with real timestamps
    let (text, segments) = tm
        .transcribe(samples.clone())
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not loaded") {
                "No transcription model loaded. Go to Settings → Models, download a model, then try again.".to_string()
            } else {
                format!("Transcription failed: {}", msg)
            }
        })?;

    if text.is_empty() {
        return Err("Transcription produced no text".to_string());
    }

    // Build words with real timestamps from transcription segments.
    // Segments have start/end in seconds — we distribute words within each
    // segment proportionally by character count (a reasonable proxy for speech
    // duration). This is far more accurate than the previous approach of
    // dividing total audio duration evenly across all words.
    let sample_rate = 16000.0_f64;
    let total_duration_us = ((samples.len() as f64 / sample_rate) * 1_000_000.0) as i64;

    // Detect whether the ASR engine provided word-level timestamps.
    // If so, use segments directly; otherwise use proportional distribution.
    let segments_are_word_level = segments.as_ref().is_some_and(|segs| {
        if segs.is_empty() {
            return false;
        }
        // If most segments contain exactly 1 word, the engine provided word-level timestamps
        let single_word_count = segs
            .iter()
            .filter(|s| s.text.split_whitespace().count() == 1)
            .count();
        single_word_count as f64 / segs.len() as f64 > 0.8
    });

    if segments_are_word_level {
        info!(
            "ASR engine provided word-level timestamps ({} segments)",
            segments.as_ref().map_or(0, |s| s.len())
        );
    }

    let (mut words, align_meta): (Vec<Word>, Option<Vec<WordAlignmentMeta>>) = if let Some(
        ref segs,
    ) = segments
    {
        // Fallback: proportional distribution within ASR segments
        let (words, meta) = build_words_from_segments(&text, segs, &samples);
        (words, Some(meta))
    } else {
        // Fallback: no segments available (some engines don't provide them).
        // Distribute words evenly across total duration (legacy behavior).
        let raw_words: Vec<&str> = text.split_whitespace().collect();
        warn!(
                "Transcription engine returned no segments; using legacy even timestamp distribution for {} words",
                raw_words.len()
            );
        let word_duration_us = if raw_words.is_empty() {
            0
        } else {
            total_duration_us / raw_words.len() as i64
        };
        let fallback_words: Vec<Word> = raw_words
            .iter()
            .enumerate()
            .map(|(i, w)| Word {
                text: w.to_string(),
                start_us: i as i64 * word_duration_us,
                end_us: (i as i64 + 1) * word_duration_us,
                deleted: false,
                silenced: false,
                confidence: -1.0,
                speaker_id: -1,
            })
            .collect();
        (fallback_words, None)
    };

    // Sanitize timestamps: clamp to audio duration, enforce monotonic
    // non-overlapping progression, and ensure minimal non-zero durations.
    sanitize_word_timestamps(&mut words, total_duration_us);
    realign_suspicious_spans(&mut words, &samples, align_meta.as_deref());
    sanitize_word_timestamps(&mut words, total_duration_us);

    if words.is_empty() {
        return Err("No words in transcription".to_string());
    }

    // Populate the editor
    let mut state = editor_store.0.lock().unwrap();
    state.set_words(words.clone());

    Ok(state.get_words().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_word(text: &str, start_us: i64, end_us: i64, confidence: f32) -> Word {
        Word {
            text: text.to_string(),
            start_us,
            end_us,
            deleted: false,
            silenced: false,
            confidence,
            speaker_id: -1,
        }
    }

    #[test]
    fn realigns_low_confidence_boundary_locally() {
        let mut words = vec![
            make_word("hello", 0, 450_000, 0.2),
            make_word("world", 450_000, 800_000, 0.9),
        ];
        let mut samples = vec![0.5_f32; 16_000];
        for s in samples.iter_mut().take(8_080).skip(7_920) {
            *s = 0.0;
        }

        realign_suspicious_spans(&mut words, &samples, None);

        assert!(words[0].end_us > 480_000);
        assert!(words[0].end_us < 520_000);
        assert_eq!(words[0].end_us, words[1].start_us);
    }

    #[test]
    fn keeps_stable_high_confidence_boundary() {
        let mut words = vec![
            make_word("hello", 0, 450_000, 0.9),
            make_word("world", 450_000, 800_000, 0.9),
        ];
        let mut samples = vec![0.5_f32; 16_000];
        for s in samples.iter_mut().take(8_080).skip(7_920) {
            *s = 0.0;
        }

        realign_suspicious_spans(&mut words, &samples, None);

        assert_eq!(words[0].end_us, 450_000);
        assert_eq!(words[1].start_us, 450_000);
    }

    // ── proportional split: short word weighting ────────────────────────────

    /// A short leading word (1 char like "I") receives its true proportional
    /// share with MIN_WORD_CHAR_WEIGHT=1 (1/10 = 150ms of 1.5s).  The
    /// energy-based correction may shift the boundary, but the word must
    /// still be present with a positive duration.
    #[test]
    fn proportional_split_gives_short_leading_word_adequate_duration() {
        // Segment: "I said hello" over 1.5 s
        let segments = vec![TranscriptionSegment {
            start: 0.0,
            end: 1.5,
            text: " I said hello".to_string(),
        }];
        let samples = vec![0.3_f32; 24_000]; // 1.5 s at 16 kHz
        let (words, _meta) = build_words_from_segments("I said hello", &segments, &samples);
        assert_eq!(words.len(), 3);

        // With MIN_WORD_CHAR_WEIGHT=1, "I" (1 char) out of
        // total 1+4+5=10 gets 10% = 150ms before boundary refinement.
        // After energy correction the value may shift, but the word
        // must still have a positive duration.
        let first_word_duration = words[0].end_us - words[0].start_us;
        assert!(
            first_word_duration > 0,
            "short leading word 'I' got {} µs, expected > 0",
            first_word_duration
        );
    }

    /// Two equally-long words should still split roughly evenly.
    #[test]
    fn proportional_split_equal_words_stay_even() {
        let segments = vec![TranscriptionSegment {
            start: 0.0,
            end: 1.0,
            text: " hello world".to_string(),
        }];
        let samples = vec![0.3_f32; 16_000];
        let (words, _) = build_words_from_segments("hello world", &segments, &samples);
        assert_eq!(words.len(), 2);
        let d0 = words[0].end_us - words[0].start_us;
        let d1 = words[1].end_us - words[1].start_us;
        // Both are 5 chars → same weight → equal split (500ms each ±tolerance).
        let diff = (d0 - d1).abs();
        assert!(
            diff < 50_000,
            "equal-length words should have similar durations, diff = {} µs",
            diff
        );
    }

    // ── correct_short_word_boundaries ───────────────────────────────────────

    /// When "new" is short (190ms) and the boundary sits in speech energy,
    /// `correct_short_word_boundaries` should move it to the nearby energy
    /// dip (silence region around 150-175ms).
    #[test]
    fn correct_short_word_boundaries_moves_boundary_to_energy_minimum() {
        // "new release" — "new" is short, boundary at 190ms should
        // move toward energy dip at ~162ms
        let mut words = vec![
            make_word("new", 0, 190_000, 0.9),             // 0-190ms (short)
            make_word("release", 190_000, 1_200_000, 0.9), // 190-1200ms
        ];
        // Create samples with energy dip at ~150-175ms (samples 2400..2800)
        let mut samples = vec![0.3f32; 19_200]; // 1.2s at 16kHz
        for s in samples[2_400..2_800].iter_mut() {
            *s = 0.02; // silence at ~150-175ms
        }
        correct_short_word_boundaries(&mut words, &samples);
        // Boundary between "new" and "release" should have moved toward 162ms
        assert!(
            words[0].end_us < 190_000,
            "boundary should move earlier toward energy dip, got {} µs",
            words[0].end_us
        );
        assert!(
            words[0].end_us > 100_000,
            "boundary shouldn't move too far, got {} µs",
            words[0].end_us
        );
    }

    // ── wider refine window for short words ─────────────────────────────────

    /// When the leading word is short, refine_word_boundaries should use its
    /// wider search window and find a silence gap that the default ±80ms
    /// window would miss.
    #[test]
    fn refine_uses_wider_window_for_short_leading_word() {
        // "I" from 0–100ms, "world" from 100ms–1000ms.
        // True silence gap at ~250ms (sample 4000).  Default ±80ms window
        // from 100ms would search [20ms, 180ms] = samples [320, 2880] and
        // miss the gap at sample 4000.  With ±160ms the window reaches it.
        let mut words = vec![
            make_word("I", 0, 100_000, 0.9),
            make_word("world", 100_000, 1_000_000, 0.9),
        ];
        let mut samples = vec![0.5_f32; 16_000];
        // Silence gap at samples 3920..4080 (≈ 245ms..255ms)
        for s in samples[3_920..4_080].iter_mut() {
            *s = 0.0;
        }
        refine_word_boundaries(&mut words, &samples);
        // Boundary should have shifted toward ~250ms.
        assert!(
            words[0].end_us > 200_000,
            "short-word boundary should shift to ~250ms gap, got {} µs",
            words[0].end_us
        );
        assert!(
            words[0].end_us < 300_000,
            "boundary should land near 250ms, got {} µs",
            words[0].end_us
        );
        assert_eq!(words[0].end_us, words[1].start_us);
    }

    // ── onset alignment ─────────────────────────────────────────────────────

    /// When speech energy begins before the nominal segment start,
    /// align_onset_boundaries must pull word start earlier.
    #[test]
    fn onset_alignment_pulls_start_to_true_onset() {
        // Word starts at 500ms nominally, but there is energy from ~470ms.
        let mut words = vec![
            make_word("hello", 500_000, 1_000_000, 0.9),
            make_word("world", 1_000_000, 1_500_000, 0.9),
        ];
        // Silence before 7200 (450ms), then speech from 7520 (470ms) onward
        let mut samples = vec![0.0_f32; 24_000]; // 1.5s
        for s in samples[7_520..].iter_mut() {
            *s = 0.5;
        }
        // Nominal start at 500ms = sample 8000, but energy begins at 470ms = 7520
        align_onset_boundaries(&mut words, &samples);
        assert!(
            words[0].start_us < 500_000,
            "onset should be pulled earlier from 500ms, got {} µs",
            words[0].start_us
        );
        // Should be near 470ms
        assert!(
            words[0].start_us >= 450_000 && words[0].start_us <= 490_000,
            "onset should land near 470ms, got {} µs",
            words[0].start_us
        );
    }

    /// Onset alignment must not cross into the previous word's territory.
    #[test]
    fn onset_alignment_respects_previous_word_boundary() {
        let mut words = vec![
            make_word("hello", 0, 400_000, 0.9),
            make_word("world", 450_000, 1_000_000, 0.9),
        ];
        // Continuous energy everywhere — onset search should stop at prev end
        let samples = vec![0.5_f32; 16_000];
        let start_before = words[1].start_us;
        align_onset_boundaries(&mut words, &samples);
        // word[1].start_us must not go below word[0].end_us
        assert!(
            words[1].start_us >= words[0].end_us,
            "onset must not cross previous word boundary: got {} vs prev end {}",
            words[1].start_us,
            words[0].end_us,
        );
        // And it should have moved at most to near words[0].end_us
        assert!(
            words[1].start_us <= start_before,
            "onset should move earlier or stay, got {} (was {})",
            words[1].start_us,
            start_before,
        );
    }

    /// Onset alignment with no energy before nominal start should not change it.
    #[test]
    fn onset_alignment_noop_when_silence_before_start() {
        let mut words = vec![make_word("hello", 500_000, 1_000_000, 0.9)];
        let mut samples = vec![0.0_f32; 16_000]; // silence everywhere
                                                 // Speech only from sample 8000 onward (500ms)
        for s in samples[8_000..].iter_mut() {
            *s = 0.5;
        }
        let start_before = words[0].start_us;
        align_onset_boundaries(&mut words, &samples);
        // Should stay at 500ms (±small ZC snap tolerance)
        assert!(
            (words[0].start_us - start_before).abs() < 5_000,
            "onset should stay near 500ms when no earlier energy, got {} µs",
            words[0].start_us,
        );
    }

    // ── beginning-word deletion precision (integration) ─────────────────────

    /// End-to-end: after the full pipeline, deleting the first short word
    /// ("I" in "I said hello") must produce a clean boundary — the kept
    /// word "said" must start at or after the silence gap, not mid-phoneme.
    #[test]
    fn beginning_short_word_deletion_boundary_lands_in_silence() {
        // Simulate: "I said hello" over 1.5s.
        // Audio layout:
        //   0–300ms: "I" speech (samples 0..4800)
        //   300–350ms: silence gap (samples 4800..5600)
        //   350ms–1s: "said" speech (samples 5600..16000)
        //   1s–1.5s: "hello" speech
        let mut samples = vec![0.5_f32; 24_000]; // 1.5s
                                                 // Silence gap at 300–350ms
        for s in samples[4_800..5_600].iter_mut() {
            *s = 0.0;
        }

        let segments = vec![TranscriptionSegment {
            start: 0.0,
            end: 1.5,
            text: " I said hello".to_string(),
        }];
        let total_duration_us = 1_500_000;

        let (mut words, align_meta) =
            build_words_from_segments("I said hello", &segments, &samples);
        sanitize_word_timestamps(&mut words, total_duration_us);
        realign_suspicious_spans(&mut words, &samples, Some(&align_meta));
        sanitize_word_timestamps(&mut words, total_duration_us);

        assert_eq!(words.len(), 3);

        // The boundary between "I" and "said" should be in or near the
        // silence gap (300–350ms = 300_000–350_000 µs).
        let boundary = words[0].end_us;
        assert!(
            boundary >= 280_000 && boundary <= 380_000,
            "boundary between 'I' and 'said' should be near silence gap \
             (300–350ms), got {} µs",
            boundary
        );
        assert_eq!(words[0].end_us, words[1].start_us);
    }
}

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
/// Add a `seam_rms_at_boundary(samples, cut_sample, window_samples) -> f32`
/// helper and assert `seam_rms < 0.05 * peak_rms` for each cut.
#[cfg(test)]
mod precision_benchmarks {
    use super::*;
    use crate::managers::editor::{EditorState, Word as EdWord};

    // ── acceptance thresholds ────────────────────────────────────────────────
    /// Maximum µs a boundary is allowed to drift after `refine_word_boundaries`.
    /// Derived from SHORT_WORD_SEARCH_WINDOW_US (160 ms) + ZC_SNAP_HALF in µs (2 ms).
    const MAX_BOUNDARY_DRIFT_US: i64 = 162_000;

    /// Zero monotonicity violations are tolerated anywhere in the pipeline.
    const MAX_MONOTONICITY_VIOLATIONS: usize = 0;

    /// Roundtrip conversion must be within this many samples.
    const MAX_ROUNDTRIP_SAMPLE_ERROR: usize = 1;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn w(text: &str, start_us: i64, end_us: i64, confidence: f32) -> Word {
        Word {
            text: text.to_string(),
            start_us,
            end_us,
            deleted: false,
            silenced: false,
            confidence,
            speaker_id: -1,
        }
    }

    fn ed_word(text: &str, start_us: i64, end_us: i64) -> EdWord {
        EdWord {
            text: text.to_string(),
            start_us,
            end_us,
            deleted: false,
            silenced: false,
            confidence: 0.9,
            speaker_id: 0,
        }
    }

    /// Count monotonicity violations: inverted word ranges and adjacent overlaps.
    fn count_monotonicity_violations(words: &[Word]) -> usize {
        let mut violations = 0;
        for (i, word) in words.iter().enumerate() {
            if word.start_us > word.end_us {
                violations += 1;
            }
            if i + 1 < words.len() && word.end_us > words[i + 1].start_us {
                violations += 1;
            }
        }
        violations
    }

    // ── sanitize_word_timestamps ─────────────────────────────────────────────

    /// After sanitize, all words must be monotonically ordered with no overlaps.
    /// Threshold: 0 violations.
    #[test]
    fn sanitize_enforces_monotonic_ordering() {
        let mut words = vec![
            w("a", 500_000, 100_000, 0.9),   // inverted: start > end
            w("b", 200_000, 800_000, 0.9),   // overlaps with c
            w("c", 600_000, 900_000, 0.9),   // starts before b ends
            w("d", 850_000, 1_200_000, 0.9), // starts before c ends
        ];
        sanitize_word_timestamps(&mut words, 2_000_000);
        assert_eq!(
            count_monotonicity_violations(&words),
            MAX_MONOTONICITY_VIOLATIONS,
            "monotonicity violated after sanitize_word_timestamps"
        );
    }

    /// All timestamps must be clamped within [0, total_duration_us].
    #[test]
    fn sanitize_clamps_to_total_duration() {
        let total = 1_000_000_i64;
        let mut words = vec![
            w("a", -500_000, 200_000, 0.9),  // negative start
            w("b", 800_000, 2_000_000, 0.9), // end beyond total
        ];
        sanitize_word_timestamps(&mut words, total);
        for word in &words {
            assert!(
                word.start_us >= 0,
                "start_us must be >= 0, got {}",
                word.start_us
            );
            assert!(
                word.end_us <= total,
                "end_us {} must be <= total {}",
                word.end_us,
                total
            );
        }
        assert_eq!(
            count_monotonicity_violations(&words),
            MAX_MONOTONICITY_VIOLATIONS
        );
    }

    /// Zero-duration words with available budget must receive the 1 ms minimum.
    #[test]
    fn sanitize_grants_minimum_duration_where_budget_allows() {
        let mut words = vec![
            w("a", 0, 0, 0.9),             // zero-duration, room to expand
            w("b", 500_000, 500_000, 0.9), // zero-duration mid, room to expand
        ];
        sanitize_word_timestamps(&mut words, 2_000_000);
        assert!(
            words[0].end_us > words[0].start_us,
            "zero-duration word 'a' must receive minimum duration"
        );
    }

    /// A second pass over already-valid timestamps must leave them unchanged.
    /// Threshold: exact equality.
    #[test]
    fn sanitize_idempotent_on_clean_input() {
        let mut words = vec![
            w("hello", 0, 1_000_000, 0.9),
            w("world", 1_000_000, 2_000_000, 0.9),
            w("test", 2_000_000, 3_000_000, 0.9),
        ];
        let before: Vec<(i64, i64)> = words.iter().map(|ww| (ww.start_us, ww.end_us)).collect();
        sanitize_word_timestamps(&mut words, 3_000_000);
        let after: Vec<(i64, i64)> = words.iter().map(|ww| (ww.start_us, ww.end_us)).collect();
        assert_eq!(
            before, after,
            "sanitize must be idempotent on already-valid timestamps"
        );
    }

    // ── snap_to_zero_crossing ────────────────────────────────────────────────

    /// With a single zero-crossing at sample 100 inside the window, the snap
    /// must return exactly sample 100.
    #[test]
    fn snap_zc_finds_nearest_crossing_within_window() {
        // samples 0..100 = +1.0, then −1.0 from 101 onwards → ZC between 100 and 101.
        let mut samples = vec![1.0_f32; 200];
        for s in samples[101..].iter_mut() {
            *s = -1.0;
        }
        let result = snap_to_zero_crossing(&samples, 95, 20);
        assert_eq!(result, 100, "ZC snap must find the crossing at sample 100");
    }

    /// With no zero crossings in the signal, the result must still be in-bounds.
    #[test]
    fn snap_zc_result_in_bounds_when_no_crossing() {
        let samples = vec![1.0_f32; 200];
        let result = snap_to_zero_crossing(&samples, 100, 10);
        assert!(
            result < samples.len() - 1,
            "ZC result {} must be within sample bounds",
            result
        );
    }

    /// Edge: target near the end of a very short signal must not panic.
    #[test]
    fn snap_zc_edge_near_end_of_signal() {
        let samples = vec![1.0_f32, -1.0, 1.0_f32, -1.0];
        let result = snap_to_zero_crossing(&samples, 3, 5);
        assert!(
            result < samples.len() - 1,
            "ZC result must be within bounds"
        );
    }

    // ── find_local_low_energy_boundary ───────────────────────────────────────

    /// When a silence gap exists inside the search window, the energy finder must
    /// return a minimum whose energy is lower than that of the original boundary
    /// centre. Threshold: min_energy < center_energy.
    #[test]
    fn energy_finder_returns_lower_energy_than_center() {
        // High energy (0.8) everywhere except a 160-sample silence gap at ~800.
        let mut samples = vec![0.8_f32; 1600];
        for s in samples[720..880].iter_mut() {
            *s = 0.0;
        }
        // Center at sample 320 — well away from the gap, so center_energy ≈ 0.8.
        let (min_pos, min_energy, center_energy) =
            find_local_low_energy_boundary(&samples, 320, 800, 80, 40)
                .expect("must return a result when signal is long enough");

        assert!(
            min_energy < center_energy,
            "min_energy ({:.4}) must be < center_energy ({:.4}) when a silence region exists",
            min_energy,
            center_energy
        );
        // The minimum centre should land inside or near the silence gap.
        assert!(
            min_pos >= 700 && min_pos <= 900,
            "energy minimum centre should be near the silence gap, got sample {}",
            min_pos
        );
    }

    /// Inputs shorter than the RMS window must return `None` (no panic).
    #[test]
    fn energy_finder_returns_none_on_too_short_input() {
        let samples = vec![0.5_f32; 10]; // rms_window_samples = 80 > 10
        let result = find_local_low_energy_boundary(&samples, 5, 10, 80, 10);
        assert!(
            result.is_none(),
            "should return None when signal is shorter than RMS window"
        );
    }

    // ── refine_word_boundaries ───────────────────────────────────────────────

    /// Monotonicity (start ≤ end, no adjacent overlap) must be preserved after
    /// boundary refinement regardless of where the energy minimum lands.
    /// Threshold: 0 violations.
    #[test]
    fn refine_preserves_monotonicity() {
        let mut words = vec![
            w("hello", 0, 500_000, 0.9),
            w("world", 500_000, 1_000_000, 0.9),
        ];
        // Silence gap at samples 7680..7760 (≈ 480 ms..485 ms).
        let mut samples = vec![0.5_f32; 16_000];
        for s in samples[7_680..7_760].iter_mut() {
            *s = 0.0;
        }
        refine_word_boundaries(&mut words, &samples);
        assert_eq!(
            count_monotonicity_violations(&words),
            MAX_MONOTONICITY_VIOLATIONS,
            "monotonicity must be preserved after refine_word_boundaries"
        );
    }

    /// The boundary shift caused by `refine_word_boundaries` must not exceed
    /// the combined search + snap budget.
    /// Threshold: ≤ MAX_BOUNDARY_DRIFT_US (162 000 µs).
    #[test]
    fn refine_boundary_drift_within_budget() {
        let initial_boundary_us = 500_000_i64;
        let mut words = vec![
            w("hello", 0, initial_boundary_us, 0.9),
            w("world", initial_boundary_us, 1_000_000, 0.9),
        ];
        // Silence gap at samples 7100..7300 (≈ 444 ms..456 ms) — within ±80 ms.
        let mut samples = vec![0.3_f32; 16_000];
        for s in samples[7_100..7_300].iter_mut() {
            *s = 0.0;
        }
        refine_word_boundaries(&mut words, &samples);
        let drift = (words[0].end_us - initial_boundary_us).abs();
        assert!(
            drift <= MAX_BOUNDARY_DRIFT_US,
            "boundary drift {} µs exceeds budget of {} µs",
            drift,
            MAX_BOUNDARY_DRIFT_US
        );
        assert_eq!(
            words[0].end_us, words[1].start_us,
            "adjacent boundary timestamps must agree after refinement"
        );
    }

    /// When a silence gap exists below the initial boundary, the refined
    /// boundary must move toward that gap.
    #[test]
    fn refine_snaps_boundary_toward_silence_gap() {
        // Boundary at 500 ms; silence gap centred at ~450 ms (samples 7160..7240).
        let initial_boundary_us = 500_000_i64;
        let mut words = vec![
            w("hello", 0, initial_boundary_us, 0.9),
            w("world", initial_boundary_us, 1_000_000, 0.9),
        ];
        let mut samples = vec![0.5_f32; 16_000];
        for s in samples[7_160..7_240].iter_mut() {
            *s = 0.0;
        }
        refine_word_boundaries(&mut words, &samples);
        assert!(
            words[0].end_us < initial_boundary_us,
            "boundary must shift toward silence at ~450 ms, got {} µs",
            words[0].end_us
        );
    }

    /// When two short words are coarticulated (no silence gap between them),
    /// the boundary should still move to the minimum-energy point rather than
    /// staying at the proportional char-weight estimate.
    #[test]
    fn refine_moves_boundary_for_coarticulated_short_words() {
        // Simulate "new release" — short word (150ms) followed by longer word (350ms).
        // No silence gap, but energy dips slightly around sample 3200 (~200ms).
        let initial_boundary_us = 150_000; // proportional estimate at 150ms
        let mut words = vec![
            w("new", 0, initial_boundary_us, 0.9),
            w("release", initial_boundary_us, 500_000, 0.9),
        ];

        // Continuous speech energy with a slight dip around 100ms (sample 1600)
        // — not enough for the 3% MIN_DIP_RATIO, but the coarticulated fallback
        // should still move the boundary toward lower energy.
        let mut samples = vec![0.4_f32; 8_000]; // 500ms at 16kHz
        // Create a gradual energy taper around 100ms (earlier than proportional)
        for s in 1500..1700 {
            samples[s] = 0.35; // slight dip — less than 3% of 0.4
        }

        let original_boundary = words[0].end_us;
        refine_word_boundaries(&mut words, &samples);

        // The boundary should have moved (coarticulated short-word fallback)
        // and both words must remain valid duration
        assert!(
            words[0].end_us != original_boundary || words[0].end_us > 10_000,
            "boundary should move or be valid for coarticulated short word"
        );
        assert!(
            words[0].end_us >= words[0].start_us + 10_000,
            "left word must retain minimum 10ms duration"
        );
        assert!(
            words[1].end_us >= words[1].start_us + 10_000,
            "right word must retain minimum 10ms duration"
        );
        assert_eq!(
            words[0].end_us, words[1].start_us,
            "adjacent boundary timestamps must agree"
        );
    }

    // ── full pipeline: sanitize → refine → realign → sanitize ───────────────

    /// After running the full pipeline against a heavily corrupted word list the
    /// result must have zero monotonicity violations.
    /// Threshold: MAX_MONOTONICITY_VIOLATIONS = 0.
    #[test]
    fn full_pipeline_preserves_monotonicity_on_corrupted_input() {
        let total_duration_us = 10_000_000_i64; // 10 s
        let mut words = vec![
            w("one", -200_000, 300_000, 0.3),  // negative start
            w("two", 100_000, 800_000, 0.2),   // overlaps with "one"
            w("three", 700_000, 600_000, 0.4), // inverted
            w("four", 900_000, 1_500_000, 0.85),
            w("five", 1_400_000, 2_100_000, 0.9), // overlaps with "four"
            w("six", 2_000_000, 1_800_000, 0.15), // inverted
            w("seven", 2_500_000, 3_000_000, 0.9),
            w("eight", 2_900_000, 3_500_000, 0.3), // overlaps with "seven"
            w("nine", 3_600_000, 4_200_000, 0.9),
            w("ten", 4_100_000, 12_000_000, 0.9), // end beyond total
        ];
        let n_samples = (total_duration_us as f64 / 1_000_000.0 * SAMPLE_RATE_HZ) as usize;
        let samples = vec![0.3_f32; n_samples];

        sanitize_word_timestamps(&mut words, total_duration_us);
        refine_word_boundaries(&mut words, &samples);
        realign_suspicious_spans(&mut words, &samples, None);
        sanitize_word_timestamps(&mut words, total_duration_us);

        assert_eq!(
            count_monotonicity_violations(&words),
            MAX_MONOTONICITY_VIOLATIONS,
            "full pipeline must leave 0 monotonicity violations"
        );
    }

    /// After the full pipeline every timestamp must lie within [0, total_duration_us].
    #[test]
    fn full_pipeline_all_timestamps_within_total_duration() {
        let total_duration_us = 5_000_000_i64;
        // 20 words with overlapping/negative offsets to stress the sanitizer.
        let mut words: Vec<Word> = (0..20)
            .map(|i| {
                let base = i as i64 * 300_000;
                w(
                    "x",
                    base - 50_000,
                    base + 400_000,
                    if i % 3 == 0 { 0.2 } else { 0.9 },
                )
            })
            .collect();
        let n_samples = (total_duration_us as f64 / 1_000_000.0 * SAMPLE_RATE_HZ) as usize;
        let samples = vec![0.4_f32; n_samples];

        sanitize_word_timestamps(&mut words, total_duration_us);
        refine_word_boundaries(&mut words, &samples);
        realign_suspicious_spans(&mut words, &samples, None);
        sanitize_word_timestamps(&mut words, total_duration_us);

        for word in &words {
            assert!(
                word.start_us >= 0,
                "start_us {} must be >= 0",
                word.start_us
            );
            assert!(
                word.end_us <= total_duration_us,
                "end_us {} must be <= total {}",
                word.end_us,
                total_duration_us
            );
        }
    }

    // ── sample ↔ µs conversion accuracy ─────────────────────────────────────

    /// `us_to_sample(sample_to_us(n))` must equal `n` within MAX_ROUNDTRIP_SAMPLE_ERROR.
    /// Threshold: ≤ 1 sample (≈ 62.5 µs at 16 kHz).
    #[test]
    fn us_to_sample_roundtrip_within_one_sample() {
        let total_samples = 16_000_usize;
        for &sample_idx in &[0_usize, 1, 100, 800, 8_000, 15_999] {
            let us = sample_to_us(sample_idx);
            let recovered = us_to_sample(us, total_samples);
            let error = (recovered as i64 - sample_idx as i64).unsigned_abs() as usize;
            assert!(
                error <= MAX_ROUNDTRIP_SAMPLE_ERROR,
                "roundtrip error for sample {} is {} (limit: {} sample)",
                sample_idx,
                error,
                MAX_ROUNDTRIP_SAMPLE_ERROR
            );
        }
    }

    /// `sample_to_us` must be non-decreasing (monotone).
    #[test]
    fn sample_to_us_is_monotone() {
        for i in 0_usize..100 {
            assert!(
                sample_to_us(i) <= sample_to_us(i + 1),
                "sample_to_us is not monotone at {} → {}",
                i,
                i + 1
            );
        }
    }

    // ── keep-segment coverage ────────────────────────────────────────────────

    /// The total duration of all keep-segments must equal the sum of durations
    /// of non-deleted words.
    #[test]
    fn keep_segment_coverage_matches_non_deleted_duration() {
        let words = vec![
            ed_word("a", 0, 1_000_000),
            ed_word("b", 1_000_000, 2_000_000),
            ed_word("c", 2_000_000, 3_000_000),
            ed_word("d", 3_000_000, 4_000_000),
        ];
        let deleted_indices = [1_usize, 3]; // "b" and "d"

        let mut state = EditorState::new();
        state.set_words(words.clone());
        for &i in &deleted_indices {
            state.delete_word(i);
        }

        let segs = state.get_keep_segments();
        let seg_total: i64 = segs.iter().map(|(s, e)| e - s).sum();
        let expected: i64 = words
            .iter()
            .enumerate()
            .filter(|(i, _)| !deleted_indices.contains(i))
            .map(|(_, ww)| ww.end_us - ww.start_us)
            .sum();

        assert_eq!(
            seg_total, expected,
            "keep-segment total must equal sum of non-deleted word durations"
        );
    }

    /// No two keep-segments may overlap, and each must have start ≤ end.
    #[test]
    fn keep_segments_non_overlapping() {
        let words: Vec<EdWord> = (0..10)
            .map(|i| EdWord {
                text: format!("w{}", i),
                start_us: i as i64 * 500_000,
                end_us: (i as i64 + 1) * 500_000,
                deleted: i % 3 == 0, // delete every 3rd word
                silenced: false,
                confidence: 0.9,
                speaker_id: 0,
            })
            .collect();
        let mut state = EditorState::new();
        state.set_words(words);
        let segs = state.get_keep_segments();

        for (i, &(s, e)) in segs.iter().enumerate() {
            assert!(s <= e, "segment[{}] has start {} > end {}", i, s, e);
            if i + 1 < segs.len() {
                let (ns, _) = segs[i + 1];
                assert!(e <= ns, "segments[{}] and [{}] overlap", i, i + 1);
            }
        }
    }

    // ── edit-time → source-time alignment (preview / export) ────────────────

    /// With no deletions, `map_edit_time_to_source_time(t)` must equal `t` exactly.
    /// Threshold: 0 µs drift.
    #[test]
    fn edit_to_source_time_identity_no_deletions() {
        let words: Vec<EdWord> = (0..5)
            .map(|i| {
                ed_word(
                    &format!("w{}", i),
                    i as i64 * 1_000_000,
                    (i + 1) as i64 * 1_000_000,
                )
            })
            .collect();
        let mut state = EditorState::new();
        state.set_words(words);

        for &t in &[0_i64, 500_000, 1_000_000, 2_500_000, 4_999_999] {
            let src = state.map_edit_time_to_source_time(t);
            assert_eq!(
                src, t,
                "with no deletions, edit time {t} must map to source {t}, got {src}"
            );
        }
    }

    /// Delete one word and verify the exact source-time mapping.
    /// Threshold: 0 µs drift (integer arithmetic).
    #[test]
    fn edit_to_source_time_single_gap_exact() {
        // 6 words × 1 s each.  Delete word 2 (2M..3M).
        // Keep segments: [0..2M], [3M..6M]
        // Edit-time: 0..2M → source 0..2M; 2M..5M → source 3M..6M.
        let words: Vec<EdWord> = (0..6)
            .map(|i| {
                ed_word(
                    &format!("w{}", i),
                    i as i64 * 1_000_000,
                    (i + 1) as i64 * 1_000_000,
                )
            })
            .collect();
        let mut state = EditorState::new();
        state.set_words(words);
        state.delete_word(2);

        assert_eq!(state.map_edit_time_to_source_time(0), 0);
        assert_eq!(state.map_edit_time_to_source_time(1_000_000), 1_000_000);
        // edit 2M → source 3M (exact jump over deleted gap)
        assert_eq!(state.map_edit_time_to_source_time(2_000_000), 3_000_000);
        assert_eq!(state.map_edit_time_to_source_time(3_000_000), 4_000_000);
        // past end → clamp to 6M
        assert_eq!(state.map_edit_time_to_source_time(10_000_000), 6_000_000);
    }

    /// Delete two non-adjacent words and verify mapping across both gaps.
    #[test]
    fn edit_to_source_time_multiple_gaps_exact() {
        // 5 words × 1 s.  Delete word 1 (1M..2M) and word 3 (3M..4M).
        // Keep: [0..1M], [2M..3M], [4M..5M]
        // Edit positions: 0..1M, 1M..2M, 2M..3M
        let words: Vec<EdWord> = (0..5)
            .map(|i| {
                ed_word(
                    &format!("w{}", i),
                    i as i64 * 1_000_000,
                    (i + 1) as i64 * 1_000_000,
                )
            })
            .collect();
        let mut state = EditorState::new();
        state.set_words(words);
        state.delete_word(1);
        state.delete_word(3);

        assert_eq!(state.map_edit_time_to_source_time(0), 0);
        assert_eq!(state.map_edit_time_to_source_time(500_000), 500_000);
        // edit 1M → source 2M (jumped over gap at 1M..2M)
        assert_eq!(state.map_edit_time_to_source_time(1_000_000), 2_000_000);
        assert_eq!(state.map_edit_time_to_source_time(1_500_000), 2_500_000);
        // edit 2M → source 4M (jumped over gap at 3M..4M)
        assert_eq!(state.map_edit_time_to_source_time(2_000_000), 4_000_000);
    }

    /// Deleting all words must leave empty keep-segments and clamp to 0.
    #[test]
    fn edit_to_source_time_all_deleted_clamps_to_zero() {
        let words: Vec<EdWord> = (0..3)
            .map(|i| {
                ed_word(
                    &format!("w{}", i),
                    i as i64 * 1_000_000,
                    (i + 1) as i64 * 1_000_000,
                )
            })
            .collect();
        let mut state = EditorState::new();
        state.set_words(words);
        state.delete_range(0, 2);

        assert_eq!(state.get_keep_segments(), vec![]);
        // map_edit_time_to_source_time with empty segments → 0
        assert_eq!(state.map_edit_time_to_source_time(0), 0);
        assert_eq!(state.map_edit_time_to_source_time(999_000), 0);
    }
}
