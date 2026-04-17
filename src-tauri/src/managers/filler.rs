/// Filler word and pause detection for transcript editing.
///
/// Analyzes a word list to identify filler words (e.g., "um", "uh", "like")
/// and long pauses between words. Results can drive bulk-delete suggestions
/// in the editor UI.
use crate::managers::editor::Word;

/// Default filler words (English). Must stay in sync with
/// `DEFAULT_DISCARD_WORDS` in `src/components/settings/DiscardWords.tsx`.
pub const DEFAULT_FILLERS: &[&str] = &[
    "um",
    "uh",
    "uh huh",
    "hmm",
    "mm",
    "mhm",
    "er",
    "ah",
    "like",
    "you know",
    "I mean",
    "basically",
    "actually",
    "literally",
    "so",
    "right",
    "kind of",
    "sort of",
];

/// Minimum gap between words (in microseconds) to be considered a pause.
pub const DEFAULT_PAUSE_THRESHOLD_US: i64 = 1_500_000; // 1.5 seconds

/// Configuration for filler/pause detection.
pub struct FillerConfig {
    /// Words to treat as fillers (matched case-insensitively, punctuation stripped).
    pub filler_words: Vec<String>,
    /// Gap in microseconds that qualifies as a "long pause".
    pub pause_threshold_us: i64,
    /// If true, detected fillers are automatically marked deleted.
    #[allow(dead_code)]
    pub auto_delete_fillers: bool,
    /// If true, detected pauses are automatically marked silenced.
    #[allow(dead_code)]
    pub auto_silence_pauses: bool,
}

impl Default for FillerConfig {
    fn default() -> Self {
        Self {
            filler_words: DEFAULT_FILLERS.iter().map(|s| s.to_string()).collect(),
            pause_threshold_us: DEFAULT_PAUSE_THRESHOLD_US,
            auto_delete_fillers: false,
            auto_silence_pauses: false,
        }
    }
}

/// Results from analyzing a word list for fillers and pauses.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResult {
    /// Indices of words identified as fillers.
    pub filler_indices: Vec<usize>,
    /// `(gap_after_word_index, gap_duration_us)` for each detected pause.
    pub pauses: Vec<(usize, i64)>,
    /// Indices of the second word in each adjacent duplicate pair.
    pub duplicate_indices: Vec<usize>,
}

/// Strip leading/trailing punctuation from a word, returning a lowercase copy.
fn normalize(word: &str) -> String {
    word.trim_matches(|c: char| c.is_ascii_punctuation())
        .to_lowercase()
}

/// Normalize filler word for fuzzy matching.
/// "umm" → "um", "uhhh" → "uh", "hmmm" → "hm", "ummmmm" → "um"
fn normalize_filler(word: &str) -> String {
    let lower = word.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    if chars.len() < 2 {
        return lower;
    }
    // Collapse trailing runs of the same character to a single instance
    let last_char = *chars.last().unwrap();
    let mut end = chars.len();
    while end > 1 && chars[end - 2] == last_char {
        end -= 1;
    }
    chars[..end].iter().collect()
}

/// Detect adjacent duplicate words (case-insensitive).
/// Returns indices of the SECOND word in each duplicate pair.
/// "the the best best part" → returns indices of second "the" and second "best"
pub fn detect_duplicates(words: &[Word]) -> Vec<usize> {
    let mut duplicates = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if words[i].deleted {
            i += 1;
            continue;
        }
        // Look for the next non-deleted word
        let mut j = i + 1;
        while j < words.len() && words[j].deleted {
            j += 1;
        }
        if j < words.len() && words[i].text.to_lowercase() == words[j].text.to_lowercase() {
            duplicates.push(j);
            // Continue from after the duplicate to catch triples: "the the the" → [1, 2]
            i = j;
        } else {
            i = j;
        }
    }
    duplicates
}

