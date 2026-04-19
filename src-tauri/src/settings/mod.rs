//! Settings module.
//!
//! Behavior-preserving split of the former monolithic `settings.rs`:
//! - [`types`]    — enums, structs, and trait impls for the on-disk schema
//! - [`defaults`] — `default_*` factories + `get_default_settings` +
//!   `ensure_post_process_defaults` migration
//! - [`sanitize`] — validation helpers for post-process provider inputs
//! - [`io`]       — Tauri store read/write + convenience accessors
//!
//! External callers keep using `crate::settings::<Name>` paths; every
//! previously-public item is re-exported below.

pub const OLLAMA_PROVIDER_ID: &str = "ollama";
pub const LM_STUDIO_PROVIDER_ID: &str = "lm_studio";
pub const CUSTOM_LOCAL_PROVIDER_ID: &str = "custom";
/// Provider ID for the in-process local-GGUF path (Feature B,
/// `local-llm-model-catalog`). When this is the active provider and
/// `local_llm_model_id` is `Some(_)`, the cleanup dispatcher routes
/// through `managers::llm::LlmManager` instead of the HTTP client.
pub const LOCAL_GGUF_PROVIDER_ID: &str = "local";
pub const SETTINGS_STORE_PATH: &str = "settings_store.json";

mod defaults;
mod io;
mod sanitize;
mod types;

pub use defaults::{
    default_desktop_profile, default_mobile_profile, ensure_caption_defaults,
    get_default_settings,
};
pub use io::{get_settings, write_settings};
pub use sanitize::{
    is_local_post_process_provider, sanitize_local_post_process_base_url,
    sanitize_post_process_model,
};
pub use types::{
    AppSettings, CaptionFontFamily, CaptionProfile, CaptionProfileSet, LLMPrompt, LogLevel,
    ModelUnloadTimeout, Orientation, OrtAcceleratorSetting, PostProcessProvider, ProfileScope,
    VideoDims, WhisperAcceleratorSetting,
};

/// Known experimental feature keys. Each variant maps 1:1 to a
/// per-flag `bool` field on `AppSettings`. Keep in sync with the
/// frontend `experiments` registry in `src/lib/experiments.ts` —
/// the two sides are the "two mouths of the same SSOT rule" noted
/// in the Blueprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentKey {
    SimplifyMode,
    LazyStreamClose,
}

/// Defence-in-depth getter for experimental booleans.
///
/// When the master toggle `experimental_enabled` is `false`, every
/// experiment reads as `false` regardless of the stored per-flag
/// value. When the master is `true`, the stored per-flag value is
/// returned verbatim. Stored values are never mutated here — that
/// is the whole point of the gating layer (user opt-ins survive
/// flip-flopping the master).
pub fn is_experiment_enabled(settings: &AppSettings, key: ExperimentKey) -> bool {
    if !settings.experimental_enabled {
        return false;
    }
    match key {
        ExperimentKey::SimplifyMode => settings.experimental_simplify_mode,
        ExperimentKey::LazyStreamClose => settings.lazy_stream_close,
    }
}

// Re-export the loudness enum at the settings root so callers writing
// `crate::settings::LoudnessTarget` keep working alongside the canonical
// `crate::managers::splice::loudness::LoudnessTarget` path.
pub use crate::managers::splice::loudness::LoudnessTarget;

