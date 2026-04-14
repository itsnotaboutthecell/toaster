use std::sync::Mutex;
use tauri::State;

use crate::managers::editor::{EditorState, Word};

/// Managed state wrapper for the transcript editor engine.
pub struct EditorStore(pub Mutex<EditorState>);

#[tauri::command]
#[specta::specta]
pub fn editor_set_words(store: State<EditorStore>, words: Vec<Word>) -> Vec<Word> {
    let mut state = store.0.lock().unwrap();
    state.set_words(words);
    state.get_words().to_vec()
}

#[tauri::command]
#[specta::specta]
pub fn editor_get_words(store: State<EditorStore>) -> Vec<Word> {
    let state = store.0.lock().unwrap();
    state.get_words().to_vec()
}

#[tauri::command]
#[specta::specta]
pub fn editor_delete_word(store: State<EditorStore>, index: usize) -> bool {
    let mut state = store.0.lock().unwrap();
    state.delete_word(index)
}

#[tauri::command]
#[specta::specta]
pub fn editor_restore_word(store: State<EditorStore>, index: usize) -> bool {
    let mut state = store.0.lock().unwrap();
    state.restore_word(index)
}

#[tauri::command]
#[specta::specta]
pub fn editor_delete_range(store: State<EditorStore>, start: usize, end: usize) -> bool {
    let mut state = store.0.lock().unwrap();
    state.delete_range(start, end)
}

#[tauri::command]
#[specta::specta]
pub fn editor_restore_all(store: State<EditorStore>) -> bool {
    let mut state = store.0.lock().unwrap();
    state.restore_all()
}

#[tauri::command]
#[specta::specta]
pub fn editor_split_word(store: State<EditorStore>, index: usize, position: usize) -> bool {
    let mut state = store.0.lock().unwrap();
    state.split_word(index, position)
}

#[tauri::command]
#[specta::specta]
pub fn editor_silence_word(store: State<EditorStore>, index: usize) -> bool {
    let mut state = store.0.lock().unwrap();
    state.silence_word(index)
}

#[tauri::command]
#[specta::specta]
pub fn editor_undo(store: State<EditorStore>) -> bool {
    let mut state = store.0.lock().unwrap();
    state.undo()
}

#[tauri::command]
#[specta::specta]
pub fn editor_redo(store: State<EditorStore>) -> bool {
    let mut state = store.0.lock().unwrap();
    state.redo()
}

#[tauri::command]
#[specta::specta]
pub fn editor_get_keep_segments(store: State<EditorStore>) -> Vec<(i64, i64)> {
    let state = store.0.lock().unwrap();
    state.get_keep_segments()
}
