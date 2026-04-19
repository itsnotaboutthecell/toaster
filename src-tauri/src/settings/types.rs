//! Settings value types: enums, structs, and their trait impls.
//!
//! All `default_*` fns referenced by `#[serde(default = "...")]` live in
//! `super::defaults` and are brought into scope via the `use` below so the
//! serde macro expansion can resolve them.

use super::defaults::*;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;
use std::collections::HashMap;

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
    #[serde(default = "default_app_language")]
    pub app_language: String,
    /// Master gate for the Experimental settings group. When `false`,
    /// per-flag booleans still store whatever the user last set, but the
    /// `is_experiment_enabled` getter (and the matching
    /// `useExperiment` hook on the frontend) return `false` so no
    /// experimental feature actually activates. Defence-in-depth:
    /// stored values are preserved across master toggle flips so a
    /// user's prior opt-in comes back when they re-enable the master.
    #[serde(default)]
    pub experimental_enabled: bool,
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
    /// **R-006 (Silero VAD reintroduction)** — when true, the
    /// transcription manager runs Silero over the decoded file audio
    /// before any `transcribe-rs` pass and hands only the speech
    /// windows to the ASR. Falls back silently to the full-file path
    /// when the Silero ONNX is not on disk or ORT init fails
    /// (BLUEPRINT AD-8). Default `true` because the happy path is a
    /// wall-time + hallucination-rate win per R-002.
    #[serde(default = "default_vad_prefilter_enabled")]
    pub vad_prefilter_enabled: bool,
    /// **R-006 (Silero VAD reintroduction)** — when true, the splice
    /// boundary snap in `managers::splice::boundaries` consults a
    /// P(speech) curve within the existing zero-crossing / energy
    /// radii. Default `false` pending the eval-win gate in R-003;
    /// with it off, the code path is byte-identical to pre-feature
    /// (AC-003-d).
    #[serde(default = "default_vad_refine_boundaries")]
    pub vad_refine_boundaries: bool,
    /// Loudness normalization target for export. Single source of truth
    /// for the `loudnorm` filter — see
    /// `managers::splice::loudness::build_loudnorm_filter`. Frontend
    /// only stores this enum and renders a Select; it MUST NOT
    /// hand-build a `loudnorm=...` string. Legacy
    /// `normalize_audio_on_export` migrates to `PodcastMinus16` via
    /// `settings::migrate_loudness_setting`.
    #[serde(default)]
    pub loudness_target: crate::managers::splice::loudness::LoudnessTarget,
    #[serde(default)]
    pub export_volume_db: f32,
    #[serde(default)]
    pub export_fade_in_ms: u32,
    #[serde(default)]
    pub export_fade_out_ms: u32,
    /// Consumed by `export_edited_media` when no per-invocation override
    /// is supplied (Round-8: the user-facing format picker moved from
    /// Settings → Advanced → Export into the Editor's per-project
    /// ExportMenu; the two persisted settings fields were removed and
    /// hard-coded defaults now fall back to Mp4 for video sources and
    /// Wav for audio-only sources inside `export_edited_media`).
    #[serde(default = "default_caption_font_size")]
    pub caption_font_size: u32,
    #[serde(default = "default_caption_bg_color")]
    pub caption_bg_color: String,
    #[serde(default = "default_caption_text_color")]
    pub caption_text_color: String,
    #[serde(default = "default_caption_position")]
    pub caption_position: u32,
    #[serde(default)]
    pub caption_font_family: CaptionFontFamily,
    #[serde(default = "default_caption_radius_px")]
    pub caption_radius_px: u32,
    #[serde(default = "default_caption_padding_x_px")]
    pub caption_padding_x_px: u32,
    #[serde(default = "default_caption_padding_y_px")]
    pub caption_padding_y_px: u32,
    #[serde(default = "default_caption_max_width_percent")]
    pub caption_max_width_percent: u32,
    /// Per-orientation caption profiles. Slice B single-source-of-truth
    /// for caption geometry — preview and export both read through
    /// `managers::captions::compute_caption_layout(&profile, dims)`.
    /// The flat `caption_*` fields above are retained one release for
    /// backward-compat; on first load `defaults::ensure_caption_defaults`
    /// seeds this from those flat fields.
    #[serde(default = "default_caption_profiles")]
    pub caption_profiles: CaptionProfileSet,
    /// Idempotency latch for the flat-field → profiles migration. Once
    /// true, `ensure_caption_defaults` is a no-op on subsequent loads.
    #[serde(default)]
    pub caption_profiles_was_migrated: bool,
    #[serde(default = "default_settings_version")]
    pub settings_version: u32,
}

/// Per-orientation caption profile. Carries the 9 user-configurable
/// caption geometry fields that persist in settings and projects.
/// Preview and libass export both consume geometry derived from this
/// via `managers::captions::compute_caption_layout` — the single
/// source of truth for caption layout math (AGENTS.md, Slice B).
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type,
)]
pub struct CaptionProfile {
    pub font_size: u32,
    pub bg_color: String,
    pub text_color: String,
    pub position: u32,
    pub font_family: CaptionFontFamily,
    pub radius_px: u32,
    pub padding_x_px: u32,
    pub padding_y_px: u32,
    pub max_width_percent: u32,
}

/// Pair of caption profiles selected by orientation. Desktop is used
/// for landscape (width/height > 1.0), Mobile for portrait or square.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type,
)]
pub struct CaptionProfileSet {
    pub desktop: CaptionProfile,
    pub mobile: CaptionProfile,
}

/// Orientation selector for caption commands. Auto-detection happens
/// in the frontend editor radio; the Tauri surface only exposes the
/// two concrete profiles.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type,
)]
pub enum Orientation {
    Desktop,
    Mobile,
}

/// Scope for `set_caption_profile` — whether the write lands on
/// `AppSettings` (user-default) or the currently-open `ProjectSettings`
/// (per-project override).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type,
)]
pub enum ProfileScope {
    App,
    Project,
}

/// Video dimensions in pixels. Input to `compute_caption_layout`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type,
)]
pub struct VideoDims {
    pub width: u32,
    pub height: u32,
}

/// Font family choice for captions. The preview CSS and the exported ASS
/// both read from this enum so they stay in visual sync.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, specta::Type,
)]
pub enum CaptionFontFamily {
    #[default]
    Inter,
    Roboto,
    SystemUi,
}

impl AppSettings {}
