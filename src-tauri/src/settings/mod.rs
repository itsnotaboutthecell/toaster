//! Settings module.
//!
//! Behavior-preserving split of the former monolithic `settings.rs`:
//! - [`types`]    — enums, structs, and trait impls for the on-disk schema
//! - [`defaults`] — `default_*` factories + `get_default_settings`
//! - [`io`]       — Tauri store read/write + convenience accessors
//!
//! External callers keep using `crate::settings::<Name>` paths; every
//! previously-public item is re-exported below.

pub const SETTINGS_STORE_PATH: &str = "settings_store.json";

mod defaults;
mod io;
mod types;

pub use defaults::{
    default_desktop_profile, default_mobile_profile, ensure_caption_defaults, get_default_settings,
};
pub use io::{get_settings, write_settings};
pub use types::{
    AppSettings, CaptionFontFamily, CaptionProfile, CaptionProfileSet, LogLevel,
    ModelUnloadTimeout, Orientation, OrtAcceleratorSetting, ProfileScope,
    VideoDims, WhisperAcceleratorSetting,
};

/// Known experimental feature keys. Each variant maps 1:1 to a
/// per-flag `bool` field on `AppSettings`. Keep in sync with the
/// frontend `experiments` registry in `src/lib/experiments.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentKey {
    LazyStreamClose,
}

/// Defence-in-depth getter for experimental booleans.
pub fn is_experiment_enabled(settings: &AppSettings, key: ExperimentKey) -> bool {
    if !settings.experimental_enabled {
        return false;
    }
    match key {
        ExperimentKey::LazyStreamClose => settings.lazy_stream_close,
    }
}

pub use crate::managers::splice::loudness::LoudnessTarget;

/// Migrate the legacy `normalize_audio_on_export` boolean into the new
/// `loudness_target` enum.
pub fn migrate_loudness_setting(
    legacy_normalize: Option<bool>,
    current_target: Option<LoudnessTarget>,
) -> LoudnessTarget {
    if let Some(target) = current_target {
        if target != LoudnessTarget::Off {
            return target;
        }
    }
    match legacy_normalize {
        Some(true) => LoudnessTarget::PodcastMinus16,
        Some(false) => LoudnessTarget::Off,
        None => current_target.unwrap_or(LoudnessTarget::Off),
    }
}

#[cfg(test)]
mod tests {
    use super::defaults::validate_settings;
    use super::*;

    #[test]
    fn experiment_getter_returns_false_when_master_disabled() {
        let mut settings = get_default_settings();
        settings.experimental_enabled = false;
        settings.lazy_stream_close = true;

        assert!(!is_experiment_enabled(
            &settings,
            ExperimentKey::LazyStreamClose
        ));
        assert!(settings.lazy_stream_close);
    }

    #[test]
    fn experiment_getter_returns_stored_value_when_master_enabled() {
        let mut settings = get_default_settings();
        settings.experimental_enabled = true;
        settings.lazy_stream_close = true;

        assert!(is_experiment_enabled(
            &settings,
            ExperimentKey::LazyStreamClose
        ));
    }

    #[test]
    fn experimental_enabled_defaults_to_false_on_fresh_install() {
        let settings = get_default_settings();
        assert!(!settings.experimental_enabled);
    }

    #[test]
    fn test_validate_settings_clamps_position() {
        let mut s = get_default_settings();
        s.caption_position = 150;
        validate_settings(&mut s);
        assert_eq!(s.caption_position, 90);
    }

    #[test]
    fn test_validate_settings_fixes_invalid_color() {
        let mut s = get_default_settings();
        s.caption_text_color = "not-a-color".to_string();
        validate_settings(&mut s);
        assert_eq!(s.caption_text_color, "#FFFFFF");
    }

    #[test]
    fn test_validate_settings_allows_valid_colors() {
        let mut s = get_default_settings();
        s.caption_text_color = "#FF0000".to_string();
        s.caption_bg_color = "#00FF00AA".to_string();
        validate_settings(&mut s);
        assert_eq!(s.caption_text_color, "#FF0000");
        assert_eq!(s.caption_bg_color, "#00FF00AA");
    }

    #[test]
    fn test_validate_settings_clamps_volume() {
        let mut s = get_default_settings();
        s.export_volume_db = 100.0;
        validate_settings(&mut s);
        assert_eq!(s.export_volume_db, 24.0);
    }

    #[test]
    fn test_settings_version_present() {
        let s = get_default_settings();
        assert_eq!(s.settings_version, 1);
    }

    #[test]
    fn caption_migration_seeds_profiles_from_flat_fields() {
        let mut s = get_default_settings();
        s.caption_profiles_was_migrated = false;
        s.caption_font_size = 33;
        s.caption_position = 77;
        s.caption_max_width_percent = 55;
        s.caption_bg_color = "#112233AA".to_string();

        let changed = ensure_caption_defaults(&mut s);
        assert!(changed, "first migration should mutate");
        assert!(s.caption_profiles_was_migrated);
        assert_eq!(s.caption_profiles.desktop.font_size, 33);
        assert_eq!(s.caption_profiles.desktop.position, 77);
        assert_eq!(s.caption_profiles.desktop.max_width_percent, 55);
        assert_eq!(s.caption_profiles.desktop.bg_color, "#112233AA");
        assert_eq!(s.caption_profiles.desktop, s.caption_profiles.mobile);
    }

    #[test]
    fn caption_migration_idempotent() {
        let mut s = get_default_settings();
        s.caption_profiles_was_migrated = false;
        assert!(ensure_caption_defaults(&mut s));

        s.caption_profiles.mobile.font_size = 999;

        let second = ensure_caption_defaults(&mut s);
        assert!(!second);
        assert_eq!(s.caption_profiles.mobile.font_size, 999);
    }

    #[test]
    fn caption_profiles_survive_full_settings_roundtrip() {
        let mut s = get_default_settings();
        s.caption_profiles.desktop.font_size = 61;
        s.caption_profiles.mobile.padding_x_px = 22;

        let json = serde_json::to_value(&s).expect("serialize");
        let back: AppSettings = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back.caption_profiles.desktop.font_size, 61);
        assert_eq!(back.caption_profiles.mobile.padding_x_px, 22);
        assert!(back.caption_profiles_was_migrated);
    }

    #[test]
    fn migrate_loudness_setting_maps_legacy_boolean() {
        assert_eq!(
            migrate_loudness_setting(Some(true), None),
            LoudnessTarget::PodcastMinus16
        );
        assert_eq!(
            migrate_loudness_setting(Some(true), Some(LoudnessTarget::Off)),
            LoudnessTarget::PodcastMinus16
        );
        assert_eq!(
            migrate_loudness_setting(Some(false), None),
            LoudnessTarget::Off
        );
        assert_eq!(
            migrate_loudness_setting(None, Some(LoudnessTarget::StreamingMinus14)),
            LoudnessTarget::StreamingMinus14
        );
        assert_eq!(
            migrate_loudness_setting(Some(true), Some(LoudnessTarget::StreamingMinus14)),
            LoudnessTarget::StreamingMinus14
        );
        assert_eq!(migrate_loudness_setting(None, None), LoudnessTarget::Off);
    }
}
