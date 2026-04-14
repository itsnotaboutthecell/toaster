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
            Word { text: "Hello".into(),  start_us: 0,       end_us: 1_000_000, deleted: false, silenced: false, confidence: 0.95, speaker_id: 0 },
            Word { text: "world".into(),  start_us: 1_000_000, end_us: 2_000_000, deleted: false, silenced: false, confidence: 0.90, speaker_id: 0 },
            Word { text: "this".into(),   start_us: 2_000_000, end_us: 3_000_000, deleted: false, silenced: false, confidence: 0.85, speaker_id: 0 },
            Word { text: "is".into(),     start_us: 3_000_000, end_us: 4_000_000, deleted: false, silenced: false, confidence: 0.80, speaker_id: 1 },
            Word { text: "a".into(),      start_us: 4_000_000, end_us: 5_000_000, deleted: false, silenced: false, confidence: 0.75, speaker_id: 1 },
            Word { text: "test".into(),   start_us: 5_000_000, end_us: 6_000_000, deleted: false, silenced: false, confidence: 0.70, speaker_id: 1 },
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
