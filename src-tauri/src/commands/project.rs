use tauri::State;

use crate::commands::editor::EditorStore;
use crate::managers::media::MediaStore;
use crate::managers::project::ToasterProject;
use crate::settings::CaptionProfileSet;
use std::sync::Mutex;

/// Shared state holding the currently open project's caption profiles.
/// `None` means "inherit app-level"; `Some(set)` means the project owns
/// them. `save_project` serializes `Some` into the `.toaster` file;
/// `load_project` deserializes it back.
#[derive(Default)]
pub struct CurrentProjectStore(pub Mutex<Option<CaptionProfileSet>>);

/// Save the current project to a .toaster file.
#[tauri::command]
#[specta::specta]
pub fn save_project(
    app: tauri::AppHandle,
    editor_store: State<EditorStore>,
    media_store: State<MediaStore>,
    project_store: State<CurrentProjectStore>,
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

    // Caption profiles: persist the currently open project's override.
    // If the project has no override yet, crystallize the app-level
    // profiles so the saved file is fully self-describing.
    let project_profiles = project_store
        .0
        .lock()
        .ok()
        .and_then(|g| g.clone());
    let profiles = project_profiles.unwrap_or_else(|| {
        let settings = crate::settings::get_settings(&app);
        settings.caption_profiles
    });
    project.settings.caption_profiles = Some(profiles);

    project.save(std::path::Path::new(&path))
}

/// Load a .toaster project file and populate the editor + media state.
#[tauri::command]
#[specta::specta]
pub fn load_project(
    editor_store: State<EditorStore>,
    media_store: State<MediaStore>,
    project_store: State<CurrentProjectStore>,
    path: String,
) -> Result<String, String> {
    let project = ToasterProject::load(std::path::Path::new(&path))?;

    // Restore editor words
    let mut editor = crate::lock_recovery::try_lock(editor_store.0.lock()).map_err(|e| e.to_string())?;
    editor.set_words(project.words);

    // Restore caption profiles into the shared project store. v1.0.0
    // projects have no profiles → store `None`, which makes
    // `get_caption_profile` fall back to the app-level defaults.
    if let Ok(mut guard) = project_store.0.lock() {
        *guard = project.settings.caption_profiles.clone();
    }

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
