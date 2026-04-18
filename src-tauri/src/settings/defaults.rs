//! Default value factories for `AppSettings` and nested types.
//!
//! Every `#[serde(default = "...")]` attribute in `super::types` points to a
//! function here. A handful of more structural builders (`get_default_settings`,
//! `ensure_post_process_defaults`) also live here because they compose the
//! smaller helpers.

use super::types::{
    AppSettings, CaptionFontFamily, LLMPrompt, LogLevel, ModelUnloadTimeout, OrtAcceleratorSetting,
    PostProcessProvider, RecordingRetentionPeriod, SecretMap, ShortcutBinding,
    WhisperAcceleratorSetting,
};
use super::{
    APPLE_INTELLIGENCE_DEFAULT_MODEL_ID, APPLE_INTELLIGENCE_PROVIDER_ID, CUSTOM_LOCAL_PROVIDER_ID,
    LM_STUDIO_PROVIDER_ID, OLLAMA_PROVIDER_ID,
};
use log::debug;
use std::collections::HashMap;

pub(super) fn default_model() -> String {
    "".to_string()
}

pub(super) fn default_settings_version() -> u32 {
    1
}

pub(super) fn default_caption_font_size() -> u32 {
    24
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
    4
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

pub(super) fn default_history_limit() -> usize {
    5
}

pub(super) fn default_recording_retention_period() -> RecordingRetentionPeriod {
    RecordingRetentionPeriod::PreserveLimit
}

pub(super) fn default_post_process_enabled() -> bool {
    false
}

pub(super) fn default_post_process_provider_requires_api_key() -> bool {
    true
}

pub(super) fn default_app_language() -> String {
    tauri_plugin_os::locale()
        .map(|l| l.replace('_', "-"))
        .unwrap_or_else(|| "en".to_string())
}

pub(super) fn default_post_process_provider_id() -> String {
    OLLAMA_PROVIDER_ID.to_string()
}

pub(super) fn default_post_process_providers() -> Vec<PostProcessProvider> {
    let providers = vec![
        PostProcessProvider {
            id: OLLAMA_PROVIDER_ID.to_string(),
            label: "Ollama (Local)".to_string(),
            base_url: "http://127.0.0.1:11434/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: true,
            local_only: true,
            requires_api_key: false,
        },
        PostProcessProvider {
            id: LM_STUDIO_PROVIDER_ID.to_string(),
            label: "LM Studio (Local)".to_string(),
            base_url: "http://127.0.0.1:1234/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: true,
            local_only: true,
            requires_api_key: false,
        },
        PostProcessProvider {
            id: CUSTOM_LOCAL_PROVIDER_ID.to_string(),
            label: "OpenAI-Compatible (Local)".to_string(),
            base_url: "http://127.0.0.1:11434/v1".to_string(),
            allow_base_url_edit: true,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: true,
            local_only: true,
            requires_api_key: false,
        },
    ];

    // Note: We always include Apple Intelligence on macOS ARM64 without checking availability
    // at startup. The availability check is deferred to when the user actually tries to use it.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        providers.push(PostProcessProvider {
            id: APPLE_INTELLIGENCE_PROVIDER_ID.to_string(),
            label: "Apple Intelligence".to_string(),
            base_url: "apple-intelligence://local".to_string(),
            allow_base_url_edit: false,
            models_endpoint: None,
            supports_structured_output: true,
            local_only: true,
            requires_api_key: false,
        });
    }

    providers
}

pub(super) fn default_post_process_api_keys() -> SecretMap {
    let mut map = HashMap::new();
    for provider in default_post_process_providers() {
        map.insert(provider.id, String::new());
    }
    SecretMap(map)
}

fn default_model_for_provider(provider_id: &str) -> String {
    if provider_id == APPLE_INTELLIGENCE_PROVIDER_ID {
        return APPLE_INTELLIGENCE_DEFAULT_MODEL_ID.to_string();
    }
    String::new()
}

pub(super) fn default_post_process_models() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for provider in default_post_process_providers() {
        map.insert(
            provider.id.clone(),
            default_model_for_provider(&provider.id),
        );
    }
    map
}

pub(super) fn default_post_process_prompts() -> Vec<LLMPrompt> {
    vec![LLMPrompt {
        id: "default_improve_transcriptions".to_string(),
        name: "Improve Transcriptions".to_string(),
        prompt: "Clean this transcript:\n1. Fix spelling, capitalization, and punctuation errors\n2. Convert number words to digits (twenty-five → 25, ten percent → 10%, five dollars → $5)\n3. Replace spoken punctuation with symbols (period → ., comma → ,, question mark → ?)\n4. Remove filler words (um, uh, like as filler)\n5. Keep the language in the original version (if it was french, keep it in french for example)\n6. Preserve numbers/currency/symbol tokens exactly when they already exist in the transcript\n\nPreserve exact meaning and word order. Do not paraphrase or reorder content.\n\nReturn only the cleaned transcript.\n\nTranscript:\n${output}".to_string(),
    }]
}

pub(super) fn default_whisper_gpu_device() -> i32 {
    -1 // auto
}

