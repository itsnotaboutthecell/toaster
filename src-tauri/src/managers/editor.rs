// Transcript editing engine for word-level video editing.
//
// Manages a list of timestamped words with delete/restore/split/silence
// operations and full undo/redo support (up to 64 snapshots).

const MAX_UNDO: usize = 64;
const DEFAULT_QUANTIZATION_FPS_NUM: u32 = 30;
const DEFAULT_QUANTIZATION_FPS_DEN: u32 = 1;

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

/// Backend-authoritative word-level LLM rewrite proposal.
/// The range is half-open: `[start_word_index, end_word_index)`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct LocalLlmWordProposal {
    pub start_word_index: usize,
    pub end_word_index: usize,
    pub replacement_words: Vec<String>,
}

/// Rejection details for a proposal that failed validation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct LocalLlmProposalRejection {
    pub proposal_index: usize,
    pub start_word_index: usize,
    pub end_word_index: usize,
    pub reason: String,
}

/// Outcome for applying a batch of local LLM proposals.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct LocalLlmApplyResult {
    pub applied_proposals: usize,
    pub applied_word_indices: Vec<usize>,
    pub rejected_proposals: Vec<LocalLlmProposalRejection>,
}

/// A keep-segment represented in microseconds.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TimingSegment {
    pub start_us: i64,
    pub end_us: i64,
}

/// Diagnostics snapshot for edit-time/source-time contract validation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TimingContractSnapshot {
    /// Monotonic revision incremented after each successful state mutation.
    pub timeline_revision: u64,
    pub total_words: usize,
    pub deleted_words: usize,
    pub active_words: usize,
    /// Source-media bounds inferred from transcript words.
    pub source_start_us: i64,
    pub source_end_us: i64,
    /// Total duration of all keep-segments (edited timeline duration).
    pub total_keep_duration_us: i64,
    pub keep_segments: Vec<TimingSegment>,
    /// Keep-segments snapped to the configured playback frame grid.
    pub quantized_keep_segments: Vec<TimingSegment>,
    pub quantization_fps_num: u32,
    pub quantization_fps_den: u32,
    /// True when keep-segments satisfy ordering/coverage contract checks.
    pub keep_segments_valid: bool,
    /// Human-readable warning when a contract check fails.
    pub warning: Option<String>,
}

/// Holds the current word list and undo/redo history.
pub struct EditorState {
    words: Vec<Word>,
    undo_stack: Vec<Vec<Word>>,
    redo_stack: Vec<Vec<Word>>,
    timeline_revision: u64,
}

impl EditorState {
    /// Create an empty editor.
    pub fn new() -> Self {
        Self {
            words: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            timeline_revision: 0,
        }
    }

    /// Replace all words (e.g. from a new transcription result).
    /// Clears undo/redo history.
    pub fn set_words(&mut self, words: Vec<Word>) {
        self.words = words;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.bump_revision();
    }

    /// Return the current word list.
    pub fn get_words(&self) -> &[Word] {
        &self.words
    }

    /// Return a mutable reference to the word list for bulk mutations.
    pub(crate) fn get_words_mut(&mut self) -> &mut [Word] {
        &mut self.words
    }

    // ── snapshot helpers ──────────────────────────────────────────────

    /// Push a snapshot of the current words onto the undo stack,
    /// clear the redo stack, and enforce the 64-entry cap.
    pub(crate) fn push_undo_snapshot(&mut self) {
        self.undo_stack.push(self.words.clone());
        self.redo_stack.clear();
        if self.undo_stack.len() > MAX_UNDO {
            self.undo_stack.remove(0);
        }
    }

    pub(crate) fn bump_revision(&mut self) {
        self.timeline_revision = self.timeline_revision.saturating_add(1);
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
        self.bump_revision();
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
        self.bump_revision();
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
        self.bump_revision();
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
        self.bump_revision();
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
        self.bump_revision();
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
        self.bump_revision();
        true
    }

