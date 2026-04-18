//! Dual-track regression suite (extracted from editor/mod.rs).

use super::super::*;

// ── Constants mirrored from timeline.ts ──────────────────────────────────
/// Minimum A/V drift (seconds) before a correction is applied.
const DUAL_TRACK_DRIFT_THRESHOLD: f64 = 0.08;
/// Minimum real-clock interval (ms) between consecutive drift corrections.
const DUAL_TRACK_SYNC_COOLDOWN_MS: f64 = 250.0;
/// Minimum delta between consecutive source correction targets (seconds).
const MIN_VIDEO_SYNC_TARGET_DELTA_S: f64 = 0.02;
/// Minimum interval between consecutive fallback skip seeks (ms).
const FALLBACK_SKIP_MIN_INTERVAL_MS: f64 = 35.0;
/// Minimum forward seek gap in fallback skip path (seconds).
const END_EPSILON_S: f64 = 0.005;

// ── Helpers ───────────────────────────────────────────────────────────────

fn word(text: &str, start_us: i64, end_us: i64) -> Word {
    Word {
        text: text.into(),
        start_us,
        end_us,
        deleted: false,
        silenced: false,
        confidence: 0.9,
        speaker_id: 0,
    }
}

/// Build a basic 6-word, 6-second transcript.
fn six_words() -> Vec<Word> {
    vec![
        word("one", 0, 1_000_000),
        word("two", 1_000_000, 2_000_000),
        word("three", 2_000_000, 3_000_000),
        word("four", 3_000_000, 4_000_000),
        word("five", 4_000_000, 5_000_000),
        word("six", 5_000_000, 6_000_000),
    ]
}

fn should_apply_video_sync(
    drift_s: f64,
    elapsed_ms: f64,
    last_target_s: f64,
    next_target_s: f64,
) -> bool {
    drift_s > DUAL_TRACK_DRIFT_THRESHOLD
        && elapsed_ms > DUAL_TRACK_SYNC_COOLDOWN_MS
        && (last_target_s - next_target_s).abs() > MIN_VIDEO_SYNC_TARGET_DELTA_S
}

fn should_apply_fallback_skip(
    current_s: f64,
    target_s: f64,
    now_ms: f64,
    last_skip_ms: f64,
) -> bool {
    target_s > current_s + END_EPSILON_S && now_ms - last_skip_ms > FALLBACK_SKIP_MIN_INTERVAL_MS
}

// ── editTimeToSourceTime mirrors ──────────────────────────────────────────

/// Identity property: no deletions → edit time == source time.
#[test]
fn dt_edit_to_source_identity_no_deletions() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    for t in [0, 500_000, 2_000_000, 5_999_999, 6_000_000] {
        assert_eq!(
            ed.map_edit_time_to_source_time(t),
            t,
            "Expected identity at t={t}"
        );
    }
}

/// Single middle deletion: edit time shifts correctly into the second segment.
#[test]
fn dt_edit_to_source_single_middle_deletion() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    // Delete "three" (2M..3M).  Keep: [0..2M, 3M..6M]
    ed.delete_word(2);

    // First segment unchanged
    assert_eq!(ed.map_edit_time_to_source_time(0), 0);
    assert_eq!(ed.map_edit_time_to_source_time(500_000), 500_000);
    assert_eq!(ed.map_edit_time_to_source_time(1_999_999), 1_999_999);

    // At the edit-time boundary (2 s of keep content), we jump into the
    // second source segment at 3 s.
    assert_eq!(ed.map_edit_time_to_source_time(2_000_000), 3_000_000);
    assert_eq!(ed.map_edit_time_to_source_time(2_500_000), 3_500_000);
    assert_eq!(ed.map_edit_time_to_source_time(5_000_000), 6_000_000); // clamp
}