/// Migrate / seed post-process provider settings. Returns `true` if any
/// field was mutated, signaling the caller to persist the new state.
pub fn ensure_post_process_defaults(settings: &mut AppSettings) -> bool {
    let mut changed = false;

    // Migration: Toaster is local-only for post-processing. Strip any cloud
    // providers inherited from Handy's defaults (openai, anthropic, groq,
    // cerebras, openrouter, zai, bedrock_mantle). The seed loop below will
    // re-add only the ones in default_post_process_providers().
    const LEGACY_CLOUD_PROVIDER_IDS: &[&str] = &[
        "openai",
        "anthropic",
        "groq",
        "cerebras",
        "openrouter",
        "zai",
        "bedrock_mantle",
    ];
    let before_len = settings.post_process_providers.len();
    settings
        .post_process_providers
        .retain(|p| !LEGACY_CLOUD_PROVIDER_IDS.contains(&p.id.as_str()));
    if settings.post_process_providers.len() != before_len {
        for id in LEGACY_CLOUD_PROVIDER_IDS {
            settings.post_process_api_keys.remove(*id);
            settings.post_process_models.remove(*id);
        }
        debug!("Migrated: removed cloud LLM providers from settings (Toaster is local-only)");
        changed = true;
    }

    for provider in default_post_process_providers() {
        // Use match to do a single lookup - either sync existing or add new
        match settings
            .post_process_providers
            .iter_mut()
            .find(|p| p.id == provider.id)
        {
            Some(existing) => {
                // Sync supports_structured_output field for existing providers (migration)
                if existing.supports_structured_output != provider.supports_structured_output {
                    debug!(
                        "Updating supports_structured_output for provider '{}' from {} to {}",
                        provider.id,
                        existing.supports_structured_output,
                        provider.supports_structured_output
                    );
                    existing.supports_structured_output = provider.supports_structured_output;
                    changed = true;
                }

                if existing.allow_base_url_edit != provider.allow_base_url_edit {
                    existing.allow_base_url_edit = provider.allow_base_url_edit;
                    changed = true;
                }

                if existing.models_endpoint != provider.models_endpoint {
                    existing.models_endpoint = provider.models_endpoint.clone();
                    changed = true;
                }

                if existing.local_only != provider.local_only {
                    existing.local_only = provider.local_only;
                    changed = true;
                }

                if existing.requires_api_key != provider.requires_api_key {
                    existing.requires_api_key = provider.requires_api_key;
                    changed = true;
                }

                // Local-only boundary enforcement (C2): if a local provider's
                // base_url was tampered with (malicious/malformed settings
                // import, manual JSON edit), reset it to the built-in default
                // loopback URL so runtime calls cannot exfil transcripts.
                if provider.local_only
                    && provider.id != APPLE_INTELLIGENCE_PROVIDER_ID
                    && !super::sanitize::base_url_is_loopback(&existing.base_url)
                {
                    debug!(
                        "Local-only boundary: provider '{}' had non-loopback base_url '{}'; resetting to default '{}'",
                        provider.id, existing.base_url, provider.base_url
                    );
                    existing.base_url = provider.base_url.clone();
                    changed = true;
                }
            }
            None => {
                // Provider doesn't exist, add it
                settings.post_process_providers.push(provider.clone());
                changed = true;
            }
        }

        if !settings.post_process_api_keys.contains_key(&provider.id) {
            settings
                .post_process_api_keys
                .insert(provider.id.clone(), String::new());
            changed = true;
        }

        let default_model = default_model_for_provider(&provider.id);
        match settings.post_process_models.get_mut(&provider.id) {
            Some(existing) => {
                if existing.is_empty() && !default_model.is_empty() {
                    *existing = default_model.clone();
                    changed = true;
                }
            }
            None => {
                settings
                    .post_process_models
                    .insert(provider.id.clone(), default_model);
                changed = true;
            }
        }
    }

    if !settings
        .post_process_providers
        .iter()
        .any(|provider| provider.id == settings.post_process_provider_id)
    {
        settings.post_process_provider_id = default_post_process_provider_id();
        changed = true;
    }

    changed
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
    #[cfg(target_os = "windows")]
    let default_post_process_shortcut = "ctrl+shift+space";
    #[cfg(target_os = "macos")]
    let default_post_process_shortcut = "option+shift+space";
    #[cfg(target_os = "linux")]
    let default_post_process_shortcut = "ctrl+shift+space";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let default_post_process_shortcut = "alt+shift+space";

    bindings.insert(
        "transcribe_with_post_process".to_string(),
        ShortcutBinding {
            id: "transcribe_with_post_process".to_string(),
            name: "Transcribe with Post-Processing".to_string(),
            description: "Converts your speech into text and applies AI post-processing."
                .to_string(),
            default_binding: default_post_process_shortcut.to_string(),
            current_binding: default_post_process_shortcut.to_string(),
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
        history_limit: default_history_limit(),
        recording_retention_period: default_recording_retention_period(),
        post_process_enabled: default_post_process_enabled(),
        post_process_provider_id: default_post_process_provider_id(),
        post_process_providers: default_post_process_providers(),
        post_process_api_keys: default_post_process_api_keys(),
        post_process_models: default_post_process_models(),
        post_process_prompts: default_post_process_prompts(),
        post_process_selected_prompt_id: None,
        app_language: default_app_language(),
        experimental_enabled: false,
        experimental_simplify_mode: false,
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
