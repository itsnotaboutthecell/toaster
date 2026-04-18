//! Precision eval tests (extracted from editor/mod.rs).

use super::super::*;

/// Heterogeneous, non-uniform durations — critical for anti-synthesis
/// guard. If any code path ever resets these to equal spans, this
/// fixture will detect it.
fn heterogeneous_words() -> Vec<Word> {
    vec![
        Word {
            text: "The".into(),
            start_us: 100_000,
            end_us: 250_000,
            deleted: false,
            silenced: false,
            confidence: 0.98,
            speaker_id: 0,
        },
        Word {
            text: "quick".into(),
            start_us: 280_000,
            end_us: 690_000,
            deleted: false,
            silenced: false,
            confidence: 0.97,
            speaker_id: 0,
        },
        Word {
            text: "brown".into(),
            start_us: 720_000,
            end_us: 1_180_000,
            deleted: false,
            silenced: false,
            confidence: 0.96,
            speaker_id: 0,
        },
        Word {
            text: "fox".into(),
            start_us: 1_220_000,
            end_us: 1_500_000,
            deleted: false,
            silenced: false,
            confidence: 0.99,
            speaker_id: 0,
        },
        Word {
            text: "jumps".into(),
            start_us: 1_600_000,
            end_us: 2_050_000,
            deleted: false,
            silenced: false,
            confidence: 0.95,
            speaker_id: 0,
        },
    ]
}

/// Anti-synthesis guard: per-word durations must never collapse to a
/// single equal value after any round-trip through editor state.
#[test]
fn precision_eval_no_equal_duration_synthesis() {
    let words = heterogeneous_words();
    let mut editor = EditorState::new();
    editor.set_words(words.clone());

    let got = editor.get_words();
    let original_durations: Vec<i64> = words.iter().map(|w| w.end_us - w.start_us).collect();
    let got_durations: Vec<i64> = got.iter().map(|w| w.end_us - w.start_us).collect();

    assert_eq!(
        original_durations, got_durations,
        "set_words must not mutate per-word durations",
    );

    let first = got_durations[0];
    assert!(
        got_durations.iter().any(|d| *d != first),
        "precision violation: all word durations are equal ({first}) \
         — a synthesis path has been introduced",
    );
}

/// Keep-segment round-trip: delete then undo must restore exact
/// per-word timing and produce an identical keep-segment set.
#[test]
fn precision_eval_delete_undo_roundtrip_preserves_timing() {
    let words = heterogeneous_words();
    let mut editor = EditorState::new();
    editor.set_words(words.clone());

    let original_words = editor.get_words().to_vec();
    let original_keep = editor.get_keep_segments();

    assert!(editor.delete_word(2), "delete_word(2) should succeed");
    assert_ne!(editor.get_keep_segments(), original_keep);

    assert!(editor.undo(), "undo should succeed");

    let restored = editor.get_words();
    assert_eq!(
        restored.len(),
        original_words.len(),
        "undo should restore word count",
    );
    for (o, r) in original_words.iter().zip(restored.iter()) {
        assert_eq!(o.start_us, r.start_us, "start_us drift after undo");
        assert_eq!(o.end_us, r.end_us, "end_us drift after undo");
        assert_eq!(o.text, r.text, "text drift after undo");
        assert_eq!(o.deleted, r.deleted, "deleted flag drift after undo");
    }
    assert_eq!(
        editor.get_keep_segments(),
        original_keep,
        "keep_segments drift after undo",
    );
}

/// Midstream deletion splice: deleting a word in the middle must
/// produce keep-segments whose boundaries match the kept words'
/// original timestamps exactly — no smoothing, no remnants.
#[test]
fn precision_eval_midstream_delete_clean_splice() {
    let words = heterogeneous_words();
    let mut editor = EditorState::new();
    editor.set_words(words.clone());

    assert!(editor.delete_word(2), "delete 'brown'");

    let segs = editor.get_keep_segments();
    assert_eq!(segs.len(), 2, "expected exactly two keep segments");

    let (a_start, a_end) = segs[0];
    let (b_start, b_end) = segs[1];
    assert_eq!(a_start, words[0].start_us, "first segment start drift");
    assert_eq!(a_end, words[1].end_us, "first segment end drift");
    assert_eq!(b_start, words[3].start_us, "second segment start drift");
    assert_eq!(b_end, words[4].end_us, "second segment end drift");

    let edit_point = a_end - a_start;
    let mapped = editor.map_edit_time_to_source_time(edit_point);
    assert_eq!(
        mapped, b_start,
        "splice point must map to the start of the next kept word — \
         any other value means remnant content leaked through",
    );
}

