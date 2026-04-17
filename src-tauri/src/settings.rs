use log::{debug, warn};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::fmt;
use std::net::IpAddr;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

pub const APPLE_INTELLIGENCE_PROVIDER_ID: &str = "apple_intelligence";
pub const APPLE_INTELLIGENCE_DEFAULT_MODEL_ID: &str = "Apple Intelligence";
pub const OLLAMA_PROVIDER_ID: &str = "ollama";
pub const LM_STUDIO_PROVIDER_ID: &str = "lm_studio";
pub const CUSTOM_LOCAL_PROVIDER_ID: &str = "custom";

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

// Custom deserializer to handle both old numeric format (1-5) and new string format ("trace", "debug", etc.)
impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LogLevelVisitor;

        impl<'de> Visitor<'de> for LogLevelVisitor {
            type Value = LogLevel;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or integer representing log level")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<LogLevel, E> {
                match value.to_lowercase().as_str() {
                    "trace" => Ok(LogLevel::Trace),
                    "debug" => Ok(LogLevel::Debug),
                    "info" => Ok(LogLevel::Info),
                    "warn" => Ok(LogLevel::Warn),
                    "error" => Ok(LogLevel::Error),
                    _ => Err(E::unknown_variant(
                        value,
                        &["trace", "debug", "info", "warn", "error"],
                    )),
                }
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<LogLevel, E> {
                match value {
                    1 => Ok(LogLevel::Trace),
                    2 => Ok(LogLevel::Debug),
                    3 => Ok(LogLevel::Info),
                    4 => Ok(LogLevel::Warn),
                    5 => Ok(LogLevel::Error),
                    _ => Err(E::invalid_value(de::Unexpected::Unsigned(value), &"1-5")),
                }
            }
        }

        deserializer.deserialize_any(LogLevelVisitor)
    }
}