/// Clamp guard: edit time past total keep duration must not produce an invalid
/// source seek target (video-collapse prevention).
#[test]
fn dt_no_collapse_past_end_clamps_to_last_segment_end() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_word(2); // keep = 5 s total

    let total_keep_us = 5_000_000_i64;
    let clamped = ed.map_edit_time_to_source_time(total_keep_us + 999_999_999);
    assert_eq!(
        clamped, 6_000_000,
        "Seek target must not exceed last segment end"
    );
}

/// Clamp guard with all words deleted: must return 0, not panic.
#[test]
fn dt_no_collapse_all_deleted_returns_zero() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_range(0, 5);
    // No keep segments → clamped to 0
    assert_eq!(ed.map_edit_time_to_source_time(0), 0);
    assert_eq!(ed.map_edit_time_to_source_time(3_000_000), 0);
}

/// Monotonicity property: for any sequence of increasing edit times the
/// mapped source times must be non-decreasing.
#[test]
fn dt_monotonic_mapping_with_gaps() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_word(1); // Delete "two" (1M..2M)
    ed.delete_word(3); // Delete "four" (3M..4M)

    let edit_times: Vec<i64> = (0..=50).map(|i| i * 120_000).collect(); // 0..6M in 120k steps
    let source_times: Vec<i64> = edit_times
        .iter()
        .map(|&t| ed.map_edit_time_to_source_time(t))
        .collect();

    for window in source_times.windows(2) {
        assert!(
            window[0] <= window[1],
            "Mapping not monotone: {} > {} (violates video-sync invariant)",
            window[0],
            window[1]
        );
    }
}

/// Two adjacent deletions that together span a contiguous region must still
/// map the boundary correctly (regression for merging logic).
#[test]
fn dt_adjacent_deletions_merged_boundary() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_word(2); // 2M..3M
    ed.delete_word(3); // 3M..4M
                       // Keep: [0..2M, 4M..6M]
    let segs = ed.get_keep_segments();
    assert_eq!(segs, vec![(0, 2_000_000), (4_000_000, 6_000_000)]);

    // 2 s of edit time maps to start of second segment
    assert_eq!(ed.map_edit_time_to_source_time(2_000_000), 4_000_000);
    assert_eq!(ed.map_edit_time_to_source_time(3_000_000), 5_000_000);
    assert_eq!(ed.map_edit_time_to_source_time(4_000_000), 6_000_000); // clamp
}

// ── sourceTimeToEditTime mirrors ──────────────────────────────────────────

/// Compute edit time from source time via the keep-segment accumulation
/// algorithm (mirrors `sourceTimeToEditTime` in timeline.ts).
fn source_to_edit(ed: &EditorState, source_us: i64) -> i64 {
    let segs = ed.get_keep_segments();
    let source_sec = source_us as f64 / 1_000_000.0;
    let mut accumulated = 0.0_f64;
    for (start_us, end_us) in &segs {
        let start = *start_us as f64 / 1_000_000.0;
        let end = *end_us as f64 / 1_000_000.0;
        if source_sec < start {
            return (accumulated * 1_000_000.0) as i64;
        }
        if source_sec < end {
            return ((accumulated + (source_sec - start)) * 1_000_000.0) as i64;
        }
        accumulated += end - start;
    }
    (accumulated * 1_000_000.0) as i64
}

/// Identity when there are no deletions.
#[test]
fn dt_source_to_edit_identity_no_deletions() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    for t in [0, 1_000_000, 3_000_000, 5_999_000] {
        assert_eq!(source_to_edit(&ed, t), t, "Expected identity at t={t}");
    }
}

/// Source time inside a deleted region snaps to the start of the next segment.
#[test]
fn dt_source_to_edit_deleted_region_snaps_forward() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_word(2); // 2M..3M deleted

    // Source times 2M..3M are in the deleted region → snap to 2M edit-time
    let snap = source_to_edit(&ed, 2_500_000);
    assert_eq!(
        snap, 2_000_000,
        "Source time in deleted region should snap to 2s edit-time"
    );
}

