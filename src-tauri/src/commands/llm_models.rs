//! Tauri commands for the in-process LLM catalog.
//!
//! Mirrors the surface area of `commands::models` for Whisper. The manager
//! is registered as an `Arc<LlmManager>` in `lib.rs`.

use crate::managers::llm::{LlmManager, LlmModelInfo};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
#[specta::specta]
pub async fn list_llm_models(
    llm_manager: State<'_, Arc<LlmManager>>,
) -> Result<Vec<LlmModelInfo>, String> {
    Ok(llm_manager.list_models())
}

#[tauri::command]
#[specta::specta]
pub async fn download_llm_model(
    app_handle: AppHandle,
    llm_manager: State<'_, Arc<LlmManager>>,
    model_id: String,
) -> Result<(), String> {
    let emitter = app_handle.clone();
    let emit_id = model_id.clone();
    let result = llm_manager
        .download(&model_id, move |progress| {
            let _ = emitter.emit(
                "llm-model-download-progress",
                serde_json::json!({
                    "model_id": progress.model_id,
                    "downloaded": progress.downloaded,
                    "total": progress.total,
                    "percentage": progress.percentage,
                    "speed_bps": progress.speed_bps,
                    "asset_kind": "llm",
                }),
            );
        })
        .await
        .map_err(|e| e.to_string());

    if let Err(ref error) = result {
        let _ = app_handle.emit(
            "llm-model-download-failed",
            crate::managers::llm::download_failed_payload(&emit_id, error),
        );
    } else {
        let _ = app_handle.emit(
            "llm-model-download-completed",
            serde_json::json!({ "model_id": &emit_id, "asset_kind": "llm" }),
        );
    }
    result
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_llm_download(
    app_handle: AppHandle,
    llm_manager: State<'_, Arc<LlmManager>>,
    model_id: String,
) -> Result<(), String> {
    llm_manager
        .cancel_download(&model_id)
        .map_err(|e| e.to_string())?;
    let _ = app_handle.emit(
        "llm-model-download-cancelled",
        serde_json::json!({ "model_id": &model_id, "asset_kind": "llm" }),
    );
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_llm_model(
    app_handle: AppHandle,
    llm_manager: State<'_, Arc<LlmManager>>,
    model_id: String,
) -> Result<(), String> {
    llm_manager.delete(&model_id).map_err(|e| e.to_string())?;
    // If the deleted model was the selected local LLM, clear the setting.
    let mut settings = crate::settings::get_settings(&app_handle);
    if settings.local_llm_model_id.as_deref() == Some(model_id.as_str()) {
        settings.local_llm_model_id = None;
        crate::settings::write_settings(&app_handle, settings);
    }
    let _ = app_handle.emit(
        "llm-model-deleted",
        serde_json::json!({ "model_id": &model_id, "asset_kind": "llm" }),
    );
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn set_selected_llm_model(
    app_handle: AppHandle,
    model_id: Option<String>,
) -> Result<(), String> {
    let mut settings = crate::settings::get_settings(&app_handle);
    // Validate that the id is in the catalog if provided.
    if let Some(ref id) = model_id {
        if crate::managers::llm::catalog::find_entry(id).is_none() {
            return Err(format!("Unknown LLM model id: {}", id));
        }
    }
    settings.local_llm_model_id = model_id;
    crate::settings::write_settings(&app_handle, settings);
    Ok(())
}