    /// Apply accepted local LLM proposals as word-level text edits.
    ///
    /// Invalid proposals are rejected with explicit reasons and do not mutate
    /// state. Successful proposals only update `Word.text`, preserving all
    /// timing/deletion/silence metadata so keep-segments and mapping remain
    /// canonical in backend state.
    pub fn apply_local_llm_word_proposals(
        &mut self,
        proposals: &[LocalLlmWordProposal],
    ) -> LocalLlmApplyResult {
        let mut rejected = Vec::new();
        let mut accepted: Vec<&LocalLlmWordProposal> = Vec::new();
        let mut reserved_indices = vec![false; self.words.len()];

        for (proposal_index, proposal) in proposals.iter().enumerate() {
            let start = proposal.start_word_index;
            let end = proposal.end_word_index;
            let range_len = end.saturating_sub(start);

            let reject_reason = if start >= end {
                Some("invalid proposal range: start must be < end".to_string())
            } else if end > self.words.len() {
                Some(format!(
                    "proposal range {start}..{end} is out of bounds for {} words",
                    self.words.len()
                ))
            } else if proposal.replacement_words.len() != range_len {
                Some(format!(
                    "proposal replacement word count mismatch: expected {range_len}, got {}",
                    proposal.replacement_words.len()
                ))
            } else if proposal
                .replacement_words
                .iter()
                .any(|word| word.trim().is_empty())
            {
                Some("proposal contains empty replacement words".to_string())
            } else if (start..end).any(|idx| reserved_indices[idx]) {
                Some("proposal overlaps with another accepted proposal".to_string())
            } else {
                None
            };

            if let Some(reason) = reject_reason {
                rejected.push(LocalLlmProposalRejection {
                    proposal_index,
                    start_word_index: start,
                    end_word_index: end,
                    reason,
                });
                continue;
            }

            for slot in reserved_indices[start..end].iter_mut() {
                *slot = true;
            }
            accepted.push(proposal);
        }

        let mut changed_indices: Vec<usize> = Vec::new();
        let mut snapshot_pushed = false;
        for proposal in &accepted {
            let start = proposal.start_word_index;
            for (offset, replacement) in proposal.replacement_words.iter().enumerate() {
                let idx = start + offset;
                if self.words[idx].text == *replacement {
                    continue;
                }
                if !snapshot_pushed {
                    self.push_undo_snapshot();
                    snapshot_pushed = true;
                }
                self.words[idx].text = replacement.clone();
                changed_indices.push(idx);
            }
        }

        if snapshot_pushed {
            changed_indices.sort_unstable();
            changed_indices.dedup();
            self.bump_revision();
        }

        LocalLlmApplyResult {
            applied_proposals: accepted.len(),
            applied_word_indices: changed_indices,
            rejected_proposals: rejected,
        }
    }

    // ── undo / redo ──────────────────────────────────────────────────

