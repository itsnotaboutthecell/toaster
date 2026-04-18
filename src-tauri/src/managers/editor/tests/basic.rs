//! Core EditorState regression tests (extracted from editor/mod.rs).

use super::super::*;
use super::common::make_words;

#[test]
fn set_words_and_count() {
    let mut editor = EditorState::new();
    assert_eq!(editor.get_words().len(), 0);
    editor.set_words(make_words());
    assert_eq!(editor.get_words().len(), 6);
}

#[test]
fn delete_word_marks_deleted() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    assert!(editor.delete_word(1));
    assert!(editor.get_words()[1].deleted);
    assert!(!editor.get_words()[0].deleted);
}

#[test]
fn delete_word_rejects_already_deleted() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    assert!(editor.delete_word(1));
    assert!(!editor.delete_word(1));
}

#[test]
fn delete_word_out_of_bounds() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    assert!(!editor.delete_word(99));
}

#[test]
fn undo_delete() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    editor.delete_word(1);
    assert!(editor.get_words()[1].deleted);
    assert!(editor.undo());
    assert!(!editor.get_words()[1].deleted);
}

#[test]
fn redo_after_undo() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    editor.delete_word(1);
    editor.undo();
    assert!(!editor.get_words()[1].deleted);
    assert!(editor.redo());
    assert!(editor.get_words()[1].deleted);
}

#[test]
fn undo_empty_returns_false() {
    let mut editor = EditorState::new();
    assert!(!editor.undo());
}

#[test]
fn redo_empty_returns_false() {
    let mut editor = EditorState::new();
    assert!(!editor.redo());
}

#[test]
fn new_mutation_clears_redo() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    editor.delete_word(0);
    editor.undo();
    // Now mutate again — redo should be cleared
    editor.delete_word(2);
    assert!(!editor.redo());
}

#[test]
fn delete_range() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    assert!(editor.delete_range(1, 3));
    assert!(!editor.get_words()[0].deleted);
    assert!(editor.get_words()[1].deleted);
    assert!(editor.get_words()[2].deleted);
    assert!(editor.get_words()[3].deleted);
    assert!(!editor.get_words()[4].deleted);
}

#[test]
fn delete_range_invalid() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    assert!(!editor.delete_range(3, 1)); // start > end
    assert!(!editor.delete_range(0, 99)); // end out of bounds
}

#[test]
fn restore_all() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    editor.delete_range(0, 2);
    assert!(editor.restore_all());
    assert!(editor.get_words().iter().all(|w| !w.deleted));
}

#[test]
fn restore_all_noop_when_none_deleted() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    assert!(!editor.restore_all());
}

#[test]
fn split_word_creates_two() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    // Split "Hello" (5 chars) at position 2 → "He" + "llo"
    assert!(editor.split_word(0, 2));
    assert_eq!(editor.get_words().len(), 7);
    assert_eq!(editor.get_words()[0].text, "He");
    assert_eq!(editor.get_words()[1].text, "llo");
}

#[test]
fn split_word_timestamps_proportional() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    // "Hello" is 0..1_000_000, split at 2/5
    editor.split_word(0, 2);
    assert_eq!(editor.get_words()[0].start_us, 0);
    assert_eq!(editor.get_words()[0].end_us, 400_000);
    assert_eq!(editor.get_words()[1].start_us, 400_000);
    assert_eq!(editor.get_words()[1].end_us, 1_000_000);
}

#[test]
fn split_word_invalid_position() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    assert!(!editor.split_word(0, 0)); // can't split at start
    assert!(!editor.split_word(0, 5)); // can't split at end (= full length)
    assert!(!editor.split_word(99, 1)); // out of bounds
}

#[test]
fn silence_word_toggles() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    assert!(!editor.get_words()[0].silenced);
    assert!(editor.silence_word(0));
    assert!(editor.get_words()[0].silenced);
    assert!(editor.silence_word(0));
    assert!(!editor.get_words()[0].silenced);
}

#[test]
fn silenced_ranges_returns_source_time_ranges_of_silenced_words() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    assert!(editor.get_silenced_ranges().is_empty());
    editor.silence_word(1); // "world" 1M..2M
    editor.silence_word(3); // "is"    3M..4M
    assert_eq!(
        editor.get_silenced_ranges(),
        vec![(1_000_000, 2_000_000), (3_000_000, 4_000_000)]
    );
}

#[test]
fn silenced_ranges_excludes_deleted_words() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    editor.silence_word(2); // "this" 2M..3M, will also be deleted
    editor.delete_word(2);
    // Deletion wins: silenced+deleted word is removed from the timeline
    // via keep-segments, so it must not also appear as a silence range.
    assert!(editor.get_silenced_ranges().is_empty());
}

#[test]
fn keep_segments_all_active() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    let segs = editor.get_keep_segments();
    assert_eq!(segs, vec![(0, 6_000_000)]);
}

#[test]
fn keep_segments_with_deletion() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    // Delete "this" (index 2), which spans 2M..3M
    editor.delete_word(2);
    let segs = editor.get_keep_segments();
    assert_eq!(segs, vec![(0, 2_000_000), (3_000_000, 6_000_000)]);
}

