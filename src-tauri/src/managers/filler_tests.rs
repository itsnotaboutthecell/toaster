//! Extracted from the inline `mod tests` block (monolith-split).

use super::*;

/// Helper to build a Word with sensible defaults.
fn word(text: &str, start_us: i64, end_us: i64) -> Word {
    Word {
        text: text.to_string(),
        start_us,
        end_us,
        deleted: false,
        silenced: false,
        confidence: 1.0,
        speaker_id: -1,
    }
}

fn deleted_word(text: &str, start_us: i64, end_us: i64) -> Word {
    Word {
        deleted: true,
        ..word(text, start_us, end_us)
    }
}

fn default_config() -> FillerConfig {
    FillerConfig::default()
}

// ── detect_fillers ──────────────────────────────────────────────

#[test]
fn fillers_basic_match() {
    let words = vec![
        word("Hello", 0, 500_000),
        word("um", 600_000, 800_000),
        word("world", 900_000, 1_200_000),
        word("uh", 1_300_000, 1_500_000),
        word("like", 1_600_000, 1_800_000),
    ];
    let result = detect_fillers(&words, &default_config());
    assert_eq!(result, vec![1, 3, 4]);
}

#[test]
fn fillers_case_insensitive() {
    let words = vec![
        word("Um", 0, 500_000),
        word("UH", 600_000, 800_000),
        word("Like", 900_000, 1_100_000),
    ];
    let result = detect_fillers(&words, &default_config());
    assert_eq!(result, vec![0, 1, 2]);
}

#[test]
fn fillers_with_punctuation() {
    let words = vec![
        word("um,", 0, 500_000),
        word("uh.", 600_000, 800_000),
        word("like!", 900_000, 1_100_000),
    ];
    let result = detect_fillers(&words, &default_config());
    assert_eq!(result, vec![0, 1, 2]);
}

#[test]
fn fillers_skips_deleted_words() {
    let words = vec![
        word("hello", 0, 500_000),
        deleted_word("um", 600_000, 800_000),
        word("world", 900_000, 1_200_000),
    ];
    let result = detect_fillers(&words, &default_config());
    assert!(result.is_empty());
}

