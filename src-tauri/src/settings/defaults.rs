//! Default value factories for `AppSettings` and nested types.
//!
//! Every `#[serde(default = "...")]` attribute in `super::types` points to a
//! function here. A handful of more structural builders (`get_default_settings`,
//! `ensure_post_process_defaults`) also live here because they compose the
//! smaller helpers.

use super::types::{
    AppSettings, CaptionFontFamily, CaptionProfile, CaptionProfileSet, LogLevel,
    ModelUnloadTimeout, OrtAcceleratorSetting, ShortcutBinding,
    WhisperAcceleratorSetting,
};
use std::collections::HashMap;

pub(super) fn default_model() -> String {
    "".to_string()
}

pub(super) fn default_settings_version() -> u32 {
    1
}

pub(super) fn default_caption_font_size() -> u32 {
    // 40 px on a 1080p frame is ~3.7 % of frame height, aligning with
    // broadcast/YouTube caption norms. The previous 24 px (~2.2 %) was
    // readable but felt tiny on the default export size.
    40
}

pub(super) fn default_caption_bg_color() -> String {
    "#000000B3".to_string()
}

pub(super) fn default_caption_text_color() -> String {
    "#FFFFFF".to_string()
}

pub(super) fn default_caption_position() -> u32 {
    90
}

pub(super) fn default_caption_radius_px() -> u32 {
    // Export uses libass BorderStyle=3 (opaque rectangle); preview
    // matches with `borderRadius: 0`. Radius is kept as a settable
    // field for forward compatibility but defaults to 0.
    0
}

pub(super) fn default_caption_padding_x_px() -> u32 {
    12
}

pub(super) fn default_caption_padding_y_px() -> u32 {
    4
}

pub(super) fn default_caption_max_width_percent() -> u32 {
    90
}

/// Default desktop profile. Matches the existing flat-field defaults so
/// users upgrading don't see a visual change on landscape content.
pub fn default_desktop_profile() -> CaptionProfile {
    CaptionProfile {
        font_size: default_caption_font_size(),
        bg_color: default_caption_bg_color(),
        text_color: default_caption_text_color(),
        position: default_caption_position(),
        font_family: CaptionFontFamily::default(),
        radius_px: default_caption_radius_px(),
        padding_x_px: default_caption_padding_x_px(),
        padding_y_px: default_caption_padding_y_px(),
        max_width_percent: default_caption_max_width_percent(),
    }
}

/// Default mobile profile. Differs from desktop on several axes
/// (Blueprint §Default profile values): bigger text, higher anchor
/// (thumbs sit at the bottom), narrower max-width, rounded box,
/// slightly more padding.
pub fn default_mobile_profile() -> CaptionProfile {
    CaptionProfile {
        font_size: 48,
        bg_color: default_caption_bg_color(),
        text_color: default_caption_text_color(),
        position: 80,
        font_family: CaptionFontFamily::default(),
        radius_px: 8,
        padding_x_px: 14,
        padding_y_px: 6,
        max_width_percent: 80,
    }
}

pub(super) fn default_caption_profiles() -> CaptionProfileSet {
    CaptionProfileSet {
        desktop: default_desktop_profile(),
        mobile: default_mobile_profile(),
    }
}

pub(super) fn default_preferred_output_sample_rate() -> u32 {
    48_000
}

pub(super) fn default_translate_to_english() -> bool {
    false
}

pub(super) fn default_start_hidden() -> bool {
    false
}

pub(super) fn default_update_checks_enabled() -> bool {
    true
}

pub(super) fn default_selected_language() -> String {
    "auto".to_string()
}

pub(super) fn default_debug_mode() -> bool {
    false
}

pub(super) fn default_log_level() -> LogLevel {
    LogLevel::Debug
}

pub(super) fn default_word_correction_threshold() -> f64 {
    0.18
}

pub(super) fn default_app_language() -> String {
    tauri_plugin_os::locale()
        .map(|l| l.replace('_', "-"))
        .unwrap_or_else(|| "en".to_string())
}

pub(super) fn default_whisper_gpu_device() -> i32 {
    -1 // auto
}

/// Seed `caption_profiles` from the legacy flat `caption_*` fields on
/// first load after upgrade. Idempotent via
/// `caption_profiles_was_migrated`. Returns `true` if any mutation
/// happened (caller persists).
pub fn ensure_caption_defaults(settings: &mut AppSettings) -> bool {
    if settings.caption_profiles_was_migrated {
        return false;
    }

    // Snapshot the current flat fields into a profile — this is what
    // the user saw before the upgrade. Seed both orientations with the
    // same values so nothing visually changes; the user tweaks the
    // mobile profile later and the migration latch prevents overwrite.
    let flat = CaptionProfile {
        font_size: settings.caption_font_size,
        bg_color: settings.caption_bg_color.clone(),
        text_color: settings.caption_text_color.clone(),
        position: settings.caption_position,
        font_family: settings.caption_font_family,
        radius_px: settings.caption_radius_px,
        padding_x_px: settings.caption_padding_x_px,
        padding_y_px: settings.caption_padding_y_px,
        max_width_percent: settings.caption_max_width_percent,
    };

    settings.caption_profiles = CaptionProfileSet {
        desktop: flat.clone(),
        mobile: flat,
    };
    settings.caption_profiles_was_migrated = true;
    true
}

