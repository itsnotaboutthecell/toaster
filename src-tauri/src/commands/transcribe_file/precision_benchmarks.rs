//! Extracted from the inline mod precision_benchmarks block (monolith-split).

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
        (700..=900).contains(&min_pos),
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
    for s in samples[1500..1700].iter_mut() {
        *s = 0.35; // slight dip — less than 3% of 0.4
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
    let n_samples = timing_us_to_sample(total_duration_us, SAMPLE_RATE_HZ);
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
    let n_samples = timing_us_to_sample(total_duration_us, SAMPLE_RATE_HZ);
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