#[test]
fn keep_segments_all_deleted() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    editor.delete_range(0, 5);
    let segs = editor.get_keep_segments();
    assert!(segs.is_empty());
}

#[test]
fn edit_time_to_source_time_no_deletions() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    // No deletions — edit time == source time
    assert_eq!(editor.map_edit_time_to_source_time(500_000), 500_000);
    assert_eq!(editor.map_edit_time_to_source_time(3_000_000), 3_000_000);
}

#[test]
fn edit_time_to_source_time_with_deletion() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    // Delete "this" (2M..3M). Keep segments: [0..2M], [3M..6M]
    editor.delete_word(2);
    // Edit-time 0 → source 0
    assert_eq!(editor.map_edit_time_to_source_time(0), 0);
    // Edit-time 1M → source 1M (still in first segment)
    assert_eq!(editor.map_edit_time_to_source_time(1_000_000), 1_000_000);
    // Edit-time 2M → jumps over the gap → source 3M
    assert_eq!(editor.map_edit_time_to_source_time(2_000_000), 3_000_000);
    // Edit-time 3M → source 4M
    assert_eq!(editor.map_edit_time_to_source_time(3_000_000), 4_000_000);
}

#[test]
fn edit_time_past_end_clamps() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    assert_eq!(editor.map_edit_time_to_source_time(999_000_000), 6_000_000);
}

#[test]
fn undo_stack_cap_at_64() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    for _ in 0..70 {
        editor.silence_word(0);
    }
    // Stack should be capped at MAX_UNDO
    assert!(editor.undo_stack.len() <= MAX_UNDO);
}

#[test]
fn timeline_revision_advances_on_successful_mutations_only() {
    let mut editor = EditorState::new();
    assert_eq!(editor.timing_contract_snapshot().timeline_revision, 0);

    editor.set_words(make_words());
    let r1 = editor.timing_contract_snapshot().timeline_revision;
    assert_eq!(r1, 1);

    // Invalid mutation does not change revision.
    assert!(!editor.delete_word(999));
    assert_eq!(editor.timing_contract_snapshot().timeline_revision, r1);

    assert!(editor.delete_word(1));
    let r2 = editor.timing_contract_snapshot().timeline_revision;
    assert!(r2 > r1);

    assert!(editor.undo());
    let r3 = editor.timing_contract_snapshot().timeline_revision;
    assert!(r3 > r2);
}

#[test]
fn timing_contract_snapshot_reports_expected_counts_and_duration() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    editor.delete_word(2); // removes 1 second from keep content

    let snapshot = editor.timing_contract_snapshot();
    assert_eq!(snapshot.total_words, 6);
    assert_eq!(snapshot.deleted_words, 1);
    assert_eq!(snapshot.active_words, 5);
    assert_eq!(snapshot.source_start_us, 0);
    assert_eq!(snapshot.source_end_us, 6_000_000);
    assert_eq!(snapshot.total_keep_duration_us, 5_000_000);
    assert!(snapshot.keep_segments_valid);
    assert!(snapshot.warning.is_none());
}

#[test]
fn timing_contract_snapshot_flags_duration_mismatch() {
    let mut editor = EditorState::new();
    // Unsorted active words produce a malformed keep-segment duration aggregate.
    editor.set_words(vec![
        Word {
            text: "later".into(),
            start_us: 1_000_000,
            end_us: 2_000_000,
            deleted: false,
            silenced: false,
            confidence: 1.0,
            speaker_id: 0,
        },
        Word {
            text: "earlier".into(),
            start_us: 0,
            end_us: 1_000_000,
            deleted: false,
            silenced: false,
            confidence: 1.0,
            speaker_id: 0,
        },
    ]);

    let snapshot = editor.timing_contract_snapshot();
    assert!(!snapshot.keep_segments_valid);
    assert!(snapshot.warning.is_some());
}

#[test]
fn quantize_time_us_snaps_to_nearest_frame_boundary() {
    // 30 FPS -> 33_333.333us per frame. 20_000us should round to frame 1.
    let quantized = EditorState::quantize_time_us(20_000, 30, 1);
    assert!(quantized > 0);
    assert!(quantized < 40_000);
}

#[test]
fn quantized_keep_segments_are_monotonic() {
    let editor = EditorState::new();
    let segments = vec![(0, 10_000), (9_000, 11_000), (11_000, 50_000)];
    let quantized = editor.quantize_keep_segments(&segments, 30, 1);

    for window in quantized.windows(2) {
        assert!(window[0].1 <= window[1].0);
    }
    for (start, end) in quantized {
        assert!(start <= end);
    }
}

#[test]
fn timing_contract_snapshot_includes_quantized_segments() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());
    editor.delete_word(2);

    let snapshot = editor.timing_contract_snapshot();
    assert_eq!(snapshot.quantization_fps_num, 30);
    assert_eq!(snapshot.quantization_fps_den, 1);
    assert_eq!(
        snapshot.keep_segments.len(),
        snapshot.quantized_keep_segments.len()
    );
}