pub fn get_default_settings() -> AppSettings {
    #[cfg(target_os = "windows")]
    let default_shortcut = "ctrl+space";
    #[cfg(target_os = "macos")]
    let default_shortcut = "option+space";
    #[cfg(target_os = "linux")]
    let default_shortcut = "ctrl+space";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let default_shortcut = "alt+space";

    let mut bindings = HashMap::new();
    bindings.insert(
        "transcribe".to_string(),
        ShortcutBinding {
            id: "transcribe".to_string(),
            name: "Transcribe".to_string(),
            description: "Converts your speech into text.".to_string(),
            default_binding: default_shortcut.to_string(),
            current_binding: default_shortcut.to_string(),
        },
    );
    bindings.insert(
        "cancel".to_string(),
        ShortcutBinding {
            id: "cancel".to_string(),
            name: "Cancel".to_string(),
            description: "Cancels the current recording.".to_string(),
            default_binding: "escape".to_string(),
            current_binding: "escape".to_string(),
        },
    );

    AppSettings {
        bindings,
        start_hidden: default_start_hidden(),
        update_checks_enabled: default_update_checks_enabled(),
        selected_model: "".to_string(),
        selected_output_device: None,
        preferred_output_sample_rate: default_preferred_output_sample_rate(),
        translate_to_english: false,
        selected_language: "auto".to_string(),
        debug_mode: false,
        log_level: default_log_level(),
        custom_words: Vec::new(),
        model_unload_timeout: ModelUnloadTimeout::default(),
        word_correction_threshold: default_word_correction_threshold(),
        app_language: default_app_language(),
        experimental_enabled: false,
        lazy_stream_close: false,
        custom_filler_words: Some(vec![
            "um".to_string(),
            "uh".to_string(),
            "uh huh".to_string(),
            "hmm".to_string(),
            "mm".to_string(),
            "mhm".to_string(),
            "er".to_string(),
            "ah".to_string(),
            "like".to_string(),
            "you know".to_string(),
            "I mean".to_string(),
            "basically".to_string(),
            "actually".to_string(),
            "literally".to_string(),
            "so".to_string(),
            "right".to_string(),
            "kind of".to_string(),
            "sort of".to_string(),
        ]),
        whisper_accelerator: WhisperAcceleratorSetting::default(),
        ort_accelerator: OrtAcceleratorSetting::default(),
        whisper_gpu_device: default_whisper_gpu_device(),
        normalize_audio_on_export: false,
        loudness_target: crate::managers::splice::loudness::LoudnessTarget::Off,
        export_volume_db: 0.0,
        export_fade_in_ms: 0,
        export_fade_out_ms: 0,
        caption_font_size: default_caption_font_size(),
        caption_bg_color: default_caption_bg_color(),
        caption_text_color: default_caption_text_color(),
        caption_position: default_caption_position(),
        caption_font_family: CaptionFontFamily::default(),
        caption_radius_px: default_caption_radius_px(),
        caption_padding_x_px: default_caption_padding_x_px(),
        caption_padding_y_px: default_caption_padding_y_px(),
        caption_max_width_percent: default_caption_max_width_percent(),
        caption_profiles: default_caption_profiles(),
        caption_profiles_was_migrated: true,
        settings_version: default_settings_version(),
    }
}

#[cfg(test)]
pub(super) fn validate_settings(settings: &mut AppSettings) -> bool {
    let mut changed = false;

    if settings.caption_position > 100 {
        settings.caption_position = default_caption_position();
        changed = true;
    }
    if settings.caption_font_size < 8 || settings.caption_font_size > 120 {
        settings.caption_font_size = default_caption_font_size();
        changed = true;
    }

    let is_valid_hex = |s: &str| -> bool {
        let h = s.trim_start_matches('#');
        (h.len() == 6 || h.len() == 8) && h.chars().all(|c| c.is_ascii_hexdigit())
    };
    if !is_valid_hex(&settings.caption_text_color) {
        settings.caption_text_color = default_caption_text_color();
        changed = true;
    }
    if !is_valid_hex(&settings.caption_bg_color) {
        settings.caption_bg_color = default_caption_bg_color();
        changed = true;
    }

    if settings.caption_radius_px > 64 {
        settings.caption_radius_px = default_caption_radius_px();
        changed = true;
    }
    if settings.caption_padding_x_px > 128 {
        settings.caption_padding_x_px = default_caption_padding_x_px();
        changed = true;
    }
    if settings.caption_padding_y_px > 128 {
        settings.caption_padding_y_px = default_caption_padding_y_px();
        changed = true;
    }
    if settings.caption_max_width_percent < 20 || settings.caption_max_width_percent > 100 {
        settings.caption_max_width_percent = default_caption_max_width_percent();
        changed = true;
    }

    settings.export_volume_db = settings.export_volume_db.clamp(-60.0, 24.0);
    settings.export_fade_in_ms = settings.export_fade_in_ms.min(30_000);
    settings.export_fade_out_ms = settings.export_fade_out_ms.min(30_000);

    changed
}
