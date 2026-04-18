use tauri::State;

use crate::managers::media::{MediaInfo, MediaStore};

#[tauri::command]
#[specta::specta]
pub fn media_import(store: State<MediaStore>, path: String) -> Result<MediaInfo, String> {
    let mut state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
    state.import(std::path::Path::new(&path))
}

#[tauri::command]
#[specta::specta]
pub fn media_get_current(store: State<MediaStore>) -> Result<Option<MediaInfo>, String> {
    let state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
    Ok(state.current().cloned())
}

#[tauri::command]
#[specta::specta]
pub fn media_get_asset_url(store: State<MediaStore>) -> Result<Option<String>, String> {
    let state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
    Ok(state.asset_url())
}

#[tauri::command]
#[specta::specta]
pub fn media_clear(store: State<MediaStore>) -> Result<(), String> {
    let mut state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
    state.clear();
    Ok(())
}
