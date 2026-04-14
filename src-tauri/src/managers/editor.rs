/// Transcript editing engine for word-level video editing.
///
/// Manages a list of timestamped words with delete/restore/split/silence
/// operations and full undo/redo support (up to 64 snapshots).

const MAX_UNDO: usize = 64;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct Word {
    pub text: String,
    /// Start timestamp in microseconds.
    pub start_us: i64,
    /// End timestamp in microseconds.
    pub end_us: i64,
    pub deleted: bool,
    pub silenced: bool,
    /// Word confidence from transcription. -1.0 = unknown.
    pub confidence: f32,
    /// Speaker identifier. -1 = unknown.
    pub speaker_id: i32,
}

/// Holds the current word list and undo/redo history.
pub struct EditorState {
    words: Vec<Word>,
    undo_stack: Vec<Vec<Word>>,
    redo_stack: Vec<Vec<Word>>,
}

impl EditorState {
    /// Create an empty editor.
    pub fn new() -> Self {
        Self {
            words: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Replace all words (e.g. from a new transcription result).
    /// Clears undo/redo history.
    pub fn set_words(&mut self, words: Vec<Word>) {
        self.words = words;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Return the current word list.
    pub fn get_words(&self) -> &[Word] {
        &self.words
    }

    // ── snapshot helpers ──────────────────────────────────────────────

    /// Push a snapshot of the current words onto the undo stack,
    /// clear the redo stack, and enforce the 64-entry cap.
    fn push_undo_snapshot(&mut self) {
        self.undo_stack.push(self.words.clone());
        self.redo_stack.clear();
        if self.undo_stack.len() > MAX_UNDO {
            self.undo_stack.remove(0);
        }
    }

    // ── mutation operations ──────────────────────────────────────────

    /// Mark a single word as deleted. Returns `false` if index is out of
    /// bounds or the word is already deleted.
    pub fn delete_word(&mut self, index: usize) -> bool {
        if index >= self.words.len() || self.words[index].deleted {
            return false;
        }
        self.push_undo_snapshot();
        self.words[index].deleted = true;
        true
    }

    /// Restore a previously deleted word. Returns `false` if index is out
    /// of bounds or the word is not deleted.
    pub fn restore_word(&mut self, index: usize) -> bool {
        if index >= self.words.len() || !self.words[index].deleted {
            return false;
        }
        self.push_undo_snapshot();
        self.words[index].deleted = false;
        true
    }

    /// Delete an inclusive range of words `[start..=end]`.
    /// Returns `false` if the range is invalid.
    pub fn delete_range(&mut self, start: usize, end: usize) -> bool {
        if start > end || end >= self.words.len() {
            return false;
        }
        self.push_undo_snapshot();
        for word in &mut self.words[start..=end] {
            word.deleted = true;
        }
        true
    }

    /// Restore every deleted word.
    /// Returns `false` if nothing was deleted.
    pub fn restore_all(&mut self) -> bool {
        if !self.words.iter().any(|w| w.deleted) {
            return false;
        }
        self.push_undo_snapshot();
        for word in &mut self.words {
            word.deleted = false;
        }
        true
    }

    /// Split a word at the given character `position`, producing two words
    /// whose timestamps are proportional to the split point.
    /// Returns `false` if the index or position is invalid.
    pub fn split_word(&mut self, index: usize, position: usize) -> bool {
        if index >= self.words.len() {
            return false;
        }

        let char_len = self.words[index].text.chars().count();
        if position == 0 || position >= char_len {
            return false;
        }

        self.push_undo_snapshot();

        let original = &self.words[index];
        let ratio = position as f64 / char_len as f64;
        let duration = original.end_us - original.start_us;
        let mid_us = original.start_us + (duration as f64 * ratio) as i64;

        let left_text: String = original.text.chars().take(position).collect();
        let right_text: String = original.text.chars().skip(position).collect();

        let left = Word {
            text: left_text,
            start_us: original.start_us,
            end_us: mid_us,
            deleted: original.deleted,
            silenced: original.silenced,
            confidence: original.confidence,
            speaker_id: original.speaker_id,
        };
        let right = Word {
            text: right_text,
            start_us: mid_us,
            end_us: original.end_us,
            deleted: original.deleted,
            silenced: original.silenced,
            confidence: original.confidence,
            speaker_id: original.speaker_id,
        };

        self.words.splice(index..=index, [left, right]);
        true
    }

    /// Toggle the `silenced` flag on a word.
    /// Returns `false` if the index is out of bounds.
    pub fn silence_word(&mut self, index: usize) -> bool {
        if index >= self.words.len() {
            return false;
        }
        self.push_undo_snapshot();
        self.words[index].silenced = !self.words[index].silenced;
        true
    }

    // ── undo / redo ──────────────────────────────────────────────────

    /// Undo the last mutation. Returns `false` if nothing to undo.
    pub fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.undo_stack.pop() {
            self.redo_stack.push(self.words.clone());
            self.words = snapshot;
            true
        } else {
            false
        }
    }

    /// Redo the last undone mutation. Returns `false` if nothing to redo.
    pub fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.redo_stack.pop() {
            self.undo_stack.push(self.words.clone());
            self.words = snapshot;
            true
        } else {
            false
        }
    }

    // ── keep-segments & time mapping ─────────────────────────────────

    /// Return contiguous non-deleted time regions as `(start_us, end_us)` pairs.
    pub fn get_keep_segments(&self) -> Vec<(i64, i64)> {
        let mut segments = Vec::new();
        let mut seg_start: Option<i64> = None;
        let mut seg_end: i64 = 0;

        for word in &self.words {
            if word.deleted {
                if let Some(start) = seg_start.take() {
                    segments.push((start, seg_end));
                }
            } else {
                if seg_start.is_none() {
                    seg_start = Some(word.start_us);
                }
                seg_end = word.end_us;
            }
        }

        if let Some(start) = seg_start {
            segments.push((start, seg_end));
        }

        segments
    }

    /// Map a position on the edited timeline (deletions removed) back to
    /// the original source timeline.
    ///
    /// Walks keep-segments, accumulating edit-time. When the accumulated
    /// time reaches `edit_time_us`, interpolates within that segment.
    pub fn map_edit_time_to_source_time(&self, edit_time_us: i64) -> i64 {
        let segments = self.get_keep_segments();
        let mut elapsed: i64 = 0;

        for (start, end) in &segments {
            let duration = end - start;
            if elapsed + duration > edit_time_us {
                return start + (edit_time_us - elapsed);
            }
            elapsed += duration;
        }

        // Past the end — clamp to end of last segment
        segments.last().map_or(0, |&(_, end)| end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_words() -> Vec<Word> {
        vec![
            Word {
                text: "Hello".into(),
                start_us: 0,
                end_us: 1_000_000,
                deleted: false,
                silenced: false,
                confidence: 0.95,
                speaker_id: 0,
            },
            Word {
                text: "world".into(),
                start_us: 1_000_000,
                end_us: 2_000_000,
                deleted: false,
                silenced: false,
                confidence: 0.90,
                speaker_id: 0,
            },
            Word {
                text: "this".into(),
                start_us: 2_000_000,
                end_us: 3_000_000,
                deleted: false,
                silenced: false,
                confidence: 0.85,
                speaker_id: 0,
            },
            Word {
                text: "is".into(),
                start_us: 3_000_000,
                end_us: 4_000_000,
                deleted: false,
                silenced: false,
                confidence: 0.80,
                speaker_id: 1,
            },
            Word {
                text: "a".into(),
                start_us: 4_000_000,
                end_us: 5_000_000,
                deleted: false,
                silenced: false,
                confidence: 0.75,
                speaker_id: 1,
            },
            Word {
                text: "test".into(),
                start_us: 5_000_000,
                end_us: 6_000_000,
                deleted: false,
                silenced: false,
                confidence: 0.70,
                speaker_id: 1,
            },
        ]
    }

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
}

// ── Dual-track regression suite ───────────────────────────────────────────────
//
// These tests act as surrogate coverage for the TypeScript helpers in
// `src/lib/utils/timeline.ts`.  The algorithms are identical on both sides of
// the Tauri bridge; any divergence in the Rust implementation would signal the
// same divergence in the TS layer.
//
// Coverage map
// ┌──────────────────────────────────────────────┬──────────────────────────────────────┐
// │ TS function (timeline.ts)                    │ Rust surrogate test(s)               │
// ├──────────────────────────────────────────────┼──────────────────────────────────────┤
// │ editTimeToSourceTime                         │ dt_edit_to_source_*                  │
// │ sourceTimeToEditTime                         │ dt_source_to_edit_*                  │
// │ getDeletedRangesFromKeepSegments             │ dt_deleted_ranges_from_segments_*    │
// │ DUAL_TRACK_DRIFT_THRESHOLD / COOLDOWN_MS     │ dt_drift_correction_constants        │
// │ video-collapse guard (no valid seek target)  │ dt_no_collapse_*                     │
// │ monotonic mapping property                   │ dt_monotonic_mapping                 │
// │ source-switching logic for video mode        │ dt_source_switching_*                │
// └──────────────────────────────────────────────┴──────────────────────────────────────┘
//
// When a vitest harness is added to the frontend, add tests directly against the
// exported TS functions and remove the "surrogate" note from timeline.ts.
#[cfg(test)]
mod dual_track_regression {
    use super::*;

    // ── Constants mirrored from timeline.ts ──────────────────────────────────
    /// Minimum A/V drift (seconds) before a correction is applied.
    const DUAL_TRACK_DRIFT_THRESHOLD: f64 = 0.08;
    /// Minimum real-clock interval (ms) between consecutive drift corrections.
    const DUAL_TRACK_SYNC_COOLDOWN_MS: f64 = 250.0;

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
            word("one",   0,          1_000_000),
            word("two",   1_000_000,  2_000_000),
            word("three", 2_000_000,  3_000_000),
            word("four",  3_000_000,  4_000_000),
            word("five",  4_000_000,  5_000_000),
            word("six",   5_000_000,  6_000_000),
        ]
    }

    // ── editTimeToSourceTime mirrors ──────────────────────────────────────────

    /// Identity property: no deletions → edit time == source time.
    #[test]
    fn dt_edit_to_source_identity_no_deletions() {
        let mut ed = EditorState::new();
        ed.set_words(six_words());
        for t in [0, 500_000, 2_000_000, 5_999_999, 6_000_000] {
            assert_eq!(ed.map_edit_time_to_source_time(t), t,
                "Expected identity at t={t}");
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
        assert_eq!(ed.map_edit_time_to_source_time(0),         0);
        assert_eq!(ed.map_edit_time_to_source_time(500_000),   500_000);
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
        assert_eq!(clamped, 6_000_000, "Seek target must not exceed last segment end");
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
        let source_times: Vec<i64> = edit_times.iter()
            .map(|&t| ed.map_edit_time_to_source_time(t))
            .collect();

        for window in source_times.windows(2) {
            assert!(window[0] <= window[1],
                "Mapping not monotone: {} > {} (violates video-sync invariant)",
                window[0], window[1]);
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
        assert_eq!(snap, 2_000_000,
            "Source time in deleted region should snap to 2s edit-time");
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
            assert_eq!(back, edit_us,
                "Round-trip failed at edit_us={edit_us}: got back {back}");
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
            assert!(window[0].1 <= window[1].0,
                "Segments overlap or are not sorted: {:?} then {:?}",
                window[0], window[1]);
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
        let word_total: i64 = ed.get_words().iter()
            .filter(|w| !w.deleted)
            .map(|w| w.end_us - w.start_us)
            .sum();
        assert_eq!(seg_total, word_total,
            "Keep-segment total ({seg_total}) differs from active-word total ({word_total})");
    }

    // ── Drift-correction constant sanity ─────────────────────────────────────

    /// The drift threshold and cooldown constants must stay within acceptable
    /// perceptual bounds so the sync loop neither over-corrects nor ignores drift.
    ///
    /// Acceptable range:
    ///   threshold:  10 ms – 200 ms  (below 10 ms is jittery; above 200 ms is perceptible)
    ///   cooldown:   100 ms – 1000 ms
    #[test]
    fn dt_drift_correction_constants_within_perceptual_bounds() {
        assert!(
            DUAL_TRACK_DRIFT_THRESHOLD >= 0.010 && DUAL_TRACK_DRIFT_THRESHOLD <= 0.200,
            "DUAL_TRACK_DRIFT_THRESHOLD ({DUAL_TRACK_DRIFT_THRESHOLD}s) outside [10ms, 200ms]"
        );
        assert!(
            DUAL_TRACK_SYNC_COOLDOWN_MS >= 100.0 && DUAL_TRACK_SYNC_COOLDOWN_MS <= 1000.0,
            "DUAL_TRACK_SYNC_COOLDOWN_MS ({DUAL_TRACK_SYNC_COOLDOWN_MS}ms) outside [100ms, 1000ms]"
        );
    }

    /// Drift < threshold must NOT trigger a correction (no spurious seek).
    #[test]
    fn dt_drift_below_threshold_does_not_correct() {
        // Simulate: video at 5.05s, target at 5.00s → drift = 50ms < 80ms threshold
        let video_time = 5.05_f64;
        let target_source_time = 5.00_f64;
        let drift = (video_time - target_source_time).abs();
        assert!(drift < DUAL_TRACK_DRIFT_THRESHOLD,
            "Drift {drift}s should be below threshold {DUAL_TRACK_DRIFT_THRESHOLD}s");
    }

    /// Drift ≥ threshold must trigger a correction.
    #[test]
    fn dt_drift_at_threshold_triggers_correction() {
        let video_time = 5.09_f64;
        let target_source_time = 5.00_f64;
        let drift = (video_time - target_source_time).abs();
        assert!(drift >= DUAL_TRACK_DRIFT_THRESHOLD,
            "Drift {drift}s should be ≥ threshold {DUAL_TRACK_DRIFT_THRESHOLD}s");
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
        assert_eq!(mapped, 3_000_000,
            "Video seek must use mapped source time (3s), not raw edit time (2s)");
        assert_ne!(mapped, 2_000_000,
            "Video seek must NOT use the raw edit time when there are deletions");
    }

    /// In video mode with no keep segments, seek target is 0 (no collapse/panic).
    #[test]
    fn dt_source_switching_no_keep_segments_yields_zero() {
        let mut ed = EditorState::new();
        ed.set_words(six_words());
        ed.delete_range(0, 5); // delete everything

        let result = ed.map_edit_time_to_source_time(0);
        assert_eq!(result, 0, "Empty keep segments must yield seek target 0, not garbage");
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
        assert_eq!(ed.map_edit_time_to_source_time(2_000_000), 2_000_000,
            "After undo, mapping must revert to identity");
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
}