/// Edit-time → source-time mapping stays monotonic across multiple
/// deletions. A non-monotonic map would play audio out of order.
#[test]
fn precision_eval_time_mapping_monotonic_after_multiple_deletes() {
    let words = heterogeneous_words();
    let mut editor = EditorState::new();
    editor.set_words(words);
    assert!(editor.delete_word(1));
    assert!(editor.delete_word(3));

    let samples = [0_i64, 100_000, 250_000, 500_000, 800_000];
    let mut prev = 0_i64;
    for (i, &edit_t) in samples.iter().enumerate() {
        let src = editor.map_edit_time_to_source_time(edit_t);
        if i > 0 {
            assert!(
                src >= prev,
                "time map non-monotonic at sample {i}: prev={prev} got={src}",
            );
        }
        prev = src;
    }
}

/// Fixture-based precision eval.
///
/// Loads the checked-in golden word fixture
/// (`src-tauri/tests/fixtures/toaster_example.words.golden.json`) and
/// validates the full precision contract in one pass:
///
/// 1. Loaded durations are heterogeneous (anti-synthesis baseline).
/// 2. `set_words` preserves every field byte-for-byte.
/// 3. A midstream deletion produces keep-segments whose boundaries
///    match the fixture's original per-word timestamps exactly.
/// 4. Delete + undo round-trips the fixture to its original state.
///
/// Any future regression in word timing, keep-segment arithmetic, or
/// undo fidelity will fail this test. DO NOT regenerate the fixture
/// without human verification — see
/// `.github/skills/transcript-precision-eval/SKILL.md`.
#[test]
fn precision_eval_golden_fixture_roundtrip() {
    #[derive(serde::Deserialize)]
    struct Fixture {
        words: Vec<Word>,
    }

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("toaster_example.words.golden.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read golden fixture {}: {}", path.display(), e));
    let fixture: Fixture = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse golden fixture {}: {}", path.display(), e));
    let words = fixture.words;
    assert!(
        words.len() >= 6,
        "golden fixture must have >= 6 words to exercise midstream splice"
    );

    let durations: Vec<i64> = words.iter().map(|w| w.end_us - w.start_us).collect();
    let unique: std::collections::HashSet<i64> = durations.iter().copied().collect();
    assert!(
        unique.len() >= durations.len() / 2,
        "golden fixture durations collapsed to too few unique values ({unique:?}) — \
         fixture may have been regenerated by a synthesis path"
    );

    let mut editor = EditorState::new();
    editor.set_words(words.clone());

    let loaded = editor.get_words().to_vec();
    for (orig, got) in words.iter().zip(loaded.iter()) {
        assert_eq!(orig.text, got.text);
        assert_eq!(orig.start_us, got.start_us, "start_us drift on load");
        assert_eq!(orig.end_us, got.end_us, "end_us drift on load");
        assert_eq!(orig.confidence, got.confidence);
        assert_eq!(orig.speaker_id, got.speaker_id);
    }

    let mid = words.len() / 2;
    let baseline_keep = editor.get_keep_segments();
    assert!(editor.delete_word(mid), "delete_word({mid})");

    let segs = editor.get_keep_segments();
    assert_eq!(
        segs.len(),
        2,
        "midstream delete should produce exactly two keep-segments"
    );
    let (a_start, a_end) = segs[0];
    let (b_start, b_end) = segs[1];
    assert_eq!(a_start, words[0].start_us, "seg-A start drift");
    assert_eq!(a_end, words[mid - 1].end_us, "seg-A end drift");
    assert_eq!(b_start, words[mid + 1].start_us, "seg-B start drift");
    assert_eq!(b_end, words[words.len() - 1].end_us, "seg-B end drift");

    let edit_point = a_end - a_start;
    let mapped = editor.map_edit_time_to_source_time(edit_point);
    assert_eq!(
        mapped, b_start,
        "midstream splice remnant leaked: mapped={mapped} expected={b_start}"
    );

    assert!(editor.undo(), "undo after midstream delete");
    assert_eq!(
        editor.get_keep_segments(),
        baseline_keep,
        "undo did not restore keep-segment baseline"
    );
    let restored = editor.get_words();
    for (orig, got) in words.iter().zip(restored.iter()) {
        assert_eq!(orig.start_us, got.start_us, "start_us drift after undo");
        assert_eq!(orig.end_us, got.end_us, "end_us drift after undo");
        assert_eq!(orig.deleted, got.deleted, "deleted flag drift after undo");
    }
}