#[test]
fn fillers_multi_word_you_know() {
    let words = vec![
        word("I", 0, 200_000),
        word("you", 300_000, 500_000),
        word("know", 600_000, 800_000),
        word("right", 900_000, 1_000_000),
    ];
    let result = detect_fillers(&words, &default_config());
    // "you know" → indices 1,2;  "right" → index 3
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn fillers_multi_word_i_mean() {
    let words = vec![
        word("I", 0, 200_000),
        word("mean", 300_000, 500_000),
        word("it's", 600_000, 800_000),
    ];
    let result = detect_fillers(&words, &default_config());
    assert_eq!(result, vec![0, 1]);
}

#[test]
fn fillers_multi_word_broken_by_deleted() {
    let words = vec![
        word("you", 0, 200_000),
        deleted_word("really", 300_000, 500_000),
        word("know", 600_000, 800_000),
    ];
    // "really" is deleted, so "you" and "know" are consecutive active words
    // and should match "you know".
    let result = detect_fillers(&words, &default_config());
    assert_eq!(result, vec![0, 2]);
}

#[test]
fn fillers_empty_list() {
    let result = detect_fillers(&[], &default_config());
    assert!(result.is_empty());
}

#[test]
fn fillers_no_match() {
    let words = vec![word("hello", 0, 500_000), word("world", 600_000, 1_000_000)];
    let result = detect_fillers(&words, &default_config());
    assert!(result.is_empty());
}

#[test]
fn fillers_custom_config() {
    let config = FillerConfig {
        filler_words: vec!["hmm".to_string(), "yeah".to_string()],
        ..Default::default()
    };
    let words = vec![
        word("um", 0, 500_000), // not in custom list
        word("hmm", 600_000, 800_000),
        word("yeah", 900_000, 1_100_000),
    ];
    let result = detect_fillers(&words, &config);
    assert_eq!(result, vec![1, 2]);
}

// ── detect_pauses ───────────────────────────────────────────────

#[test]
fn pauses_finds_long_gaps() {
    let words = vec![
        word("hello", 0, 500_000),
        word("world", 2_500_000, 3_000_000), // 2s gap → detected
    ];
    let result = detect_pauses(&words, &default_config());
    assert_eq!(result, vec![(0, 2_000_000)]);
}

#[test]
fn pauses_ignores_short_gaps() {
    let words = vec![
        word("hello", 0, 500_000),
        word("world", 600_000, 1_000_000), // 100ms gap → not a pause
    ];
    let result = detect_pauses(&words, &default_config());
    assert!(result.is_empty());
}

#[test]
fn pauses_skips_deleted_words() {
    let words = vec![
        word("hello", 0, 500_000),
        deleted_word("filler", 600_000, 800_000),
        word("world", 900_000, 1_200_000), // gap from hello(500k) to world(900k) = 400ms
    ];
    let result = detect_pauses(&words, &default_config());
    assert!(result.is_empty());
}

#[test]
fn pauses_gap_across_deleted_words() {
    let words = vec![
        word("hello", 0, 500_000),
        deleted_word("x", 600_000, 700_000),
        word("world", 3_000_000, 3_500_000), // gap 500k→3M = 2.5s
    ];
    let result = detect_pauses(&words, &default_config());
    assert_eq!(result, vec![(0, 2_500_000)]);
}

#[test]
fn pauses_empty_list() {
    let result = detect_pauses(&[], &default_config());
    assert!(result.is_empty());
}

#[test]
fn pauses_single_word() {
    let words = vec![word("hello", 0, 500_000)];
    let result = detect_pauses(&words, &default_config());
    assert!(result.is_empty());
}

#[test]
fn pauses_custom_threshold() {
    let config = FillerConfig {
        pause_threshold_us: 500_000, // 0.5 seconds
        ..Default::default()
    };
    let words = vec![
        word("hello", 0, 500_000),
        word("world", 1_200_000, 1_500_000), // 700ms gap → detected at 500ms threshold
    ];
    let result = detect_pauses(&words, &config);
    assert_eq!(result, vec![(0, 700_000)]);
}

// ── analyze ─────────────────────────────────────────────────────

#[test]
fn analyze_returns_fillers_and_pauses() {
    let words = vec![
        word("so", 0, 200_000),       // "so" IS a default filler
        word("um", 300_000, 500_000), // filler
        word("hello", 600_000, 1_000_000),
        word("world", 3_000_000, 3_500_000), // 2s gap after "hello"
    ];
    let result = analyze(&words, &default_config());
    assert_eq!(result.filler_indices, vec![0, 1]); // both "so" and "um"
    assert_eq!(result.pauses, vec![(2, 2_000_000)]);
}

#[test]
fn analyze_empty_words() {
    let result = analyze(&[], &default_config());
    assert_eq!(
        result,
        AnalysisResult {
            filler_indices: vec![],
            pauses: vec![],
            duplicate_indices: vec![],
        }
    );
}

// ── normalize_filler ────────────────────────────────────────────

#[test]
fn normalize_filler_collapses_trailing_repeat() {
    assert_eq!(normalize_filler("umm"), "um");
    assert_eq!(normalize_filler("uhhh"), "uh");
    assert_eq!(normalize_filler("hmmm"), "hm");
    assert_eq!(normalize_filler("ummmmm"), "um");
}

#[test]
fn normalize_filler_already_normalized() {
    assert_eq!(normalize_filler("um"), "um");
}

#[test]
fn normalize_filler_no_trailing_repeat() {
    assert_eq!(normalize_filler("like"), "like");
}

#[test]
fn fuzzy_filler_matches_umm() {
    let words = vec![
        word("hello", 0, 500_000),
        word("umm", 600_000, 800_000),
        word("world", 900_000, 1_200_000),
    ];
    let result = detect_fillers(&words, &default_config());
    assert_eq!(result, vec![1]);
}

// ── detect_duplicates ───────────────────────────────────────────

#[test]
fn duplicates_finds_adjacent_pair() {
    let words = vec![
        word("the", 0, 200_000),
        word("the", 300_000, 500_000),
        word("best", 600_000, 800_000),
    ];
    assert_eq!(detect_duplicates(&words), vec![1]);
}

#[test]
fn duplicates_no_match_non_adjacent() {
    let words = vec![
        word("the", 0, 200_000),
        word("a", 300_000, 500_000),
        word("the", 600_000, 800_000),
    ];
    assert_eq!(detect_duplicates(&words), Vec::<usize>::new());
}

#[test]
fn duplicates_triple() {
    let words = vec![
        word("the", 0, 200_000),
        word("the", 300_000, 500_000),
        word("the", 600_000, 800_000),
    ];
    assert_eq!(detect_duplicates(&words), vec![1, 2]);
}

#[test]
fn duplicates_skips_deleted() {
    let words = vec![
        word("the", 0, 200_000),
        deleted_word("the", 300_000, 500_000),
        word("best", 600_000, 800_000),
    ];
    assert_eq!(detect_duplicates(&words), Vec::<usize>::new());
}

#[test]
fn duplicates_across_deleted_gap() {
    let words = vec![
        word("the", 0, 200_000),
        deleted_word("um", 300_000, 400_000),
        word("the", 500_000, 700_000),
    ];
    // "the" and "the" are adjacent non-deleted words
    assert_eq!(detect_duplicates(&words), vec![2]);
}

#[test]
fn duplicates_multiple_pairs() {
    let words = vec![
        word("the", 0, 200_000),
        word("the", 300_000, 500_000),
        word("best", 600_000, 800_000),
        word("best", 900_000, 1_100_000),
        word("part", 1_200_000, 1_400_000),
    ];
    assert_eq!(detect_duplicates(&words), vec![1, 3]);
}

// ── trim_pauses ─────────────────────────────────────────────────

#[test]
fn trim_reduces_2s_gap_to_300ms() {
    let mut words = vec![
        word("hello", 0, 500_000),
        word("world", 2_500_000, 3_000_000), // 2s gap
    ];
    let count = trim_pauses(&mut words, DEFAULT_PAUSE_THRESHOLD_US, DEFAULT_MAX_GAP_US);
    assert_eq!(count, 1);
    assert_eq!(words[0].start_us, 0);
    assert_eq!(words[0].end_us, 500_000);
    // world.start = 500_000 + 300_000 = 800_000
    assert_eq!(words[1].start_us, 800_000);
    assert_eq!(words[1].end_us, 1_300_000);
}

#[test]
fn trim_shifts_subsequent_words() {
    let mut words = vec![
        word("hello", 0, 500_000),
        word("world", 2_500_000, 3_000_000), // 2s gap
        word("foo", 3_100_000, 3_500_000),
    ];
    let count = trim_pauses(&mut words, DEFAULT_PAUSE_THRESHOLD_US, DEFAULT_MAX_GAP_US);
    assert_eq!(count, 1);
    let shift = 2_000_000 - DEFAULT_MAX_GAP_US; // 1_700_000
    assert_eq!(words[1].start_us, 2_500_000 - shift);
    assert_eq!(words[2].start_us, 3_100_000 - shift);
    assert_eq!(words[2].end_us, 3_500_000 - shift);
}

#[test]
fn trim_shifts_deleted_words_too() {
    // Deleted word *after* the gap should also be shifted
    let mut words = vec![
        word("hello", 0, 500_000),
        word("world", 2_500_000, 3_000_000), // 2s gap
        deleted_word("removed", 3_100_000, 3_300_000),
        word("foo", 3_400_000, 3_700_000),
    ];
    let count = trim_pauses(&mut words, DEFAULT_PAUSE_THRESHOLD_US, DEFAULT_MAX_GAP_US);
    assert_eq!(count, 1);
    let shift = 2_000_000 - DEFAULT_MAX_GAP_US;
    assert_eq!(words[1].start_us, 2_500_000 - shift);
    // Deleted word after the gap is also shifted
    assert_eq!(words[2].start_us, 3_100_000 - shift);
    assert_eq!(words[3].start_us, 3_400_000 - shift);
}

#[test]
fn trim_does_not_shift_deleted_word_inside_gap() {
    // Deleted word *within* the gap (between the two non-deleted words)
    // sits before the gap's after-index so it is not shifted
    let mut words = vec![
        word("hello", 0, 500_000),
        deleted_word("filler", 600_000, 700_000),
        word("world", 2_500_000, 3_000_000),
    ];
    let count = trim_pauses(&mut words, DEFAULT_PAUSE_THRESHOLD_US, DEFAULT_MAX_GAP_US);
    assert_eq!(count, 1);
    assert_eq!(words[1].start_us, 600_000); // not shifted
    let shift = 2_000_000 - DEFAULT_MAX_GAP_US;
    assert_eq!(words[2].start_us, 2_500_000 - shift);
}

#[test]
fn trim_ignores_gap_below_threshold() {
    let mut words = vec![
        word("hello", 0, 500_000),
        word("world", 900_000, 1_200_000), // 400ms gap
    ];
    let original_start = words[1].start_us;
    let count = trim_pauses(&mut words, DEFAULT_PAUSE_THRESHOLD_US, DEFAULT_MAX_GAP_US);
    assert_eq!(count, 0);
    assert_eq!(words[1].start_us, original_start);
}

#[test]
fn trim_handles_empty_and_single() {
    let mut empty: Vec<Word> = vec![];
    assert_eq!(
        trim_pauses(&mut empty, DEFAULT_PAUSE_THRESHOLD_US, DEFAULT_MAX_GAP_US),
        0
    );

    let mut single = vec![word("hello", 0, 500_000)];
    assert_eq!(
        trim_pauses(&mut single, DEFAULT_PAUSE_THRESHOLD_US, DEFAULT_MAX_GAP_US),
        0
    );
}

#[test]
fn trim_multiple_pauses_accumulate() {
    let mut words = vec![
        word("a", 0, 500_000),
        word("b", 2_500_000, 3_000_000), // 2s gap
        word("c", 5_000_000, 5_500_000), // 2s gap after b
    ];
    let count = trim_pauses(&mut words, DEFAULT_PAUSE_THRESHOLD_US, DEFAULT_MAX_GAP_US);
    assert_eq!(count, 2);
    let shift_each = 2_000_000 - DEFAULT_MAX_GAP_US; // 1_700_000
    assert_eq!(words[1].start_us, 2_500_000 - shift_each);
    assert_eq!(words[2].start_us, 5_000_000 - shift_each * 2);
}

// ── tighten_gaps ────────────────────────────────────────────────

#[test]
fn tighten_reduces_500ms_gap_to_250ms() {
    let mut words = vec![
        word("hello", 0, 500_000),
        word("world", 1_000_000, 1_500_000), // 500ms gap
    ];
    let count = tighten_gaps(&mut words, DEFAULT_TIGHTEN_TARGET_US);
    assert_eq!(count, 1);
    // gap was 500_000, target 250_000 → shift = 250_000
    assert_eq!(words[1].start_us, 750_000);
    assert_eq!(words[1].end_us, 1_250_000);
}

#[test]
fn tighten_ignores_gap_below_target() {
    let mut words = vec![
        word("hello", 0, 500_000),
        word("world", 700_000, 1_000_000), // 200ms gap, below 250ms target
    ];
    let original_start = words[1].start_us;
    let count = tighten_gaps(&mut words, DEFAULT_TIGHTEN_TARGET_US);
    assert_eq!(count, 0);
    assert_eq!(words[1].start_us, original_start);
}

#[test]
fn tighten_cumulative_shift() {
    let mut words = vec![
        word("a", 0, 500_000),
        word("b", 1_000_000, 1_500_000), // 500ms gap → excess 250ms
        word("c", 2_500_000, 3_000_000), // 1000ms gap → excess 750ms
    ];
    let count = tighten_gaps(&mut words, DEFAULT_TIGHTEN_TARGET_US);
    assert_eq!(count, 2);
    // b shifted by 250_000
    assert_eq!(words[1].start_us, 750_000);
    assert_eq!(words[1].end_us, 1_250_000);
    // c shifted by 250_000 + 750_000 = 1_000_000
    assert_eq!(words[2].start_us, 1_500_000);
    assert_eq!(words[2].end_us, 2_000_000);
}

#[test]
fn tighten_skips_deleted_words_for_gap_calc() {
    let mut words = vec![
        word("hello", 0, 500_000),
        deleted_word("um", 600_000, 700_000),
        word("world", 1_000_000, 1_500_000), // gap from hello.end (500k) to world.start (1M) = 500ms
    ];
    let count = tighten_gaps(&mut words, DEFAULT_TIGHTEN_TARGET_US);
    assert_eq!(count, 1);
    // excess = 500_000 - 250_000 = 250_000
    // deleted word at index 1 is before gap index (2), not shifted
    assert_eq!(words[1].start_us, 600_000);
    assert_eq!(words[2].start_us, 750_000);
}

#[test]
fn tighten_handles_empty_and_single() {
    let mut empty: Vec<Word> = vec![];
    assert_eq!(tighten_gaps(&mut empty, DEFAULT_TIGHTEN_TARGET_US), 0);

    let mut single = vec![word("hello", 0, 500_000)];
    assert_eq!(tighten_gaps(&mut single, DEFAULT_TIGHTEN_TARGET_US), 0);
}

#[test]
fn tighten_rejects_non_positive_target() {
    let mut words = vec![
        word("hello", 0, 500_000),
        word("world", 1_000_000, 1_500_000),
    ];
    assert_eq!(tighten_gaps(&mut words, 0), 0);
    assert_eq!(tighten_gaps(&mut words, -100), 0);
}

// ── cleanup cascade end-to-end ────────────────────────────────────

/// Full cleanup pipeline: detect fillers → delete → detect duplicates
/// (iteratively) → delete, then verify remaining text and keep-segments
/// contain no deleted-word regions.
#[test]
fn cleanup_cascade_produces_correct_keep_segments() {
    use crate::managers::editor::EditorState;

    // Transcript: "Yeah, so the um the the best best part about a lot
    // of this is how it can really transform the way you sound. And um
    // like the uh the the difference is gonna be noticeable kind of on
    // first use."
    let mut words = vec![
        word("Yeah,", 0, 400_000),                  // 0
        word("so", 500_000, 700_000),               // 1
        word("the", 800_000, 1_000_000),            // 2
        word("um", 1_100_000, 1_300_000),           // 3  ← filler
        word("the", 1_400_000, 1_600_000),          // 4  ← dup
        word("the", 1_700_000, 1_900_000),          // 5  ← dup
        word("best", 2_000_000, 2_200_000),         // 6
        word("best", 2_300_000, 2_500_000),         // 7  ← dup
        word("part", 2_600_000, 2_800_000),         // 8
        word("about", 2_900_000, 3_100_000),        // 9
        word("a", 3_200_000, 3_300_000),            // 10
        word("lot", 3_400_000, 3_600_000),          // 11
        word("of", 3_700_000, 3_800_000),           // 12
        word("this", 3_900_000, 4_100_000),         // 13
        word("is", 4_200_000, 4_400_000),           // 14
        word("how", 4_500_000, 4_700_000),          // 15
        word("it", 4_800_000, 4_900_000),           // 16
        word("can", 5_000_000, 5_200_000),          // 17
        word("really", 5_300_000, 5_500_000),       // 18
        word("transform", 5_600_000, 5_900_000),    // 19
        word("the", 6_000_000, 6_200_000),          // 20
        word("way", 6_300_000, 6_500_000),          // 21
        word("you", 6_600_000, 6_800_000),          // 22
        word("sound.", 6_900_000, 7_200_000),       // 23
        word("And", 7_400_000, 7_600_000),          // 24
        word("um", 7_700_000, 7_900_000),           // 25 ← filler
        word("like", 8_000_000, 8_200_000),         // 26 ← filler
        word("the", 8_300_000, 8_500_000),          // 27
        word("uh", 8_600_000, 8_800_000),           // 28 ← filler
        word("the", 8_900_000, 9_100_000),          // 29 ← dup
        word("the", 9_200_000, 9_400_000),          // 30 ← dup
        word("difference", 9_500_000, 9_900_000),   // 31
        word("is", 10_000_000, 10_200_000),         // 32
        word("gonna", 10_300_000, 10_500_000),      // 33
        word("be", 10_600_000, 10_800_000),         // 34
        word("noticeable", 10_900_000, 11_300_000), // 35
        word("kind", 11_400_000, 11_600_000),       // 36 ← filler (kind of)
        word("of", 11_700_000, 11_900_000),         // 37 ← filler (kind of)
        word("on", 12_000_000, 12_200_000),         // 38
        word("first", 12_300_000, 12_500_000),      // 39
        word("use.", 12_600_000, 12_800_000),       // 40
    ];

    let config = default_config();

    // Step 1: detect and delete fillers
    let fillers = detect_fillers(&words, &config);
    for &idx in &fillers {
        words[idx].deleted = true;
    }

    // Step 2: iteratively detect and delete duplicates
    loop {
        let dups = detect_duplicates(&words);
        if dups.is_empty() {
            break;
        }
        for &idx in &dups {
            words[idx].deleted = true;
        }
    }

    // Verify remaining (non-deleted) text
    let remaining: Vec<&str> = words
        .iter()
        .filter(|w| !w.deleted)
        .map(|w| w.text.as_str())
        .collect();

    assert_eq!(
        remaining,
        vec![
            "Yeah,",
            "the",
            "best",
            "part",
            "about",
            "a",
            "lot",
            "of",
            "this",
            "is",
            "how",
            "it",
            "can",
            "really",
            "transform",
            "the",
            "way",
            "you",
            "sound.",
            "And",
            "the",
            "difference",
            "is",
            "gonna",
            "be",
            "noticeable",
            "on",
            "first",
            "use.",
        ]
    );

    // Verify keep-segments exclude deleted word regions
    let mut editor = EditorState::new();
    editor.set_words(words.clone());
    // Replay deletions into the editor's words
    for (i, w) in words.iter().enumerate() {
        if w.deleted {
            editor.get_words_mut()[i].deleted = true;
        }
    }
    let segments = editor.get_keep_segments();

    // Every segment must only span non-deleted word time ranges
    let deleted_ranges: Vec<(i64, i64)> = words
        .iter()
        .filter(|w| w.deleted)
        .map(|w| (w.start_us, w.end_us))
        .collect();

    for (seg_start, seg_end) in &segments {
        for (del_start, del_end) in &deleted_ranges {
            // A deleted word's range must not be fully contained in a keep-segment
            let overlaps = del_start >= seg_start && del_end <= seg_end;
            assert!(
                !overlaps,
                "keep-segment ({seg_start}–{seg_end}) contains deleted word ({del_start}–{del_end})"
            );
        }
    }

    // Sanity: we should have at least 2 segments (gap around deleted regions)
    assert!(!segments.is_empty(), "expected non-empty keep-segments");
}


// ---------------------------- R-004 --------------------------------

#[test]
fn classify_gap_unknown_without_curve() {
    assert_eq!(classify_gap(0, 1_000_000, &[]), GapClassification::Unknown);
}

#[test]
fn classify_gap_true_silence_below_threshold() {
    // 10 frames × 30ms = 300ms curve, all well below GAP_SILENCE_THRESHOLD.
    let curve = vec![0.05f32; 10];
    assert_eq!(
        classify_gap(0, 300_000, &curve),
        GapClassification::TrueSilence,
    );
}

#[test]
fn classify_gap_missed_speech_above_threshold() {
    let curve = vec![0.9f32; 10];
    assert_eq!(
        classify_gap(0, 300_000, &curve),
        GapClassification::MissedSpeech,
    );
}

#[test]
fn classify_gap_non_speech_acoustic_in_middle_band() {
    let curve = vec![0.3f32; 10];
    assert_eq!(
        classify_gap(0, 300_000, &curve),
        GapClassification::NonSpeechAcoustic,
    );
}

#[test]
fn classify_pauses_maps_one_to_one_with_empty_curve() {
    let words = vec![
        word("a", 0, 200_000),
        word("b", 2_000_000, 2_200_000),
    ];
    let config = FillerConfig::default();
    let pauses = detect_pauses(&words, &config);
    let classified = classify_pauses(&pauses, &words, &[]);
    assert_eq!(classified.len(), pauses.len());
    for (_, _, class) in &classified {
        assert_eq!(*class, GapClassification::Unknown);
    }
}
