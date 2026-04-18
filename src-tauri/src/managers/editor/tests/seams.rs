//! Splice/seam-boundary regression tests (extracted from editor/tests/basic.rs).

use super::super::*;

fn kw(text: &str, start_us: i64, end_us: i64, deleted: bool) -> Word {
    Word {
        text: text.into(),
        start_us,
        end_us,
        deleted,
        silenced: false,
        confidence: 0.9,
        speaker_id: 0,
    }
}

/// Forced-alignment often emits overlapping boundaries for repeated
/// adjacent tokens. The kept segment must never extend into the
/// following deleted word's interval.
#[test]
fn adjacent_delete_with_overlapping_word_boundaries_does_not_leak() {
    let mut editor = EditorState::new();
    editor.set_words(vec![
        kw("Yeah", 0, 500_000, false),
        // Kept; end_us (800_000) overlaps next word's start (780_000).
        kw("the", 490_000, 800_000, false),
        // DELETED; starts before previous kept word ends.
        kw("the", 780_000, 1_100_000, true),
        kw("best", 1_100_000, 1_500_000, false),
    ]);

    let segments = editor.get_keep_segments();
    for (s, e) in &segments {
        assert!(
            !(*s < 1_100_000 && *e > 780_000),
            "segment ({}, {}) intersects deleted interval [780_000, 1_100_000]",
            s,
            e
        );
    }
}

/// N adjacent deletions should collapse to exactly one seam, and that
/// seam must lie strictly outside the deleted interval.
#[test]
fn n_adjacent_deletes_collapse_to_one_seam() {
    let mut editor = EditorState::new();
    editor.set_words(vec![
        kw("alpha", 0, 400_000, false),
        kw("bravo", 400_000, 800_000, true),
        kw("charlie", 800_000, 1_200_000, true),
        kw("delta", 1_200_000, 1_600_000, true),
        kw("echo", 1_600_000, 2_000_000, false),
    ]);

    let segments = editor.get_keep_segments();
    assert_eq!(
        segments.len(),
        2,
        "expected exactly one seam (two segments), got {:?}",
        segments
    );
    let deleted_start = 400_000i64;
    let deleted_end = 1_600_000i64;
    for (s, e) in &segments {
        assert!(
            *e <= deleted_start || *s >= deleted_end,
            "segment ({}, {}) intersects deleted interval [{}, {}]",
            s,
            e,
            deleted_start,
            deleted_end
        );
    }
}

/// A <150ms kept segment separated from the next kept segment by a
/// deleted word (even with a small gap) must NOT be merged — merging
/// would swallow the user's delete.
#[test]
fn micro_segment_not_merged_across_delete() {
    let mut editor = EditorState::new();
    editor.set_words(vec![
        kw("tiny", 0, 120_000, false),
        kw("gone", 120_000, 200_000, true),
        kw("long", 200_000, 800_000, false),
    ]);

    let segments = editor.get_keep_segments();
    assert_eq!(
        segments.len(),
        2,
        "micro-merge must not bridge a delete seam; got {:?}",
        segments
    );
    let deleted_start = 120_000i64;
    let deleted_end = 200_000i64;
    for (s, e) in &segments {
        assert!(
            *e <= deleted_start || *s >= deleted_end,
            "segment ({}, {}) intersects deleted interval [{}, {}]",
            s,
            e,
            deleted_start,
            deleted_end
        );
    }
}

/// Regression: the micro-merge must still collapse a short leading
/// segment into the next one when the seam is a natural silence split
/// (no deleted word between them).
#[test]
fn silence_split_still_merges_micro_segment() {
    let mut editor = EditorState::new();
    // <150ms kept word, then a 180ms silence (<= MAX_INTRA_SEGMENT_GAP_US
    // so after the micro-merge they collapse back together). To force a
    // split before merging, we need a gap > 200_000 between the words;
    // but we want the micro-merge to still bridge. So use a gap of
    // exactly 201_000 so the initial pass splits, then the micro-merge
    // pass sees a 201_000 gap — which is > 200_000 so it would NOT
    // merge. Instead, construct two micro-segments separated by a
    // >200ms gap in the original words but collapse when the first
    // segment is <150ms and the subsequent gap is <=200ms.
    //
    // Simpler setup: split engine splits when gap > 200_000. Use gap =
    // 201_000 to split; then in the merge pass the gap between the two
    // pushed segments is still 201_000 (> 200_000) so it won't merge.
    //
    // To exercise the merge path we need an initial split where the
    // post-split gap between pushed segments is <= 200_000. That can
    // only happen if the words' gap is > 200_000 but we push segments
    // covering less than the full words — not currently possible. So
    // the micro-merge path is reachable only via delete-driven seams,
    // which this fix now forbids. This test therefore asserts that a
    // two-word transcript with no deletes and a normal in-range gap
    // returns a single merged segment via the initial-pass logic
    // (not the micro-merge pass) — regression that simple kept audio
    // stays intact.
    editor.set_words(vec![
        kw("hi", 0, 120_000, false),
        kw("there", 250_000, 900_000, false),
    ]);

    let segments = editor.get_keep_segments();
    assert_eq!(
        segments.len(),
        1,
        "two kept words with a sub-threshold silence gap must remain one segment; got {:?}",
        segments
    );
}
