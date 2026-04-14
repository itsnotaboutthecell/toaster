use tauri::State;

use crate::managers::media::{MediaInfo, MediaStore};

#[tauri::command]
#[specta::specta]
pub fn media_import(store: State<MediaStore>, path: String) -> Result<MediaInfo, String> {
    let mut state = store.0.lock().unwrap();
    state.import(std::path::Path::new(&path))
}

#[tauri::command]
#[specta::specta]
pub fn media_get_current(store: State<MediaStore>) -> Result<Option<MediaInfo>, String> {
    let state = store.0.lock().unwrap();
    Ok(state.current().cloned())
}

#[tauri::command]
#[specta::specta]
pub fn media_get_asset_url(store: State<MediaStore>) -> Result<Option<String>, String> {
    let state = store.0.lock().unwrap();
    Ok(state.asset_url())
}

#[tauri::command]
#[specta::specta]
pub fn media_clear(store: State<MediaStore>) -> Result<(), String> {
    let mut state = store.0.lock().unwrap();
    state.clear();
    Ok(())
}