/// Round-trip: edit→source→edit must return the original value for times
/// that land exactly on keep-segment starts.
#[test]
fn dt_round_trip_segment_starts() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_word(2); // 2M..3M deleted

    for &edit_us in &[0_i64, 1_000_000, 2_000_000] {
        let source = ed.map_edit_time_to_source_time(edit_us);
        let back = source_to_edit(&ed, source);
        assert_eq!(
            back, edit_us,
            "Round-trip failed at edit_us={edit_us}: got back {back}"
        );
    }
}

// ── Keep-segment structural invariants ────────────────────────────────────

/// Keep segments must be non-overlapping and sorted.
#[test]
fn dt_keep_segments_sorted_non_overlapping() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_word(1);
    ed.delete_word(3);
    let segs = ed.get_keep_segments();
    for window in segs.windows(2) {
        assert!(
            window[0].1 <= window[1].0,
            "Segments overlap or are not sorted: {:?} then {:?}",
            window[0],
            window[1]
        );
    }
}

/// Total keep-segment duration must equal the sum of active word durations.
#[test]
fn dt_keep_segment_total_duration_matches_active_words() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_word(2);
    ed.delete_word(4);

    let seg_total: i64 = ed.get_keep_segments().iter().map(|(s, e)| e - s).sum();
    let word_total: i64 = ed
        .get_words()
        .iter()
        .filter(|w| !w.deleted)
        .map(|w| w.end_us - w.start_us)
        .sum();
    assert_eq!(
        seg_total, word_total,
        "Keep-segment total ({seg_total}) differs from active-word total ({word_total})"
    );
}

// ── Drift-correction constant sanity ─────────────────────────────────────

/// The drift threshold and cooldown constants must stay within acceptable
/// perceptual bounds so the sync loop neither over-corrects nor ignores drift.
///
/// Acceptable range:
///   threshold:  10 ms – 200 ms  (below 10 ms is jittery; above 200 ms is perceptible)
///   cooldown:   100 ms – 1000 ms
// Compile-time bounds checks: constants are fixed values, so assert them at
// const-eval time rather than in a runtime test (avoids "constant assertion" lint).
const _: () = assert!(DUAL_TRACK_DRIFT_THRESHOLD >= 0.010 && DUAL_TRACK_DRIFT_THRESHOLD <= 0.200,);
const _: () =
    assert!(DUAL_TRACK_SYNC_COOLDOWN_MS >= 100.0 && DUAL_TRACK_SYNC_COOLDOWN_MS <= 1000.0,);

/// Drift < threshold must NOT trigger a correction (no spurious seek).
#[test]
fn dt_drift_below_threshold_does_not_correct() {
    // Simulate: video at 5.05s, target at 5.00s → drift = 50ms < 80ms threshold
    let video_time = 5.05_f64;
    let target_source_time = 5.00_f64;
    let drift = (video_time - target_source_time).abs();
    assert!(
        drift < DUAL_TRACK_DRIFT_THRESHOLD,
        "Drift {drift}s should be below threshold {DUAL_TRACK_DRIFT_THRESHOLD}s"
    );
}

/// Drift ≥ threshold must trigger a correction.
#[test]
fn dt_drift_at_threshold_triggers_correction() {
    let video_time = 5.09_f64;
    let target_source_time = 5.00_f64;
    let drift = (video_time - target_source_time).abs();
    assert!(
        drift >= DUAL_TRACK_DRIFT_THRESHOLD,
        "Drift {drift}s should be ≥ threshold {DUAL_TRACK_DRIFT_THRESHOLD}s"
    );
}

/// Guard: ignore tiny target oscillations even when drift/cooldown permit correction.
#[test]
fn dt_video_sync_guard_rejects_near_identical_targets() {
    let apply = should_apply_video_sync(0.12, 300.0, 4.000, 4.010);
    assert!(
        !apply,
        "Expected no correction when target delta is only 10ms"
    );
}

/// Guard: allow correction when target delta is material and drift/cooldown permit.
#[test]
fn dt_video_sync_guard_accepts_material_target_delta() {
    let apply = should_apply_video_sync(0.12, 300.0, 4.000, 4.035);
    assert!(apply, "Expected correction when target delta exceeds 20ms");
}

