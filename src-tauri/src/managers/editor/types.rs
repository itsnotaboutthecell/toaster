//! Type definitions for the transcript editor state machine.
//!
//! Extracted from `editor/mod.rs`. Pure data types shared between the
//! editor state implementation, commands, and Tauri IPC surface.

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
