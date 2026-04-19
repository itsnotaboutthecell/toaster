// Transcript editing engine for word-level video editing.
//
// Manages a list of timestamped words with delete/restore/split/silence
// operations and full undo/redo support (up to 64 snapshots).

mod types;
pub use types::{TimingContractSnapshot, TimingSegment, Word};

const MAX_UNDO: usize = 64;
const DEFAULT_QUANTIZATION_FPS_NUM: u32 = 30;
const DEFAULT_QUANTIZATION_FPS_DEN: u32 = 1;

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
        /// Minimum kept-segment duration before the micro-merge pass tries
        /// to fold it into a neighbour. Prevents ultra-short glitch clips.
        const MIN_KEEP_SEGMENT_US: i64 = 150_000; // 150ms minimum

        let mut segments: Vec<(i64, i64)> = Vec::new();
        // Parallel to `segments`: true iff the seam that opened this segment
        // was created by a user delete (not by a natural silence gap). Used
        // by the micro-merge pass to refuse to bridge delete-driven seams,
        // which would otherwise put deleted audio back on the timeline.
        let mut delete_boundary_before: Vec<bool> = Vec::new();

        let mut seg_start: Option<i64> = None;
        let mut seg_end: i64 = 0;
        // Running high-water mark of any deleted word's end_us seen so far.
        // Used to clamp the next kept segment's start up, defending against
        // forced-alignment outputs where a kept word's start_us precedes
        // the end of an adjacent deleted word (overlapping boundaries).
        let mut prev_deleted_end: i64 = i64::MIN;
        // Flag carried forward until the next segment actually opens.
        let mut next_segment_after_delete = false;
        // Captured when the current segment opened, pushed alongside it.
        let mut current_opened_after_delete = false;

        for word in &self.words {
            if word.deleted {
                if let Some(start) = seg_start.take() {
                    // Clamp the closing edge down so it cannot extend past
                    // the deleted word's start even when word boundaries
                    // overlap in the alignment output.
                    let bound = seg_end.min(word.start_us);
                    if bound > start {
                        segments.push((start, bound));
                        delete_boundary_before.push(current_opened_after_delete);
                    }
                }
                prev_deleted_end = prev_deleted_end.max(word.end_us);
                next_segment_after_delete = true;
            } else {
                // Clamp the opening edge up so it cannot precede the end of
                // any earlier deleted word (handles overlapping boundaries).
                let word_start = word.start_us.max(prev_deleted_end);
                if word.end_us <= word_start {
                    // Entirely swallowed by a prior delete region; skip.
                    continue;
                }
                if let Some(start) = seg_start {
                    let gap = word_start - seg_end;
                    if gap > MAX_INTRA_SEGMENT_GAP_US {
                        // Large silence split — end current segment, start a new one.
                        segments.push((start, seg_end));
                        delete_boundary_before.push(current_opened_after_delete);
                        // The seam that just opened is a silence split, not a delete.
                        current_opened_after_delete = false;
                        seg_start = Some(word_start);
                    }
                } else {
                    current_opened_after_delete = next_segment_after_delete;
                    seg_start = Some(word_start);
                }
                next_segment_after_delete = false;
                seg_end = word.end_us;
            }
        }

        if let Some(start) = seg_start {
            segments.push((start, seg_end));
            delete_boundary_before.push(current_opened_after_delete);
        }

        // Merge micro-segments (<150ms) with their nearest neighbor to avoid
        // glitchy pops from ultra-short audio clips in the export. Refuse to
        // merge across a delete-driven seam — doing so would re-introduce
        // audio the user explicitly deleted.
        let mut i = 0;
        while i < segments.len() && segments.len() > 1 {
            let dur = segments[i].1 - segments[i].0;
            if dur < MIN_KEEP_SEGMENT_US {
                // Try forward merge (seam between i and i+1 is
                // `delete_boundary_before[i + 1]`).
                if i + 1 < segments.len() && !delete_boundary_before[i + 1] {
                    let gap = segments[i + 1].0 - segments[i].1;
                    if gap <= MAX_INTRA_SEGMENT_GAP_US {
                        segments[i] = (segments[i].0, segments[i + 1].1);
                        segments.remove(i + 1);
                        delete_boundary_before.remove(i + 1);
                        continue;
                    }
                }
                // Try backward merge (seam before i is
                // `delete_boundary_before[i]`).
                if i > 0 && !delete_boundary_before[i] {
                    let gap = segments[i].0 - segments[i - 1].1;
                    if gap <= MAX_INTRA_SEGMENT_GAP_US {
                        segments[i - 1] = (segments[i - 1].0, segments[i].1);
                        segments.remove(i);
                        delete_boundary_before.remove(i);
                        continue;
                    }
                }
            }
            i += 1;
        }

        segments
    }

    /// Return source-time ranges of every silenced (but not deleted) word.
    ///
    /// Deletion takes precedence: a word that is both deleted and silenced is
    /// excluded from the timeline entirely via `get_keep_segments`, so it
    /// does not appear here. The returned ranges are in the ORIGINAL source
    /// timeline (not the edited timeline) and are NOT merged — callers map
    /// them into edit-time when composing FFmpeg filters.
    ///
    /// Paired with `get_keep_segments` (boundary-based, silence-agnostic):
    /// keep-segments decide which audio stays on the timeline; silenced
    /// ranges decide which portions of that retained audio are muted in
    /// preview and export. Keeping these two concerns separate preserves
    /// timing (silenced words do not shrink the edited timeline) and lets
    /// the backend remain the single source of truth for both the dual
    /// preview/export render paths.
    pub fn get_silenced_ranges(&self) -> Vec<(i64, i64)> {
        self.words
            .iter()
            .filter(|w| w.silenced && !w.deleted && w.end_us > w.start_us)
            .map(|w| (w.start_us, w.end_us))
            .collect()
    }

    /// Map a position on the edited timeline (deletions removed) back to
    /// the original source timeline.
    ///
    /// Walks keep-segments, accumulating edit-time. When the accumulated
    /// time reaches `edit_time_us`, interpolates within that segment.
    ///
    /// NOTE: Production callers (preview scrubbing, waveform cursor) now
    /// route through `canonical_keep_segments_for_media` +
    /// `map_edit_time_to_source_time_from_segments` in
    /// `commands/waveform/mod.rs` so preview and export share one segment
    /// source of truth. This method is retained because the editor
    /// precision test-suite uses it as a compact reference for the
    /// semantic contract ("given an edited-timeline offset, return the
    /// source-timeline offset"); keeping it documents that contract at
    /// the type that owns the words/deletions.
    #[allow(dead_code)]
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
mod tests;