impl From<LogLevel> for tauri_plugin_log::LogLevel {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tauri_plugin_log::LogLevel::Trace,
            LogLevel::Debug => tauri_plugin_log::LogLevel::Debug,
            LogLevel::Info => tauri_plugin_log::LogLevel::Info,
            LogLevel::Warn => tauri_plugin_log::LogLevel::Warn,
            LogLevel::Error => tauri_plugin_log::LogLevel::Error,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct ShortcutBinding {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_binding: String,
    pub current_binding: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct LLMPrompt {
    pub id: String,
    pub name: String,
    pub prompt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct PostProcessProvider {
    pub id: String,
    pub label: String,
    pub base_url: String,
    #[serde(default)]
    pub allow_base_url_edit: bool,
    #[serde(default)]
    pub models_endpoint: Option<String>,
    #[serde(default)]
    pub supports_structured_output: bool,
    #[serde(default)]
    pub local_only: bool,
    #[serde(default = "default_post_process_provider_requires_api_key")]
    pub requires_api_key: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ModelUnloadTimeout {
    Never,
    Immediately,
    Min2,
    #[default]
    Min5,
    Min10,
    Min15,
    Hour1,
    Sec15, // Debug mode only
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum RecordingRetentionPeriod {
    Never,
    PreserveLimit,
    Days3,
    Weeks2,
    Months3,
}


impl ModelUnloadTimeout {
    pub fn to_minutes(self) -> Option<u64> {
        match self {
            ModelUnloadTimeout::Never => None,
            ModelUnloadTimeout::Immediately => Some(0), // Special case for immediate unloading
            ModelUnloadTimeout::Min2 => Some(2),
            ModelUnloadTimeout::Min5 => Some(5),
            ModelUnloadTimeout::Min10 => Some(10),
            ModelUnloadTimeout::Min15 => Some(15),
            ModelUnloadTimeout::Hour1 => Some(60),
            ModelUnloadTimeout::Sec15 => Some(0), // Special case for debug - handled separately
        }
    }

    pub fn to_seconds(self) -> Option<u64> {
        match self {
            ModelUnloadTimeout::Never => None,
            ModelUnloadTimeout::Immediately => Some(0), // Special case for immediate unloading
            ModelUnloadTimeout::Sec15 => Some(15),
            _ => self.to_minutes().map(|m| m * 60),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum WhisperAcceleratorSetting {
    #[default]
    Auto,
    Cpu,
    Gpu,
}


#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum OrtAcceleratorSetting {
    #[default]
    Auto,
    Cpu,
    Cuda,
    #[serde(rename = "directml")]
    DirectMl,
    Rocm,
}


#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub(crate) struct SecretMap(HashMap<String, String>);

impl fmt::Debug for SecretMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let redacted: HashMap<&String, &str> = self
            .0
            .iter()
            .map(|(k, v)| (k, if v.is_empty() { "" } else { "[REDACTED]" }))
            .collect();
        redacted.fmt(f)
    }
}

impl std::ops::Deref for SecretMap {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SecretMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/* still handy for composing the initial JSON in the store ------------- */
#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct AppSettings {
    #[serde(default)]
    pub bindings: HashMap<String, ShortcutBinding>,
    #[serde(default = "default_start_hidden")]
    pub start_hidden: bool,
    #[serde(default = "default_update_checks_enabled")]
    pub update_checks_enabled: bool,
    #[serde(default = "default_model")]
    pub selected_model: String,
    #[serde(default)]
    pub selected_output_device: Option<String>,
    #[serde(default = "default_preferred_output_sample_rate")]
    pub preferred_output_sample_rate: u32,
    #[serde(default = "default_translate_to_english")]
    pub translate_to_english: bool,
    #[serde(default = "default_selected_language")]
    pub selected_language: String,
    #[serde(default = "default_debug_mode")]
    pub debug_mode: bool,
    #[serde(default = "default_log_level")]
    pub log_level: LogLevel,
    #[serde(default)]
    pub custom_words: Vec<String>,
    #[serde(default)]
    pub model_unload_timeout: ModelUnloadTimeout,
    #[serde(default = "default_word_correction_threshold")]
    pub word_correction_threshold: f64,
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,
    #[serde(default = "default_recording_retention_period")]
    pub recording_retention_period: RecordingRetentionPeriod,
    #[serde(default = "default_post_process_enabled")]
    pub post_process_enabled: bool,
    #[serde(default = "default_post_process_provider_id")]
    pub post_process_provider_id: String,
    #[serde(default = "default_post_process_providers")]
    pub post_process_providers: Vec<PostProcessProvider>,
    #[serde(default = "default_post_process_api_keys")]
    pub post_process_api_keys: SecretMap,
    #[serde(default = "default_post_process_models")]
    pub post_process_models: HashMap<String, String>,
    #[serde(default = "default_post_process_prompts")]
    pub post_process_prompts: Vec<LLMPrompt>,
    #[serde(default)]
    pub post_process_selected_prompt_id: Option<String>,
    #[serde(default = "default_app_language")]
    pub app_language: String,
    #[serde(default)]
    pub experimental_enabled: bool,
    #[serde(default)]
    pub experimental_simplify_mode: bool,
    #[serde(default)]
    pub lazy_stream_close: bool,
    #[serde(default)]
    pub custom_filler_words: Option<Vec<String>>,
    #[serde(default)]
    pub whisper_accelerator: WhisperAcceleratorSetting,
    #[serde(default)]
    pub ort_accelerator: OrtAcceleratorSetting,
    #[serde(default = "default_whisper_gpu_device")]
    pub whisper_gpu_device: i32,
    #[serde(default)]
    pub normalize_audio_on_export: bool,
    #[serde(default)]
    pub export_volume_db: f32,
    #[serde(default)]
    pub export_fade_in_ms: u32,
    #[serde(default)]
    pub export_fade_out_ms: u32,
    #[serde(default = "default_caption_font_size")]
    pub caption_font_size: u32,
    #[serde(default = "default_caption_bg_color")]
    pub caption_bg_color: String,
    #[serde(default = "default_caption_text_color")]
    pub caption_text_color: String,
    #[serde(default = "default_caption_position")]
    pub caption_position: u32,
    #[serde(default = "default_settings_version")]
    pub settings_version: u32,
}

fn default_model() -> String {
    "".to_string()
}

fn default_settings_version() -> u32 {
    1
}

fn default_caption_font_size() -> u32 {
    24
}

fn default_caption_bg_color() -> String {
    "#000000B3".to_string()
}

fn default_caption_text_color() -> String {
    "#FFFFFF".to_string()
}

fn default_caption_position() -> u32 {
    90
}

fn default_preferred_output_sample_rate() -> u32 {
    48_000
}

fn default_translate_to_english() -> bool {
    false
}

fn default_start_hidden() -> bool {
    false
}

fn default_update_checks_enabled() -> bool {
    true
}

fn default_selected_language() -> String {
    "auto".to_string()
}

fn default_debug_mode() -> bool {
    false
}

fn default_log_level() -> LogLevel {
    LogLevel::Debug
}

fn default_word_correction_threshold() -> f64 {
    0.18
}

fn default_history_limit() -> usize {
    5
}

fn default_recording_retention_period() -> RecordingRetentionPeriod {
    RecordingRetentionPeriod::PreserveLimit
}

fn default_post_process_enabled() -> bool {
    false
}

fn default_post_process_provider_requires_api_key() -> bool {
    true
}

fn default_app_language() -> String {
    tauri_plugin_os::locale()
        .map(|l| l.replace('_', "-"))
        .unwrap_or_else(|| "en".to_string())
}

fn default_post_process_provider_id() -> String {
    OLLAMA_PROVIDER_ID.to_string()
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}

pub fn is_local_post_process_provider(provider: &PostProcessProvider) -> bool {
    provider.local_only || provider.id == APPLE_INTELLIGENCE_PROVIDER_ID
}

pub fn sanitize_local_post_process_base_url(base_url: &str) -> Result<String, String> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err("Base URL cannot be empty".to_string());
    }

    let parsed = reqwest::Url::parse(trimmed)
        .map_err(|e| format!("Invalid base URL '{}': {}", trimmed, e))?;

    match parsed.scheme() {
        "http" | "https" => {}
        _ => {
            return Err("Local provider URL must use http:// or https://".to_string());
        }
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "Base URL must include a host".to_string())?;
    if !is_loopback_host(host) {
        return Err("Local provider URL must point to localhost, 127.0.0.1, or ::1".to_string());
    }

    Ok(trimmed.trim_end_matches('/').to_string())
}

pub fn sanitize_post_process_model(model: &str) -> Result<String, String> {
    let trimmed = model.trim();

    if trimmed.len() > 256 {
        return Err("Model identifier is too long (max 256 characters)".to_string());
    }

    if trimmed.chars().any(|c| c.is_control()) {
        return Err("Model identifier contains invalid control characters".to_string());
    }

    Ok(trimmed.to_string())
}

fn default_post_process_providers() -> Vec<PostProcessProvider> {
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

fn default_post_process_api_keys() -> SecretMap {
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

fn default_post_process_models() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for provider in default_post_process_providers() {
        map.insert(
            provider.id.clone(),
            default_model_for_provider(&provider.id),
        );
    }
    map
}

fn default_post_process_prompts() -> Vec<LLMPrompt> {
    vec![LLMPrompt {
        id: "default_improve_transcriptions".to_string(),
        name: "Improve Transcriptions".to_string(),
        prompt: "Clean this transcript:\n1. Fix spelling, capitalization, and punctuation errors\n2. Convert number words to digits (twenty-five → 25, ten percent → 10%, five dollars → $5)\n3. Replace spoken punctuation with symbols (period → ., comma → ,, question mark → ?)\n4. Remove filler words (um, uh, like as filler)\n5. Keep the language in the original version (if it was french, keep it in french for example)\n6. Preserve numbers/currency/symbol tokens exactly when they already exist in the transcript\n\nPreserve exact meaning and word order. Do not paraphrase or reorder content.\n\nReturn only the cleaned transcript.\n\nTranscript:\n${output}".to_string(),
    }]
}

fn default_whisper_gpu_device() -> i32 {
    -1 // auto
}

fn ensure_post_process_defaults(settings: &mut AppSettings) -> bool {
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

pub const SETTINGS_STORE_PATH: &str = "settings_store.json";

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
        settings_version: default_settings_version(),
    }
}

impl AppSettings {
    pub fn active_post_process_provider(&self) -> Option<&PostProcessProvider> {
        self.post_process_providers
            .iter()
            .find(|provider| provider.id == self.post_process_provider_id)
    }

    pub fn post_process_provider(&self, provider_id: &str) -> Option<&PostProcessProvider> {
        self.post_process_providers
            .iter()
            .find(|provider| provider.id == provider_id)
    }

    pub fn post_process_provider_mut(
        &mut self,
        provider_id: &str,
    ) -> Option<&mut PostProcessProvider> {
        self.post_process_providers
            .iter_mut()
            .find(|provider| provider.id == provider_id)
    }
}

#[cfg(test)]
fn validate_settings(settings: &mut AppSettings) -> bool {
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

    settings.export_volume_db = settings.export_volume_db.clamp(-60.0, 24.0);
    settings.export_fade_in_ms = settings.export_fade_in_ms.min(30_000);
    settings.export_fade_out_ms = settings.export_fade_out_ms.min(30_000);

    changed
}

pub fn get_settings(app: &AppHandle) -> AppSettings {
    let store = app
        .store(crate::portable::store_path(SETTINGS_STORE_PATH))
        .expect("Failed to initialize store");

    let mut settings = if let Some(settings_value) = store.get("settings") {
        serde_json::from_value::<AppSettings>(settings_value).unwrap_or_else(|_| {
            let default_settings = get_default_settings();
            match serde_json::to_value(&default_settings) {
                Ok(val) => store.set("settings", val),
                Err(e) => warn!("Failed to serialize default settings: {}", e),
            }
            default_settings
        })
    } else {
        let default_settings = get_default_settings();
        match serde_json::to_value(&default_settings) {
            Ok(val) => store.set("settings", val),
            Err(e) => warn!("Failed to serialize default settings: {}", e),
        }
        default_settings
    };

    if ensure_post_process_defaults(&mut settings) {
        match serde_json::to_value(&settings) {
            Ok(val) => store.set("settings", val),
            Err(e) => warn!("Failed to serialize post-processed settings: {}", e),
        }
    }

    settings
}

pub fn write_settings(app: &AppHandle, settings: AppSettings) {
    let store = app
        .store(crate::portable::store_path(SETTINGS_STORE_PATH))
        .expect("Failed to initialize store");

    match serde_json::to_value(&settings) {
        Ok(val) => store.set("settings", val),
        Err(e) => warn!("Failed to serialize settings for write: {}", e),
    }
}

pub fn get_history_limit(app: &AppHandle) -> usize {
    let settings = get_settings(app);
    settings.history_limit
}

pub fn get_recording_retention_period(app: &AppHandle) -> RecordingRetentionPeriod {
    let settings = get_settings(app);
    settings.recording_retention_period
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(s.caption_position, default_caption_position());
    }

    #[test]
    fn test_validate_settings_fixes_invalid_color() {
        let mut s = get_default_settings();
        s.caption_text_color = "not-a-color".to_string();
        validate_settings(&mut s);
        assert_eq!(s.caption_text_color, default_caption_text_color());
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
}
