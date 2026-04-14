use tauri::State;

use crate::commands::editor::EditorStore;
use crate::managers::export::{self, ExportConfig, ExportFormat};

#[tauri::command]
#[specta::specta]
pub fn export_transcript(
    store: State<EditorStore>,
    format: ExportFormat,
    max_chars_per_line: Option<usize>,
    include_silenced: Option<bool>,
) -> Result<String, String> {
    let state = store.0.lock().unwrap();
    let words = state.get_words();
    let config = ExportConfig {
        max_chars_per_line: max_chars_per_line.unwrap_or(42),
        include_silenced: include_silenced.unwrap_or(false),
        ..Default::default()
    };
    Ok(export::export(words, format, &config))
}

#[tauri::command]
#[specta::specta]
pub fn export_transcript_to_file(
    store: State<EditorStore>,
    format: ExportFormat,
    path: String,
    max_chars_per_line: Option<usize>,
    include_silenced: Option<bool>,
) -> Result<(), String> {
    let state = store.0.lock().unwrap();
    let words = state.get_words();
    let config = ExportConfig {
        max_chars_per_line: max_chars_per_line.unwrap_or(42),
        include_silenced: include_silenced.unwrap_or(false),
        ..Default::default()
    };
    export::export_to_file(words, format, &config, std::path::Path::new(&path))
}