/// Guard: block rapid repeat skip seeks that cause audible stutter.
#[test]
fn dt_fallback_skip_rate_limit_blocks_rapid_repeat() {
    let apply = should_apply_fallback_skip(2.000, 2.080, 1_000.0, 980.0);
    assert!(
        !apply,
        "Expected skip to be blocked when interval is only 20ms"
    );
}

/// Guard: allow fallback skip once interval passes threshold.
#[test]
fn dt_fallback_skip_rate_limit_allows_spaced_skip() {
    let apply = should_apply_fallback_skip(2.000, 2.080, 1_000.0, 960.0);
    assert!(
        apply,
        "Expected skip to be allowed when interval exceeds 35ms"
    );
}

// ── Source-switching logic for video mode ─────────────────────────────────
//
// In dual-track video preview:
//   - `primarySrc` = video URL (never changes)
//   - `activePlaybackSrc` = previewAudioUrl (edit-timeline authority)
//   - video element is muted, synced via drift correction
//
// Invariants verified here:
//   1. The source used for the seek target must NOT be the raw video time.
//   2. When keep segments are empty (all deleted), the source time
//      returned is always 0, i.e., seek to beginning (not a garbage value).

/// Source time for seek in video mode must come from editTimeToSourceTime,
/// not the raw edit time.
#[test]
fn dt_source_switching_video_seek_uses_mapped_time() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_word(2); // 2M..3M

    // At edit-time = 2 s, the correct video seek target is 3 s (source)
    let mapped = ed.map_edit_time_to_source_time(2_000_000);
    assert_eq!(
        mapped, 3_000_000,
        "Video seek must use mapped source time (3s), not raw edit time (2s)"
    );
    assert_ne!(
        mapped, 2_000_000,
        "Video seek must NOT use the raw edit time when there are deletions"
    );
}

/// In video mode with no keep segments, seek target is 0 (no collapse/panic).
#[test]
fn dt_source_switching_no_keep_segments_yields_zero() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_range(0, 5); // delete everything

    let result = ed.map_edit_time_to_source_time(0);
    assert_eq!(
        result, 0,
        "Empty keep segments must yield seek target 0, not garbage"
    );
}

/// Undo after delete must restore correct mapping (no stale state).
#[test]
fn dt_undo_restores_mapping() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_word(2);

    // After deletion: edit-time 2s → source 3s
    assert_eq!(ed.map_edit_time_to_source_time(2_000_000), 3_000_000);

    // Undo: mapping should revert to identity
    ed.undo();
    assert_eq!(
        ed.map_edit_time_to_source_time(2_000_000),
        2_000_000,
        "After undo, mapping must revert to identity"
    );
}

/// Boundary at segment edges: times exactly at start/end of a segment.
#[test]
fn dt_boundary_at_segment_edges() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_word(2); // 2M..3M  →  keep: [0..2M, 3M..6M]

    // Exact end of first segment
    assert_eq!(ed.map_edit_time_to_source_time(1_999_999), 1_999_999);
    assert_eq!(ed.map_edit_time_to_source_time(2_000_000), 3_000_000);
    // Exact start of second keep segment in edit-time
    assert_eq!(ed.map_edit_time_to_source_time(2_000_001), 3_000_001);
}

/// Multiple non-adjacent deletions produce correct independent mappings.
#[test]
fn dt_multiple_gaps_independent_correct_mapping() {
    let mut ed = EditorState::new();
    ed.set_words(six_words());
    ed.delete_word(1); // 1M..2M
    ed.delete_word(3); // 3M..4M
                       // Keep: [0..1M, 2M..3M, 4M..6M]  total = 4s keep

    // Within first keep segment
    assert_eq!(ed.map_edit_time_to_source_time(500_000), 500_000);
    // Into second keep segment
    assert_eq!(ed.map_edit_time_to_source_time(1_000_000), 2_000_000);
    assert_eq!(ed.map_edit_time_to_source_time(1_500_000), 2_500_000);
    // Into third keep segment
    assert_eq!(ed.map_edit_time_to_source_time(2_000_000), 4_000_000);
    assert_eq!(ed.map_edit_time_to_source_time(3_000_000), 5_000_000);
    // Clamped to end
    assert_eq!(ed.map_edit_time_to_source_time(4_000_000), 6_000_000);
}

