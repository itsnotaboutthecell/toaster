/// Filler word and pause detection for transcript editing.
///
/// Analyzes a word list to identify filler words (e.g., "um", "uh", "like")
/// and long pauses between words. Results can drive bulk-delete suggestions
/// in the editor UI.

use crate::managers::editor::Word;

/// Default filler words (English). Expand later via settings.
const DEFAULT_FILLERS: &[&str] = &[
    "um", "uh", "er", "ah", "like", "you know", "so", "basically",
    "actually", "literally", "right", "okay", "well", "I mean",
];

/// Minimum gap between words (in microseconds) to be considered a pause.
const DEFAULT_PAUSE_THRESHOLD_US: i64 = 1_500_000; // 1.5 seconds

/// Configuration for filler/pause detection.
pub struct FillerConfig {
    /// Words to treat as fillers (matched case-insensitively, punctuation stripped).
    pub filler_words: Vec<String>,
    /// Gap in microseconds that qualifies as a "long pause".
    pub pause_threshold_us: i64,
    /// If true, detected fillers are automatically marked deleted.
    pub auto_delete_fillers: bool,
    /// If true, detected pauses are automatically marked silenced.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResult {
    /// Indices of words identified as fillers.
    pub filler_indices: Vec<usize>,
    /// `(gap_after_word_index, gap_duration_us)` for each detected pause.
    pub pauses: Vec<(usize, i64)>,
}

/// Strip leading/trailing punctuation from a word, returning a lowercase copy.
fn normalize(word: &str) -> String {
    word.trim_matches(|c: char| c.is_ascii_punctuation())
        .to_lowercase()
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
    let filler_set: Vec<(String, usize)> = config
        .filler_words
        .iter()
        .map(|f| (f.to_lowercase(), f.split_whitespace().count()))
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

            if filler_set.iter().any(|(f, len)| *len == window && *f == phrase) {
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
            if filler_set.iter().any(|(f, len)| *len == 1 && *f == norm) {
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

/// Analyze words and return both fillers and pauses.
pub fn analyze(words: &[Word], config: &FillerConfig) -> AnalysisResult {
    AnalysisResult {
        filler_indices: detect_fillers(words, config),
        pauses: detect_pauses(words, config),
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
        let words = vec![
            word("hello", 0, 500_000),
            word("world", 600_000, 1_000_000),
        ];
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
            word("um", 0, 500_000),     // not in custom list
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
            word("so", 0, 200_000),
            word("um", 300_000, 500_000),
            word("hello", 600_000, 1_000_000),
            word("world", 3_000_000, 3_500_000), // 2s gap after "hello"
        ];
        let result = analyze(&words, &default_config());
        assert_eq!(result.filler_indices, vec![0, 1]);
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
            }
        );
    }
}
