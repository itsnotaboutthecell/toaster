//! App settings Tauri commands.
//!
//! These commands back the editor's settings UI (captions, export, language,
//! whisper accelerator, post-process providers, filler words, etc.). They were
//! historically housed in the `shortcut` module because Handy bundled
//! everything under keyboard-shortcut handling; they have nothing to do with
//! keyboard shortcuts. Moved here by p1-shortcut-split-settings.

use tauri::{AppHandle, Emitter, Manager};

use crate::settings::{self, LLMPrompt};

/// Validate that a post-process provider exists in the user's settings.
fn validate_provider_exists(
    settings: &settings::AppSettings,
    provider_id: &str,
) -> Result<(), String> {
    if !settings
        .post_process_providers
        .iter()
        .any(|provider| provider.id == provider_id)
    {
        return Err(format!("Provider '{}' not found", provider_id));
    }
    Ok(())
}

/// Save accelerator settings, re-apply globals, and unload the model so it
/// reloads with the new backend on next transcription.
fn apply_and_reload_accelerator(app: &AppHandle, s: settings::AppSettings) {
    settings::write_settings(app, s);
    crate::managers::transcription::apply_accelerator_settings(app);

    let tm = app.state::<std::sync::Arc<crate::managers::transcription::TranscriptionManager>>();
    if tm.is_model_loaded() {
        if let Err(e) = tm.unload_model() {
            log::warn!("Failed to unload model after accelerator change: {e}");
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn change_translate_to_english_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.translate_to_english = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_selected_language_setting(app: AppHandle, language: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.selected_language = language;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_debug_mode_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.debug_mode = enabled;
    settings::write_settings(&app, settings);

    // Emit event to notify frontend of debug mode change
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "debug_mode",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_update_checks_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.update_checks_enabled = enabled;
    settings::write_settings(&app, settings);

    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "update_checks_enabled",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn update_custom_words(app: AppHandle, words: Vec<String>) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.custom_words = words;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_custom_filler_words_setting(
    app: AppHandle,
    words: Vec<String>,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.custom_filler_words = Some(words);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_caption_font_size_setting(app: AppHandle, size: u32) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.caption_font_size = size.clamp(12, 72);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_caption_bg_color_setting(app: AppHandle, color: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.caption_bg_color = color;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_caption_text_color_setting(app: AppHandle, color: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.caption_text_color = color;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_caption_position_setting(app: AppHandle, position: u32) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.caption_position = position.clamp(0, 100);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_word_correction_threshold_setting(
    app: AppHandle,
    threshold: f64,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.word_correction_threshold = threshold;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_enabled_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.post_process_enabled = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_base_url_setting(
    app: AppHandle,
    provider_id: String,
    base_url: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let label = settings
        .post_process_provider(&provider_id)
        .map(|provider| provider.label.clone())
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

    let provider = settings
        .post_process_provider_mut(&provider_id)
        .expect("Provider looked up above must exist");

    if !provider.allow_base_url_edit {
        return Err(format!(
            "Provider '{}' does not allow editing the base URL",
            label
        ));
    }

    let sanitized_base_url = if settings::is_local_post_process_provider(provider) {
        settings::sanitize_local_post_process_base_url(&base_url)?
    } else {
        let trimmed = base_url.trim().trim_end_matches('/').to_string();
        if trimmed.is_empty() {
            return Err("Base URL cannot be empty".to_string());
        }
        trimmed
    };

    provider.base_url = sanitized_base_url;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_api_key_setting(
    app: AppHandle,
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;
    settings.post_process_api_keys.insert(provider_id, api_key);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_model_setting(
    app: AppHandle,
    provider_id: String,
    model: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;
    let sanitized_model = settings::sanitize_post_process_model(&model)?;
    settings
        .post_process_models
        .insert(provider_id, sanitized_model);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_post_process_provider(app: AppHandle, provider_id: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    validate_provider_exists(&settings, &provider_id)?;

    if let Some(provider) = settings.post_process_provider(&provider_id) {
        if settings::is_local_post_process_provider(provider) {
            settings::sanitize_local_post_process_base_url(&provider.base_url).map_err(|e| {
                format!(
                    "Invalid local base URL for '{}': {}. Update the provider base URL and try again.",
                    provider.label, e
                )
            })?;
        }
    }

    settings.post_process_provider_id = provider_id;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn add_post_process_prompt(
    app: AppHandle,
    name: String,
    prompt: String,
) -> Result<LLMPrompt, String> {
    let mut settings = settings::get_settings(&app);

    // Generate unique ID using timestamp and random component
    let id = format!("prompt_{}", chrono::Utc::now().timestamp_millis());

    let new_prompt = LLMPrompt {
        id: id.clone(),
        name,
        prompt,
    };

    settings.post_process_prompts.push(new_prompt.clone());
    settings::write_settings(&app, settings);

    Ok(new_prompt)
}

#[tauri::command]
#[specta::specta]
pub fn update_post_process_prompt(
    app: AppHandle,
    id: String,
    name: String,
    prompt: String,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);

    if let Some(existing_prompt) = settings
        .post_process_prompts
        .iter_mut()
        .find(|p| p.id == id)
    {
        existing_prompt.name = name;
        existing_prompt.prompt = prompt;
        settings::write_settings(&app, settings);
        Ok(())
    } else {
        Err(format!("Prompt with id '{}' not found", id))
    }
}

#[tauri::command]
#[specta::specta]
pub fn delete_post_process_prompt(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);

    // Don't allow deleting the last prompt
    if settings.post_process_prompts.len() <= 1 {
        return Err("Cannot delete the last prompt".to_string());
    }

    // Find and remove the prompt
    let original_len = settings.post_process_prompts.len();
    settings.post_process_prompts.retain(|p| p.id != id);

    if settings.post_process_prompts.len() == original_len {
        return Err(format!("Prompt with id '{}' not found", id));
    }

    // If the deleted prompt was selected, select the first one or None
    if settings.post_process_selected_prompt_id.as_ref() == Some(&id) {
        settings.post_process_selected_prompt_id =
            settings.post_process_prompts.first().map(|p| p.id.clone());
    }

    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn fetch_post_process_models(
    app: AppHandle,
    provider_id: String,
) -> Result<Vec<String>, String> {
    let settings = settings::get_settings(&app);

    // Find the provider
    let provider = settings
        .post_process_providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

    if settings::is_local_post_process_provider(provider) {
        settings::sanitize_local_post_process_base_url(&provider.base_url).map_err(|e| {
            format!(
                "Invalid local endpoint for '{}': {}. Expected localhost/loopback OpenAI-compatible URL.",
                provider.label, e
            )
        })?;
    }

    // Get API key
    let api_key = settings
        .post_process_api_keys
        .get(&provider_id)
        .cloned()
        .unwrap_or_default();

    // Skip fetching if no API key for providers that require one
    if provider.requires_api_key && api_key.trim().is_empty() {
        return Err(format!(
            "API key is required for {}. Please add an API key to list available models.",
            provider.label
        ));
    }

    match crate::llm_client::fetch_models(provider, api_key).await {
        Ok(models) => {
            if settings::is_local_post_process_provider(provider) && models.is_empty() {
                Err(format!(
                    "Connected to '{}' but no models were returned from its /models endpoint. Ensure OpenAI compatibility mode is enabled.",
                    provider.label
                ))
            } else {
                Ok(models)
            }
        }
        Err(error) => {
            if settings::is_local_post_process_provider(provider) {
                Err(format!(
                    "Could not reach local provider '{}' at '{}': {}. Make sure the local server is running and exposes OpenAI-compatible /models.",
                    provider.label, provider.base_url, error
                ))
            } else {
                Err(error)
            }
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn set_post_process_selected_prompt(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);

    // Verify the prompt exists
    if !settings.post_process_prompts.iter().any(|p| p.id == id) {
        return Err(format!("Prompt with id '{}' not found", id));
    }

    settings.post_process_selected_prompt_id = Some(id);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_lazy_stream_close_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.lazy_stream_close = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_normalize_audio_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.normalize_audio_on_export = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

/// Update the export `loudness_target` setting (R-003 / AC-001-a).
///
/// Frontend sends the enum string ("off" / "podcast_-16" /
/// "streaming_-14"); the backend never receives a `loudnorm=...` filter
/// from TS — that is built only by
/// `managers::splice::loudness::build_loudnorm_filter`.
#[tauri::command]
#[specta::specta]
pub fn change_loudness_target_setting(
    app: AppHandle,
    target: settings::LoudnessTarget,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.loudness_target = target;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_export_volume_db_setting(app: AppHandle, volume_db: f32) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.export_volume_db = volume_db.clamp(-12.0, 12.0);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_export_fade_in_ms_setting(app: AppHandle, fade_in_ms: u32) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.export_fade_in_ms = fade_in_ms;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_export_fade_out_ms_setting(app: AppHandle, fade_out_ms: u32) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.export_fade_out_ms = fade_out_ms;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_app_language_setting(app: AppHandle, language: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.app_language = language.clone();
    settings::write_settings(&app, settings);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_whisper_accelerator_setting(
    app: AppHandle,
    accelerator: settings::WhisperAcceleratorSetting,
) -> Result<(), String> {
    let mut s = settings::get_settings(&app);
    s.whisper_accelerator = accelerator;
    apply_and_reload_accelerator(&app, s);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_ort_accelerator_setting(
    app: AppHandle,
    accelerator: settings::OrtAcceleratorSetting,
) -> Result<(), String> {
    let mut s = settings::get_settings(&app);
    s.ort_accelerator = accelerator;
    apply_and_reload_accelerator(&app, s);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_whisper_gpu_device(app: AppHandle, device: i32) -> Result<(), String> {
    let mut s = settings::get_settings(&app);
    s.whisper_gpu_device = device;
    apply_and_reload_accelerator(&app, s);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_available_accelerators() -> crate::managers::transcription::AvailableAccelerators {
    tauri::async_runtime::spawn_blocking(crate::managers::transcription::get_available_accelerators)
        .await
        .expect("get_available_accelerators panicked")
}