/// Detect filler words in the word list. Returns indices of filler words.
///
/// Multi-word fillers (e.g., "you know") are detected by joining consecutive
/// non-deleted words and checking for a match. When a multi-word filler is
/// found, all constituent word indices are included in the result.
pub fn detect_fillers(words: &[Word], config: &FillerConfig) -> Vec<usize> {
    let mut indices: Vec<usize> = Vec::new();

    // Pre-compute the maximum token count among filler phrases.
    let max_filler_tokens = config
        .filler_words
        .iter()
        .map(|f| f.split_whitespace().count())
        .max()
        .unwrap_or(0);

    // Build a set of normalized filler phrases for fast lookup.
    // For single-word fillers, also store a fuzzy-normalized form.
    let filler_set: Vec<(String, String, usize)> = config
        .filler_words
        .iter()
        .map(|f| {
            let lower = f.to_lowercase();
            let fuzzy = if f.split_whitespace().count() == 1 {
                normalize_filler(&lower)
            } else {
                lower.clone()
            };
            (lower, fuzzy, f.split_whitespace().count())
        })
        .collect();

    // Collect indices of non-deleted words so we can walk them in order.
    let active: Vec<usize> = words
        .iter()
        .enumerate()
        .filter(|(_, w)| !w.deleted)
        .map(|(i, _)| i)
        .collect();

    let mut skip_until = 0usize; // active-array index to skip to (for multi-word matches)

    for (ai, &wi) in active.iter().enumerate() {
        if ai < skip_until {
            continue;
        }

        // Try longest filler phrases first so "you know" beats "you".
        let mut matched = false;
        for window in (1..=max_filler_tokens).rev() {
            if ai + window > active.len() {
                continue;
            }
            let phrase: String = (0..window)
                .map(|offset| normalize(&words[active[ai + offset]].text))
                .collect::<Vec<_>>()
                .join(" ");

            if filler_set
                .iter()
                .any(|(exact, fuzzy, len)| {
                    if *len != window {
                        return false;
                    }
                    if *exact == phrase {
                        return true;
                    }
                    // Fuzzy match for single-word fillers
                    if window == 1 {
                        let norm_phrase = normalize_filler(&phrase);
                        return *fuzzy == norm_phrase;
                    }
                    false
                })
            {
                for offset in 0..window {
                    indices.push(active[ai + offset]);
                }
                skip_until = ai + window;
                matched = true;
                break;
            }
        }

        if !matched {
            // Single-word check (already covered by window==1 above, but kept
            // explicit for clarity).
            let norm = normalize(&words[wi].text);
            let fuzzy_norm = normalize_filler(&norm);
            if filler_set.iter().any(|(exact, fuzzy, len)| {
                *len == 1 && (*exact == norm || *fuzzy == fuzzy_norm)
            }) {
                indices.push(wi);
            }
        }
    }

    indices.sort_unstable();
    indices.dedup();
    indices
}

/// Detect long pauses between words. Returns `(gap_after_word_index, gap_duration_us)`.
///
/// Only considers non-deleted words when measuring gaps.
pub fn detect_pauses(words: &[Word], config: &FillerConfig) -> Vec<(usize, i64)> {
    let active: Vec<usize> = words
        .iter()
        .enumerate()
        .filter(|(_, w)| !w.deleted)
        .map(|(i, _)| i)
        .collect();

    let mut pauses = Vec::new();
    for pair in active.windows(2) {
        let (i, j) = (pair[0], pair[1]);
        let gap = words[j].start_us - words[i].end_us;
        if gap >= config.pause_threshold_us {
            pauses.push((i, gap));
        }
    }
    pauses
}

/// Default maximum gap after trimming (300 ms).
pub const DEFAULT_MAX_GAP_US: i64 = 300_000;

/// Trim long pauses by reducing gaps to `max_gap_us`.
///
/// Walks the word list and, for every gap between non-deleted words that
/// exceeds `pause_threshold_us`, trims the excess beyond `max_gap_us` by
/// shifting all subsequent word timestamps earlier. Deleted words between
/// pauses are shifted along with everything else so their timing stays
/// consistent.
///
/// Returns the number of pauses trimmed.
pub fn trim_pauses(words: &mut [Word], pause_threshold_us: i64, max_gap_us: i64) -> usize {
    if words.len() < 2 {
        return 0;
    }

    // First pass: find gaps that exceed the threshold and compute excess.
    // Each entry is (index_of_word_after_gap, excess_to_remove).
    let mut gaps: Vec<(usize, i64)> = Vec::new();

    let mut prev_end: Option<i64> = None;
    for (i, word) in words.iter().enumerate() {
        if word.deleted {
            continue;
        }
        if let Some(pe) = prev_end {
            let gap = word.start_us - pe;
            if gap >= pause_threshold_us {
                let excess = gap - max_gap_us;
                if excess > 0 {
                    gaps.push((i, excess));
                }
            }
        }
        prev_end = Some(word.end_us);
    }

    if gaps.is_empty() {
        return 0;
    }

    let count = gaps.len();

    // Second pass: apply cumulative shift to all words at or after each gap.
    let mut gap_idx = 0;
    let mut cumulative_shift: i64 = 0;

    for (i, word) in words.iter_mut().enumerate() {
        while gap_idx < gaps.len() && gaps[gap_idx].0 <= i {
            cumulative_shift += gaps[gap_idx].1;
            gap_idx += 1;
        }

        if cumulative_shift > 0 {
            word.start_us -= cumulative_shift;
            word.end_us -= cumulative_shift;
        }
    }

    count
}

/// Default target gap duration after tightening (250ms).
pub const DEFAULT_TIGHTEN_TARGET_US: i64 = 250_000;

