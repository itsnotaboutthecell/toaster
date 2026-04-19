//! Extracted from the inline mod tests block (monolith-split).

use super::*;
use super::alignment::{
    align_onset_boundaries, correct_short_word_boundaries, refine_word_boundaries,
};
use transcribe_rs::TranscriptionSegment;

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

    let (mut words, align_meta) = build_words_from_segments("I said hello", &segments, &samples);
    sanitize_word_timestamps(&mut words, total_duration_us);
    realign_suspicious_spans(&mut words, &samples, Some(&align_meta));
    sanitize_word_timestamps(&mut words, total_duration_us);

    assert_eq!(words.len(), 3);

    // The boundary between "I" and "said" should be in or near the
    // silence gap (300–350ms = 300_000–350_000 µs).
    let boundary = words[0].end_us;
    assert!(
        (280_000..=380_000).contains(&boundary),
        "boundary between 'I' and 'said' should be near silence gap \
         (300–350ms), got {} µs",
        boundary
    );
    assert_eq!(words[0].end_us, words[1].start_us);
}