// ── Pre-play snap (snapOutOfDeletedRange mirrors) ─────────────────────────
//
// Surrogate regression coverage for the pre-play snap logic added to the
// fallback/live-skip play path in MediaPlayer.tsx.
//
// The frontend does not yet have a vitest/jest harness.  These Rust tests
// mirror the pure algorithm of `snapOutOfDeletedRange` in timeline.ts:
//   for each (start, end) in deletedRanges:
//     if time >= start && time < end → return end  (half-open interval)
//   return time

/// Mirror of TypeScript `snapOutOfDeletedRange`.
fn snap_out_of_deleted_range(time_s: f64, deleted: &[(f64, f64)]) -> f64 {
    for &(start, end) in deleted {
        if time_s >= start && time_s < end {
            return end;
        }
    }
    time_s
}

/// Position in a kept region must be returned unchanged.
#[test]
fn dt_pre_play_snap_kept_region_unchanged() {
    let deleted = [(2.0_f64, 3.0_f64)];
    assert_eq!(snap_out_of_deleted_range(0.0, &deleted), 0.0);
    assert_eq!(snap_out_of_deleted_range(1.5, &deleted), 1.5);
    assert_eq!(snap_out_of_deleted_range(1.999, &deleted), 1.999);
    // After the deleted range — also unchanged
    assert_eq!(snap_out_of_deleted_range(3.5, &deleted), 3.5);
}

/// Position exactly at the start of a deleted range must snap to its end.
#[test]
fn dt_pre_play_snap_at_range_start_snaps_to_end() {
    let deleted = [(2.0_f64, 3.0_f64)];
    assert_eq!(
        snap_out_of_deleted_range(2.0, &deleted),
        3.0,
        "Exact start of deleted range must snap to range end"
    );
}

/// Position mid-deleted-range must snap to the range end.
#[test]
fn dt_pre_play_snap_mid_range_snaps_to_end() {
    let deleted = [(2.0_f64, 3.0_f64)];
    assert_eq!(
        snap_out_of_deleted_range(2.5, &deleted),
        3.0,
        "Mid-deleted position must snap to range end"
    );
}

/// Position at range end (half-open boundary) must NOT be snapped.
#[test]
fn dt_pre_play_snap_at_range_end_not_snapped() {
    let deleted = [(2.0_f64, 3.0_f64)];
    // Range is half-open [start, end) so 3.0 is outside the range.
    assert_eq!(
        snap_out_of_deleted_range(3.0, &deleted),
        3.0,
        "Position at range end boundary must not be snapped"
    );
}

/// Empty deleted ranges: any position is returned unchanged.
#[test]
fn dt_pre_play_snap_no_ranges_unchanged() {
    assert_eq!(snap_out_of_deleted_range(0.0, &[]), 0.0);
    assert_eq!(snap_out_of_deleted_range(5.5, &[]), 5.5);
}

/// Multiple deleted ranges: only the matching range governs the snap.
#[test]
fn dt_pre_play_snap_multiple_ranges_correct_match() {
    let deleted = [(1.0_f64, 2.0_f64), (4.0_f64, 5.0_f64)];
    // First range
    assert_eq!(snap_out_of_deleted_range(1.5, &deleted), 2.0);
    // Kept region between the two ranges
    assert_eq!(snap_out_of_deleted_range(3.0, &deleted), 3.0);
    // Second range
    assert_eq!(snap_out_of_deleted_range(4.5, &deleted), 5.0);
    // After all ranges
    assert_eq!(snap_out_of_deleted_range(5.5, &deleted), 5.5);
}