/// Tighten all inter-word gaps to a maximum target duration.
/// Unlike trim_pauses (which only handles very long pauses), this
/// shortens ALL gaps exceeding the target — creating a tighter pace.
/// Returns the number of gaps shortened.
pub fn tighten_gaps(words: &mut [Word], target_gap_us: i64) -> usize {
    if words.len() < 2 || target_gap_us <= 0 {
        return 0;
    }

    let mut gaps: Vec<(usize, i64)> = Vec::new();
    let mut prev_end: Option<(usize, i64)> = None;

    for (i, word) in words.iter().enumerate() {
        if word.deleted {
            continue;
        }
        if let Some((_, pe)) = prev_end {
            let gap = word.start_us - pe;
            if gap > target_gap_us {
                gaps.push((i, gap - target_gap_us));
            }
        }
        prev_end = Some((i, word.end_us));
    }

    if gaps.is_empty() {
        return 0;
    }

    let count = gaps.len();
    let mut gap_idx = 0;
    let mut cumulative_shift: i64 = 0;

    for (i, word) in words.iter_mut().enumerate() {
        while gap_idx < gaps.len() && gaps[gap_idx].0 <= i {
            cumulative_shift += gaps[gap_idx].1;
            gap_idx += 1;
        }
        if cumulative_shift > 0 {
            word.start_us -= cumulative_shift;
            word.end_us -= cumulative_shift;
        }
    }

    count
}

/// Analyze words and return fillers, pauses, and duplicates.
#[cfg(test)]
pub fn analyze(words: &[Word], config: &FillerConfig) -> AnalysisResult {
    AnalysisResult {
        filler_indices: detect_fillers(words, config),
        pauses: detect_pauses(words, config),
        duplicate_indices: detect_duplicates(words),
    }
}

#[cfg(test)]
mod tests {
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
            word("um", 300_000, 500_000),  // filler
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
        assert_eq!(trim_pauses(&mut empty, DEFAULT_PAUSE_THRESHOLD_US, DEFAULT_MAX_GAP_US), 0);

        let mut single = vec![word("hello", 0, 500_000)];
        assert_eq!(trim_pauses(&mut single, DEFAULT_PAUSE_THRESHOLD_US, DEFAULT_MAX_GAP_US), 0);
    }

    #[test]
    fn trim_multiple_pauses_accumulate() {
        let mut words = vec![
            word("a", 0, 500_000),
            word("b", 2_500_000, 3_000_000),    // 2s gap
            word("c", 5_000_000, 5_500_000),    // 2s gap after b
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
            word("b", 1_000_000, 1_500_000),  // 500ms gap → excess 250ms
            word("c", 2_500_000, 3_000_000),  // 1000ms gap → excess 750ms
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
            word("Yeah,", 0, 400_000),           // 0
            word("so", 500_000, 700_000),         // 1
            word("the", 800_000, 1_000_000),      // 2
            word("um", 1_100_000, 1_300_000),     // 3  ← filler
            word("the", 1_400_000, 1_600_000),    // 4  ← dup
            word("the", 1_700_000, 1_900_000),    // 5  ← dup
            word("best", 2_000_000, 2_200_000),   // 6
            word("best", 2_300_000, 2_500_000),   // 7  ← dup
            word("part", 2_600_000, 2_800_000),   // 8
            word("about", 2_900_000, 3_100_000),  // 9
            word("a", 3_200_000, 3_300_000),      // 10
            word("lot", 3_400_000, 3_600_000),    // 11
            word("of", 3_700_000, 3_800_000),     // 12
            word("this", 3_900_000, 4_100_000),   // 13
            word("is", 4_200_000, 4_400_000),     // 14
            word("how", 4_500_000, 4_700_000),    // 15
            word("it", 4_800_000, 4_900_000),     // 16
            word("can", 5_000_000, 5_200_000),    // 17
            word("really", 5_300_000, 5_500_000), // 18
            word("transform", 5_600_000, 5_900_000), // 19
            word("the", 6_000_000, 6_200_000),    // 20
            word("way", 6_300_000, 6_500_000),    // 21
            word("you", 6_600_000, 6_800_000),    // 22
            word("sound.", 6_900_000, 7_200_000), // 23
            word("And", 7_400_000, 7_600_000),    // 24
            word("um", 7_700_000, 7_900_000),     // 25 ← filler
            word("like", 8_000_000, 8_200_000),   // 26 ← filler
            word("the", 8_300_000, 8_500_000),    // 27
            word("uh", 8_600_000, 8_800_000),     // 28 ← filler
            word("the", 8_900_000, 9_100_000),    // 29 ← dup
            word("the", 9_200_000, 9_400_000),    // 30 ← dup
            word("difference", 9_500_000, 9_900_000), // 31
            word("is", 10_000_000, 10_200_000),   // 32
            word("gonna", 10_300_000, 10_500_000), // 33
            word("be", 10_600_000, 10_800_000),   // 34
            word("noticeable", 10_900_000, 11_300_000), // 35
            word("kind", 11_400_000, 11_600_000), // 36 ← filler (kind of)
            word("of", 11_700_000, 11_900_000),   // 37 ← filler (kind of)
            word("on", 12_000_000, 12_200_000),   // 38
            word("first", 12_300_000, 12_500_000), // 39
            word("use.", 12_600_000, 12_800_000), // 40
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
                "Yeah,", "the", "best", "part", "about", "a", "lot",
                "of", "this", "is", "how", "it", "can", "really", "transform",
                "the", "way", "you", "sound.", "And", "the", "difference",
                "is", "gonna", "be", "noticeable", "on", "first", "use.",
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
        assert!(
            !segments.is_empty(),
            "expected non-empty keep-segments"
        );
    }
}