/// Migrate the legacy `normalize_audio_on_export` boolean into the new
/// `loudness_target` enum.
///
/// Behavior:
/// - If `loudness_target` is already set to a non-default value, it
///   wins (a present-day user choice always takes precedence over
///   legacy boolean state).
/// - Otherwise, `normalize_audio_on_export = true` becomes
///   `LoudnessTarget::PodcastMinus16` (preserves -16 LUFS behavior of
///   the old hard-coded filter at `commands/waveform/mod.rs:121`),
///   and `false` becomes `LoudnessTarget::Off`.
///
/// AC-004-a / AC-004-b: existing settings files migrate cleanly on
/// first load.
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
    use super::types::SecretMap;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn experiment_getter_returns_false_when_master_disabled() {
        let mut settings = get_default_settings();
        settings.experimental_enabled = false;
        settings.experimental_simplify_mode = true;
        settings.lazy_stream_close = true;

        assert!(
            !is_experiment_enabled(&settings, ExperimentKey::SimplifyMode),
            "simplify mode must read false when master toggle is off, even if stored value is true"
        );
        assert!(
            !is_experiment_enabled(&settings, ExperimentKey::LazyStreamClose),
            "lazy stream close must read false when master toggle is off"
        );

        // Defence-in-depth: stored per-flag values are preserved
        // across the master flip so the user's prior opt-in comes
        // back when they re-enable.
        assert!(settings.experimental_simplify_mode);
        assert!(settings.lazy_stream_close);
    }

    #[test]
    fn experiment_getter_returns_stored_value_when_master_enabled() {
        let mut settings = get_default_settings();
        settings.experimental_enabled = true;
        settings.experimental_simplify_mode = true;
        settings.lazy_stream_close = false;

        assert!(is_experiment_enabled(
            &settings,
            ExperimentKey::SimplifyMode
        ));
        assert!(!is_experiment_enabled(
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
    fn default_settings_disable_experimental_simplify_mode() {
        let settings = get_default_settings();
        assert!(!settings.experimental_simplify_mode);
    }

    #[test]
    fn debug_output_redacts_api_keys() {
        let mut settings = get_default_settings();
        settings
            .post_process_api_keys
            .insert("openai".to_string(), "sk-proj-secret-key-12345".to_string());
        settings.post_process_api_keys.insert(
            "anthropic".to_string(),
            "sk-ant-secret-key-67890".to_string(),
        );
        settings
            .post_process_api_keys
            .insert("empty_provider".to_string(), "".to_string());

        let debug_output = format!("{:?}", settings);

        assert!(!debug_output.contains("sk-proj-secret-key-12345"));
        assert!(!debug_output.contains("sk-ant-secret-key-67890"));
        assert!(debug_output.contains("[REDACTED]"));
    }

    #[test]
    fn secret_map_debug_redacts_values() {
        let map = SecretMap(HashMap::from([("key".into(), "secret".into())]));
        let out = format!("{:?}", map);
        assert!(!out.contains("secret"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn default_post_process_provider_prefers_local_ollama() {
        let settings = get_default_settings();
        assert_eq!(settings.post_process_provider_id, OLLAMA_PROVIDER_ID);

        let ollama = settings
            .post_process_providers
            .iter()
            .find(|provider| provider.id == OLLAMA_PROVIDER_ID)
            .expect("ollama provider should exist");
        assert!(ollama.local_only);
        assert!(!ollama.requires_api_key);
    }

    #[test]
    fn sanitize_local_base_url_rejects_non_loopback_hosts() {
        let result = sanitize_local_post_process_base_url("https://example.com/v1");
        assert!(result.is_err());
    }

    #[test]
    fn sanitize_local_base_url_normalizes_trailing_slash() {
        let result = sanitize_local_post_process_base_url("http://127.0.0.1:11434/v1/");
        assert_eq!(
            result.expect("expected valid loopback URL"),
            "http://127.0.0.1:11434/v1"
        );
    }

    #[test]
    fn sanitize_post_process_model_rejects_control_characters() {
        let result = sanitize_post_process_model("llama3\nbad");
        assert!(result.is_err());
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
        // AC-002-a: after a first load with the migration latch off,
        // both desktop + mobile profiles snapshot the flat-field values.
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
        // Both orientations seed from the same flat fields.
        assert_eq!(s.caption_profiles.desktop, s.caption_profiles.mobile);
    }

    #[test]
    fn caption_migration_idempotent() {
        // AC-002-b: running ensure_caption_defaults twice does not
        // overwrite user-tweaked mobile values.
        let mut s = get_default_settings();
        s.caption_profiles_was_migrated = false;
        assert!(ensure_caption_defaults(&mut s));

        // Simulate a user tweak to the mobile profile after first load.
        s.caption_profiles.mobile.font_size = 999;

        let second = ensure_caption_defaults(&mut s);
        assert!(!second, "idempotent: second call must not mutate");
        assert_eq!(s.caption_profiles.mobile.font_size, 999);
    }

    #[test]
    fn caption_profiles_survive_full_settings_roundtrip() {
        // AC-002-c: serde round-trip through the Settings store shape
        // keeps caption_profiles intact.
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
        // AC-004-a: (true, _) -> "podcast_-16", (false, _) -> "off",
        // (absent, present) -> present.
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
        // Present non-default user choice always wins over legacy bool.
        assert_eq!(
            migrate_loudness_setting(Some(true), Some(LoudnessTarget::StreamingMinus14)),
            LoudnessTarget::StreamingMinus14
        );
        // Absent on both sides: stay Off.
        assert_eq!(migrate_loudness_setting(None, None), LoudnessTarget::Off);
    }
}