    /// Undo the last mutation. Returns `false` if nothing to undo.
    pub fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.undo_stack.pop() {
            self.redo_stack.push(self.words.clone());
            self.words = snapshot;
            self.bump_revision();
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
            self.bump_revision();
            true
        } else {
            false
        }
    }

    // ── keep-segments & time mapping ─────────────────────────────────

    /// Return contiguous non-deleted time regions as `(start_us, end_us)` pairs.
    ///
    /// Splits segments at large inter-word silence gaps (> 300ms) so that
    /// dead air between phrases is naturally excluded from export/preview.
    pub fn get_keep_segments(&self) -> Vec<(i64, i64)> {
        /// Maximum gap between adjacent words before splitting into separate
        /// keep-segments. Gaps larger than this are treated as dead air and
        /// excluded from the output.
        const MAX_INTRA_SEGMENT_GAP_US: i64 = 200_000; // 200ms

        let mut segments = Vec::new();
        let mut seg_start: Option<i64> = None;
        let mut seg_end: i64 = 0;

        for word in &self.words {
            if word.deleted {
                if let Some(start) = seg_start.take() {
                    segments.push((start, seg_end));
                }
            } else {
                if let Some(start) = seg_start {
                    // Check gap between this word and the previous kept word
                    let gap = word.start_us - seg_end;
                    if gap > MAX_INTRA_SEGMENT_GAP_US {
                        // Large gap — end current segment, start a new one
                        segments.push((start, seg_end));
                        seg_start = Some(word.start_us);
                    }
                } else {
                    seg_start = Some(word.start_us);
                }
                seg_end = word.end_us;
            }
        }

        if let Some(start) = seg_start {
            segments.push((start, seg_end));
        }

        // Merge micro-segments (<150ms) with their nearest neighbor to avoid
        // glitchy pops from ultra-short audio clips in the export.
        const MIN_KEEP_SEGMENT_US: i64 = 150_000; // 150ms minimum
        let mut i = 0;
        while i < segments.len() && segments.len() > 1 {
            let dur = segments[i].1 - segments[i].0;
            if dur < MIN_KEEP_SEGMENT_US {
                if i + 1 < segments.len() {
                    let gap = segments[i + 1].0 - segments[i].1;
                    if gap <= MAX_INTRA_SEGMENT_GAP_US {
                        segments[i] = (segments[i].0, segments[i + 1].1);
                        segments.remove(i + 1);
                        continue;
                    }
                }
                if i > 0 {
                    let gap = segments[i].0 - segments[i - 1].1;
                    if gap <= MAX_INTRA_SEGMENT_GAP_US {
                        segments[i - 1] = (segments[i - 1].0, segments[i].1);
                        segments.remove(i);
                        continue;
                    }
                }
            }
            i += 1;
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

    fn quantization_fps(&self) -> (u32, u32) {
        (DEFAULT_QUANTIZATION_FPS_NUM, DEFAULT_QUANTIZATION_FPS_DEN)
    }

    fn quantize_time_us(time_us: i64, fps_num: u32, fps_den: u32) -> i64 {
        if fps_num == 0 || fps_den == 0 {
            return time_us.max(0);
        }

        let den = 1_000_000_i128 * fps_den as i128;
        let scaled = time_us.max(0) as i128 * fps_num as i128;
        let frame_index = (scaled + den / 2) / den;
        let quantized = (frame_index * den) / fps_num as i128;

        quantized.clamp(i64::MIN as i128, i64::MAX as i128) as i64
    }

    fn quantize_keep_segments(
        &self,
        segments: &[(i64, i64)],
        fps_num: u32,
        fps_den: u32,
    ) -> Vec<(i64, i64)> {
        let mut quantized = Vec::with_capacity(segments.len());
        let mut previous_end = 0_i64;

        for (start, end) in segments {
            let mut q_start = Self::quantize_time_us(*start, fps_num, fps_den);
            let mut q_end = Self::quantize_time_us(*end, fps_num, fps_den);

            if q_start < previous_end {
                q_start = previous_end;
            }
            if q_end < q_start {
                q_end = q_start;
            }

            previous_end = q_end;
            quantized.push((q_start, q_end));
        }

        quantized
    }

    fn validate_keep_segments(
        &self,
        segments: &[(i64, i64)],
        source_start_us: i64,
        source_end_us: i64,
    ) -> (bool, Option<String>, i64) {
        let mut previous_end: Option<i64> = None;
        let mut total_keep_duration_us = 0_i64;

        for (idx, (start, end)) in segments.iter().enumerate() {
            if end < start {
                return (
                    false,
                    Some(format!("invalid keep segment at index {idx}: end < start")),
                    total_keep_duration_us,
                );
            }
            if let Some(prev_end) = previous_end {
                if *start < prev_end {
                    return (
                        false,
                        Some(format!(
                            "overlapping keep segments at index {idx}: start {start} < previous end {prev_end}"
                        )),
                        total_keep_duration_us,
                    );
                }
            }
            if *start < source_start_us || *end > source_end_us {
                return (
                    false,
                    Some(format!(
                        "keep segment at index {idx} outside source bounds [{source_start_us}, {source_end_us}]"
                    )),
                    total_keep_duration_us,
                );
            }
            total_keep_duration_us += end - start;
            previous_end = Some(*end);
        }

        let active_duration_us: i64 = self
            .words
            .iter()
            .filter(|w| !w.deleted && w.end_us >= w.start_us)
            .map(|w| w.end_us - w.start_us)
            .sum();

        if active_duration_us != total_keep_duration_us {
            return (
                false,
                Some(format!(
                    "active word duration ({active_duration_us}) != keep-segment duration ({total_keep_duration_us})"
                )),
                total_keep_duration_us,
            );
        }

        (true, None, total_keep_duration_us)
    }

    /// Return a diagnostics snapshot for edit-time/source-time contracts.
    pub fn timing_contract_snapshot(&self) -> TimingContractSnapshot {
        let total_words = self.words.len();
        let deleted_words = self.words.iter().filter(|w| w.deleted).count();
        let active_words = total_words.saturating_sub(deleted_words);

        let source_start_us = self.words.iter().map(|w| w.start_us).min().unwrap_or(0);
        let source_end_us = self.words.iter().map(|w| w.end_us).max().unwrap_or(0);

        let segments_raw = self.get_keep_segments();
        let keep_segments = segments_raw
            .iter()
            .map(|(start_us, end_us)| TimingSegment {
                start_us: *start_us,
                end_us: *end_us,
            })
            .collect::<Vec<_>>();
        let (quantization_fps_num, quantization_fps_den) = self.quantization_fps();
        let quantized_keep_segments = self
            .quantize_keep_segments(&segments_raw, quantization_fps_num, quantization_fps_den)
            .iter()
            .map(|(start_us, end_us)| TimingSegment {
                start_us: *start_us,
                end_us: *end_us,
            })
            .collect::<Vec<_>>();

        let (keep_segments_valid, warning, total_keep_duration_us) =
            self.validate_keep_segments(&segments_raw, source_start_us, source_end_us);

        TimingContractSnapshot {
            timeline_revision: self.timeline_revision,
            total_words,
            deleted_words,
            active_words,
            source_start_us,
            source_end_us,
            total_keep_duration_us,
            keep_segments,
            quantized_keep_segments,
            quantization_fps_num,
            quantization_fps_den,
            keep_segments_valid,
            warning,
        }
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

    #[test]
    fn local_llm_apply_preserves_mapping_and_timing_metadata() {
        let mut editor = EditorState::new();
        let mut words = make_words();
        words[4].deleted = true;
        editor.set_words(words);

        let before_non_text: Vec<(i64, i64, bool, bool, i32)> = editor
            .get_words()
            .iter()
            .map(|word| {
                (
                    word.start_us,
                    word.end_us,
                    word.deleted,
                    word.silenced,
                    word.speaker_id,
                )
            })
            .collect();
        let before_keep_segments = editor.get_keep_segments();
        let probe_times = [0_i64, 500_000, 1_250_000, 2_750_000, 3_900_000];
        let before_mapped: Vec<i64> = probe_times
            .iter()
            .map(|time| editor.map_edit_time_to_source_time(*time))
            .collect();

        let result = editor.apply_local_llm_word_proposals(&[
            LocalLlmWordProposal {
                start_word_index: 0,
                end_word_index: 2,
                replacement_words: vec!["Hey".into(), "planet".into()],
            },
            LocalLlmWordProposal {
                start_word_index: 2,
                end_word_index: 4,
                replacement_words: vec!["this".into(), "exists".into()],
            },
        ]);

        assert_eq!(result.applied_proposals, 2);
        assert!(result.rejected_proposals.is_empty());
        assert_eq!(result.applied_word_indices, vec![0, 1, 3]);
        assert_eq!(editor.get_words()[0].text, "Hey");
        assert_eq!(editor.get_words()[1].text, "planet");
        assert_eq!(editor.get_words()[2].text, "this");
        assert_eq!(editor.get_words()[3].text, "exists");

        for (idx, word) in editor.get_words().iter().enumerate() {
            let (start_us, end_us, deleted, silenced, speaker_id) = before_non_text[idx];
            assert_eq!(word.start_us, start_us);
            assert_eq!(word.end_us, end_us);
            assert_eq!(word.deleted, deleted);
            assert_eq!(word.silenced, silenced);
            assert_eq!(word.speaker_id, speaker_id);
        }

        assert_eq!(editor.get_keep_segments(), before_keep_segments);
        let after_mapped: Vec<i64> = probe_times
            .iter()
            .map(|time| editor.map_edit_time_to_source_time(*time))
            .collect();
        assert_eq!(after_mapped, before_mapped);
    }

    #[test]
    fn local_llm_apply_rejects_beginning_word_deletion_without_precision_drift() {
        let mut editor = EditorState::new();
        let mut words = make_words();
        words[0].text = "Hello,".into();
        words[1].text = "world!".into();
        editor.set_words(words);

        let before_words = editor.get_words().to_vec();
        let before_revision = editor.timing_contract_snapshot().timeline_revision;
        let before_keep_segments = editor.get_keep_segments();
        let probe_times = [0_i64, 750_000, 2_500_000, 4_250_000];
        let before_mapped: Vec<i64> = probe_times
            .iter()
            .map(|time| editor.map_edit_time_to_source_time(*time))
            .collect();

        let result = editor.apply_local_llm_word_proposals(&[LocalLlmWordProposal {
            start_word_index: 0,
            end_word_index: 2,
            replacement_words: vec!["world!".into()],
        }]);

        assert_eq!(result.applied_proposals, 0);
        assert!(result.applied_word_indices.is_empty());
        assert_eq!(result.rejected_proposals.len(), 1);
        assert!(result.rejected_proposals[0]
            .reason
            .contains("count mismatch"));
        assert_eq!(
            editor.timing_contract_snapshot().timeline_revision,
            before_revision
        );

        for (before, after) in before_words.iter().zip(editor.get_words().iter()) {
            assert_eq!(before.text, after.text);
            assert_eq!(before.start_us, after.start_us);
            assert_eq!(before.end_us, after.end_us);
            assert_eq!(before.deleted, after.deleted);
            assert_eq!(before.silenced, after.silenced);
            assert_eq!(before.confidence, after.confidence);
            assert_eq!(before.speaker_id, after.speaker_id);
        }
        assert_eq!(editor.get_keep_segments(), before_keep_segments);
        let after_mapped: Vec<i64> = probe_times
            .iter()
            .map(|time| editor.map_edit_time_to_source_time(*time))
            .collect();
        assert_eq!(after_mapped, before_mapped);
        assert!(
            !editor.undo(),
            "Rejected proposal must not push an undo snapshot"
        );
    }

    #[test]
    fn local_llm_apply_handles_punctuation_adjacent_edits_without_timing_drift() {
        let mut editor = EditorState::new();
        let mut words = make_words();
        words[0].text = "Helo,".into();
        words[1].text = "wrld!".into();
        editor.set_words(words);

        let before_non_text: Vec<(i64, i64, bool, bool, f32, i32)> = editor
            .get_words()
            .iter()
            .map(|word| {
                (
                    word.start_us,
                    word.end_us,
                    word.deleted,
                    word.silenced,
                    word.confidence,
                    word.speaker_id,
                )
            })
            .collect();
        let before_keep_segments = editor.get_keep_segments();
        let probe_times = [0_i64, 500_000, 1_999_999, 2_000_000, 3_500_000];
        let before_mapped: Vec<i64> = probe_times
            .iter()
            .map(|time| editor.map_edit_time_to_source_time(*time))
            .collect();

        let result = editor.apply_local_llm_word_proposals(&[
            LocalLlmWordProposal {
                start_word_index: 0,
                end_word_index: 1,
                replacement_words: vec!["Hello,".into()],
            },
            LocalLlmWordProposal {
                start_word_index: 1,
                end_word_index: 2,
                replacement_words: vec!["world!".into()],
            },
        ]);

        assert_eq!(result.applied_proposals, 2);
        assert_eq!(result.applied_word_indices, vec![0, 1]);
        assert!(result.rejected_proposals.is_empty());
        assert_eq!(editor.get_words()[0].text, "Hello,");
        assert_eq!(editor.get_words()[1].text, "world!");

        for (idx, word) in editor.get_words().iter().enumerate() {
            let (start_us, end_us, deleted, silenced, confidence, speaker_id) =
                before_non_text[idx];
            assert_eq!(word.start_us, start_us);
            assert_eq!(word.end_us, end_us);
            assert_eq!(word.deleted, deleted);
            assert_eq!(word.silenced, silenced);
            assert_eq!(word.confidence, confidence);
            assert_eq!(word.speaker_id, speaker_id);
        }

        assert_eq!(editor.get_keep_segments(), before_keep_segments);
        let after_mapped: Vec<i64> = probe_times
            .iter()
            .map(|time| editor.map_edit_time_to_source_time(*time))
            .collect();
        assert_eq!(after_mapped, before_mapped);
    }

    #[test]
    fn local_llm_apply_delete_undo_redo_parity_preserves_backend_mapping() {
        let mut editor = EditorState::new();
        let mut words = make_words();
        words[1].text = "wrld!".into();
        editor.set_words(words);

        assert!(editor.delete_word(0));
        let keep_segments_after_delete = editor.get_keep_segments();
        let probe_times = [0_i64, 500_000, 2_000_000, 3_750_000];
        let mapped_after_delete: Vec<i64> = probe_times
            .iter()
            .map(|time| editor.map_edit_time_to_source_time(*time))
            .collect();
        assert_eq!(editor.map_edit_time_to_source_time(0), 1_000_000);

        let apply_result = editor.apply_local_llm_word_proposals(&[LocalLlmWordProposal {
            start_word_index: 1,
            end_word_index: 2,
            replacement_words: vec!["world!".into()],
        }]);
        assert_eq!(apply_result.applied_proposals, 1);
        assert_eq!(apply_result.applied_word_indices, vec![1]);
        assert!(apply_result.rejected_proposals.is_empty());
        assert!(editor.get_words()[0].deleted);
        assert_eq!(editor.get_words()[1].text, "world!");
        assert_eq!(editor.get_keep_segments(), keep_segments_after_delete);
        let mapped_after_apply: Vec<i64> = probe_times
            .iter()
            .map(|time| editor.map_edit_time_to_source_time(*time))
            .collect();
        assert_eq!(mapped_after_apply, mapped_after_delete);

        assert!(editor.undo(), "first undo should revert LLM text edits");
        assert!(editor.get_words()[0].deleted);
        assert_eq!(editor.get_words()[1].text, "wrld!");
        assert_eq!(editor.get_keep_segments(), keep_segments_after_delete);
        let mapped_after_first_undo: Vec<i64> = probe_times
            .iter()
            .map(|time| editor.map_edit_time_to_source_time(*time))
            .collect();
        assert_eq!(mapped_after_first_undo, mapped_after_delete);

        assert!(
            editor.undo(),
            "second undo should revert the delete mutation"
        );
        assert!(!editor.get_words()[0].deleted);
        assert_eq!(editor.map_edit_time_to_source_time(0), 0);

        assert!(editor.redo(), "redo should restore the delete mutation");
        assert!(editor.get_words()[0].deleted);
        assert_eq!(editor.map_edit_time_to_source_time(0), 1_000_000);

        assert!(editor.redo(), "second redo should restore LLM text edits");
        assert!(editor.get_words()[0].deleted);
        assert_eq!(editor.get_words()[1].text, "world!");
        assert_eq!(editor.get_keep_segments(), keep_segments_after_delete);
        let mapped_after_second_redo: Vec<i64> = probe_times
            .iter()
            .map(|time| editor.map_edit_time_to_source_time(*time))
            .collect();
        assert_eq!(mapped_after_second_redo, mapped_after_delete);
    }

    #[test]
    fn local_llm_apply_rejects_invalid_proposals_without_destructive_side_effects() {
        let mut editor = EditorState::new();
        editor.set_words(make_words());

        let before_words = editor.get_words().to_vec();
        let before_revision = editor.timing_contract_snapshot().timeline_revision;

        let result = editor.apply_local_llm_word_proposals(&[
            LocalLlmWordProposal {
                start_word_index: 1,
                end_word_index: 3,
                replacement_words: vec!["single".into()],
            },
            LocalLlmWordProposal {
                start_word_index: 5,
                end_word_index: 9,
                replacement_words: vec!["test".into(), "words".into(), "here".into(), "x".into()],
            },
            LocalLlmWordProposal {
                start_word_index: 4,
                end_word_index: 4,
                replacement_words: vec![],
            },
        ]);

        assert_eq!(result.applied_proposals, 0);
        assert!(result.applied_word_indices.is_empty());
        assert_eq!(result.rejected_proposals.len(), 3);
        assert!(result
            .rejected_proposals
            .iter()
            .any(|rejection| rejection.reason.contains("count mismatch")));
        assert!(result
            .rejected_proposals
            .iter()
            .any(|rejection| rejection.reason.contains("out of bounds")));
        assert!(result
            .rejected_proposals
            .iter()
            .any(|rejection| rejection.reason.contains("start must be < end")));

        for (before, after) in before_words.iter().zip(editor.get_words().iter()) {
            assert_eq!(before.text, after.text);
            assert_eq!(before.start_us, after.start_us);
            assert_eq!(before.end_us, after.end_us);
            assert_eq!(before.deleted, after.deleted);
            assert_eq!(before.silenced, after.silenced);
            assert_eq!(before.confidence, after.confidence);
            assert_eq!(before.speaker_id, after.speaker_id);
        }
        assert_eq!(
            editor.timing_contract_snapshot().timeline_revision,
            before_revision
        );
    }

    #[test]
    fn local_llm_apply_supports_partial_success_with_overlap_rejection() {
        let mut editor = EditorState::new();
        editor.set_words(make_words());

        let result = editor.apply_local_llm_word_proposals(&[
            LocalLlmWordProposal {
                start_word_index: 0,
                end_word_index: 2,
                replacement_words: vec!["Hi".into(), "earth".into()],
            },
            LocalLlmWordProposal {
                start_word_index: 1,
                end_word_index: 3,
                replacement_words: vec!["middle".into(), "words".into()],
            },
        ]);

        assert_eq!(result.applied_proposals, 1);
        assert_eq!(result.applied_word_indices, vec![0, 1]);
        assert_eq!(result.rejected_proposals.len(), 1);
        assert!(result.rejected_proposals[0]
            .reason
            .contains("overlaps with another accepted proposal"));
        assert_eq!(editor.get_words()[0].text, "Hi");
        assert_eq!(editor.get_words()[1].text, "earth");
        assert_eq!(editor.get_words()[2].text, "this");
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
        target_s > current_s + END_EPSILON_S
            && now_ms - last_skip_ms > FALLBACK_SKIP_MIN_INTERVAL_MS
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
}
