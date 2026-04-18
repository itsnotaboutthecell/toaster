//! Audio-aware "smart" cleanup commands.
//!
//! The Handy-era cleanup flow collapses adjacent repeated words by
//! blindly deleting the *second* token in each pair. That's a positional
//! heuristic — it's wrong whenever the speaker mumbled the first take
//! and nailed the second (or the middle of a triple). This module
//! replaces that rule with an audio-driven scorer: every repeat-group
//! member is scored for articulation against the source audio, and the
//! highest-scoring one survives. The survivor index is derived from the
//! audio, never from position.

use std::path::Path;
use std::process::Command;

use tauri::State;

use crate::commands::editor::EditorStore;
use crate::managers::disfluency;
use crate::managers::editor::Word;
use crate::managers::media::MediaStore;

/// Per-group decision exposed to the UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SmartGroupDecision {
    /// Lowercase, punctuation-stripped repeated token (e.g. "the").
    pub token: String,
    /// Word indices that formed the repeat group, in transcript order.
    pub members: Vec<usize>,
    /// Index (into the word list) chosen as the survivor.
    pub survivor_index: usize,
    /// Indices marked deleted by this decision.
    pub deleted_indices: Vec<usize>,
    /// Articulation score of the survivor in `[0, 1]`.
    pub survivor_score: f32,
    /// Articulation score of every member, aligned with `members`.
    pub member_scores: Vec<f32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SmartCleanupResult {
    pub groups_collapsed: usize,
    pub words_deleted: usize,
    pub decisions: Vec<SmartGroupDecision>,
    /// Sample rate used when scoring (16000 when audio was available, 0
    /// when the command ran without audio and refused to fall back).
    pub sample_rate: u32,
}

/// Decode the current media file to 16 kHz mono f32 samples using
/// ffmpeg. Same shape as `transcribe_file::extract`'s pipeline but kept
/// local to the cleanup command so we don't pull on ASR-only helpers.
///
/// `pub(crate)` so `cleanup_all` can reuse it.
pub(crate) fn decode_media_audio(path: &Path) -> Result<Vec<f32>, String> {
    let output = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-i",
            &path.to_string_lossy(),
            "-vn",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-f",
            "f32le",
            "pipe:1",
        ])
        .output()
        .map_err(|e| format!("ffmpeg audio decode failed to start: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "ffmpeg audio decode failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let mut samples = Vec::with_capacity(output.stdout.len() / 4);
    for chunk in output.stdout.chunks_exact(4) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(samples)
}

/// Run the audio-aware survivor planner over `words`, returning the
/// (decisions, total_deleted) pair. Pure helper for both the command
/// below and the existing `cleanup_all` flow.
pub(crate) fn plan_smart_collapse(
    words: &[Word],
    samples: &[f32],
    sample_rate: u32,
) -> (Vec<SmartGroupDecision>, Vec<usize>) {
    let mut decisions = Vec::new();
    let mut indices_to_delete = Vec::new();

    for d in disfluency::plan(words, samples, sample_rate) {
        indices_to_delete.extend(d.losers.iter().copied());
        decisions.push(SmartGroupDecision {
            token: d.group.token.clone(),
            members: d.group.members.clone(),
            survivor_index: d.survivor,
            deleted_indices: d.losers.clone(),
            survivor_score: d.survivor_score,
            member_scores: d.member_scores.iter().map(|s| s.articulation).collect(),
        });
    }

    (decisions, indices_to_delete)
}

/// Collapse every adjacent repetition group in the current transcript
/// by keeping the **clearest** take and deleting the rest. Returns per-
/// group detail so the UI can highlight what was decided and why.
///
/// If no media is loaded or audio decode fails, the command refuses to
/// run rather than silently falling back to the positional rule. That
/// refusal is intentional — positional "keep the first" behavior is
/// still available via the legacy `delete_duplicates` command.
#[tauri::command]
#[specta::specta]
pub fn cleanup_smart_duplicates(
    store: State<'_, EditorStore>,
    media_store: State<'_, MediaStore>,
) -> Result<SmartCleanupResult, String> {
    let media_path = {
        let media = crate::lock_recovery::try_lock(media_store.0.lock()).map_err(|e| e.to_string())?;
        media
            .current()
            .map(|m| m.path.clone())
            .ok_or_else(|| "no media loaded — cannot score audio clarity".to_string())?
    };

    let samples = decode_media_audio(&media_path)?;
    let sample_rate = 16_000u32;

    let mut state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
    let (decisions, indices_to_delete) =
        plan_smart_collapse(state.get_words(), &samples, sample_rate);

    if indices_to_delete.is_empty() {
        return Ok(SmartCleanupResult {
            groups_collapsed: 0,
            words_deleted: 0,
            decisions,
            sample_rate,
        });
    }

    state.push_undo_snapshot();
    let words = state.get_words_mut();
    let mut actually_deleted: usize = 0;
    for idx in &indices_to_delete {
        if let Some(w) = words.get_mut(*idx) {
            if !w.deleted {
                w.deleted = true;
                actually_deleted += 1;
            }
        }
    }
    state.bump_revision();

    Ok(SmartCleanupResult {
        groups_collapsed: decisions.len(),
        words_deleted: actually_deleted,
        decisions,
        sample_rate,
    })
}
