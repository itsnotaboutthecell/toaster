use tauri::State;

use crate::commands::editor::EditorStore;
use crate::managers::media::MediaStore;
use crate::managers::project::ToasterProject;

/// Save the current project to a .toaster file.
#[tauri::command]
#[specta::specta]
pub fn save_project(
    editor_store: State<EditorStore>,
    media_store: State<MediaStore>,
    path: String,
    name: Option<String>,
) -> Result<(), String> {
    let editor = crate::lock_recovery::try_lock(editor_store.0.lock()).map_err(|e| e.to_string())?;
    let media = crate::lock_recovery::try_lock(media_store.0.lock()).map_err(|e| e.to_string())?;

    let project_name = name.unwrap_or_else(|| {
        media
            .current()
            .map(|m| m.file_name.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    });

    let mut project = ToasterProject::new(&project_name);
    project.source_media = media.current().map(|m| m.path.clone());
    project.set_words(editor.get_words().to_vec());

    project.save(std::path::Path::new(&path))
}

/// Load a .toaster project file and populate the editor + media state.
#[tauri::command]
#[specta::specta]
pub fn load_project(
    editor_store: State<EditorStore>,
    media_store: State<MediaStore>,
    path: String,
) -> Result<String, String> {
    let project = ToasterProject::load(std::path::Path::new(&path))?;

    // Restore editor words
    let mut editor = crate::lock_recovery::try_lock(editor_store.0.lock()).map_err(|e| e.to_string())?;
    editor.set_words(project.words);

    // Restore media if path exists
    if let Some(ref media_path) = project.source_media {
        if media_path.exists() {
            let mut media = crate::lock_recovery::try_lock(media_store.0.lock()).map_err(|e| e.to_string())?;
            media.import(media_path)?;
        }
    }

    Ok(project
        .source_media
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default())
}
