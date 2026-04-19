//! App settings Tauri commands.
//!
//! These commands back the editor's settings UI (captions, export, language,
//! whisper accelerator, post-process providers, filler words, etc.). They were
//! historically housed in the `shortcut` module because Handy bundled
//! everything under keyboard-shortcut handling; they have nothing to do with
//! keyboard shortcuts. Moved here by p1-shortcut-split-settings.

use tauri::{AppHandle, Emitter, Manager};

use crate::settings::{self, CaptionFontFamily};

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
pub fn change_caption_font_family_setting(
    app: AppHandle,
    family: CaptionFontFamily,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.caption_font_family = family;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_caption_radius_px_setting(app: AppHandle, radius: u32) -> Result<(), String> {
    // mirrors settings::defaults::ensure_caption_defaults:589 (upper bound 64).
    let mut settings = settings::get_settings(&app);
    settings.caption_radius_px = radius.min(64);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_caption_padding_x_px_setting(app: AppHandle, padding: u32) -> Result<(), String> {
    // mirrors settings::defaults::ensure_caption_defaults:593 (upper bound 128).
    let mut settings = settings::get_settings(&app);
    settings.caption_padding_x_px = padding.min(128);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_caption_padding_y_px_setting(app: AppHandle, padding: u32) -> Result<(), String> {
    // mirrors settings::defaults::ensure_caption_defaults:597 (upper bound 128).
    let mut settings = settings::get_settings(&app);
    settings.caption_padding_y_px = padding.min(128);
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_caption_max_width_percent_setting(
    app: AppHandle,
    percent: u32,
) -> Result<(), String> {
    // mirrors settings::defaults::ensure_caption_defaults:601 (valid range 20..=100).
    let mut settings = settings::get_settings(&app);
    settings.caption_max_width_percent = percent.clamp(20, 100);
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

/// Update the default export format used when the source media carries
/// a video stream (Round-6 Phase D). Frontend sends the enum; backend
/// owns codec/extension mapping via `export_format_codec_map`.
// Round-8: `change_export_format_video_setting` + `change_export_format_audio_setting`
// removed. Format selection moved from persisted settings into the
// Editor's per-project ExportMenu; the backend picks a source-type-
// default when no `format_override` is supplied.

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
