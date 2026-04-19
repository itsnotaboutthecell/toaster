use std::sync::Mutex;
use tauri::State;

use crate::managers::editor::{EditorState, TimingContractSnapshot, Word};

/// Managed state wrapper for the transcript editor engine.
pub struct EditorStore(pub Mutex<EditorState>);

/// Atomic frontend projection of editor state after a backend transaction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct EditorProjection {
    pub words: Vec<Word>,
    pub timing_contract: TimingContractSnapshot,
}

fn build_projection(state: &EditorState) -> EditorProjection {
    EditorProjection {
        words: state.get_words().to_vec(),
        timing_contract: state.timing_contract_snapshot(),
    }
}

#[tauri::command]
#[specta::specta]
pub fn editor_set_words(store: State<EditorStore>, words: Vec<Word>) -> Vec<Word> {
    let mut state = crate::lock_recovery::recover_lock(store.0.lock());
    state.set_words(words);
    state.get_words().to_vec()
}

#[tauri::command]
#[specta::specta]
pub fn editor_get_words(store: State<EditorStore>) -> Vec<Word> {
    let state = crate::lock_recovery::recover_lock(store.0.lock());
    state.get_words().to_vec()
}

#[tauri::command]
#[specta::specta]
pub fn editor_delete_word(store: State<EditorStore>, index: usize) -> bool {
    let mut state = crate::lock_recovery::recover_lock(store.0.lock());
    state.delete_word(index)
}

#[tauri::command]
#[specta::specta]
pub fn editor_restore_word(store: State<EditorStore>, index: usize) -> bool {
    let mut state = crate::lock_recovery::recover_lock(store.0.lock());
    state.restore_word(index)
}

#[tauri::command]
#[specta::specta]
pub fn editor_delete_range(store: State<EditorStore>, start: usize, end: usize) -> bool {
    let mut state = crate::lock_recovery::recover_lock(store.0.lock());
    state.delete_range(start, end)
}

#[tauri::command]
#[specta::specta]
pub fn editor_restore_all(store: State<EditorStore>) -> bool {
    let mut state = crate::lock_recovery::recover_lock(store.0.lock());
    state.restore_all()
}

#[tauri::command]
#[specta::specta]
pub fn editor_split_word(store: State<EditorStore>, index: usize, position: usize) -> bool {
    let mut state = crate::lock_recovery::recover_lock(store.0.lock());
    state.split_word(index, position)
}

#[tauri::command]
#[specta::specta]
pub fn editor_silence_word(store: State<EditorStore>, index: usize) -> bool {
    let mut state = crate::lock_recovery::recover_lock(store.0.lock());
    state.silence_word(index)
}

#[tauri::command]
#[specta::specta]
pub fn editor_undo(store: State<EditorStore>) -> bool {
    let mut state = crate::lock_recovery::recover_lock(store.0.lock());
    state.undo()
}

#[tauri::command]
#[specta::specta]
pub fn editor_redo(store: State<EditorStore>) -> bool {
    let mut state = crate::lock_recovery::recover_lock(store.0.lock());
    state.redo()
}

#[tauri::command]
#[specta::specta]
pub fn editor_get_keep_segments(store: State<EditorStore>) -> Vec<(i64, i64)> {
    let state = crate::lock_recovery::recover_lock(store.0.lock());
    let snapshot = state.timing_contract_snapshot();
    if snapshot.keep_segments_valid {
        snapshot
            .keep_segments
            .into_iter()
            .map(|seg| (seg.start_us, seg.end_us))
            .filter(|(start_us, end_us)| end_us > start_us)
            .collect()
    } else {
        snapshot
            .quantized_keep_segments
            .into_iter()
            .map(|seg| (seg.start_us, seg.end_us))
            .filter(|(start_us, end_us)| end_us > start_us)
            .collect()
    }
}

#[tauri::command]
#[specta::specta]
pub fn editor_get_timing_contract(store: State<EditorStore>) -> TimingContractSnapshot {
    let state = crate::lock_recovery::recover_lock(store.0.lock());
    state.timing_contract_snapshot()
}

#[tauri::command]
#[specta::specta]
pub fn editor_get_projection(store: State<EditorStore>) -> EditorProjection {
    let state = crate::lock_recovery::recover_lock(store.0.lock());
    build_projection(&state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_words() -> Vec<Word> {
        vec![
            Word {
                text: "alpha".to_string(),
                start_us: 0,
                end_us: 1_000_000,
                deleted: false,
                silenced: false,
                confidence: 1.0,
                speaker_id: 0,
            },
            Word {
                text: "beta".to_string(),
                start_us: 1_000_000,
                end_us: 2_000_000,
                deleted: false,
                silenced: false,
                confidence: 1.0,
                speaker_id: 0,
            },
        ]
    }

    #[test]
    fn build_projection_returns_words_and_timing_contract() {
        let mut state = EditorState::new();
        state.set_words(sample_words());
        state.delete_word(1);

        let projection = build_projection(&state);
        assert_eq!(projection.words.len(), 2);
        assert_eq!(projection.timing_contract.total_words, 2);
        assert_eq!(projection.timing_contract.deleted_words, 1);
        assert_eq!(projection.timing_contract.keep_segments.len(), 1);
    }

    #[test]
    fn projection_timing_revision_matches_editor_revision() {
        let mut state = EditorState::new();
        state.set_words(sample_words());
        let first = build_projection(&state).timing_contract.timeline_revision;

        state.delete_word(0);
        let second = build_projection(&state).timing_contract.timeline_revision;

        assert!(second > first);
    }
}
