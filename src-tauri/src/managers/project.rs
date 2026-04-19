/// Project save/load system for Toaster (.toaster files).
///
/// A `.toaster` project is a pretty-printed JSON file that stores project
/// metadata, the source media path, the full word list (transcript with
/// edit states), filler-detection config, and export settings.
use crate::managers::editor::Word;
use crate::settings::CaptionProfileSet;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Current on-disk project format version. Bumped to 1.1.0 for
/// `caption-profiles-persistence` (Slice B), which adds the optional
/// `settings.caption_profiles` field. v1.0.0 files still load cleanly
/// via `#[serde(default)]` on the new field; the next save rewrites
/// them as 1.1.0 with the crystallized profiles.
pub const PROJECT_VERSION: &str = "1.1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToasterProject {
    pub version: String,
    pub name: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last-modified timestamp (updated on every save).
    pub modified_at: String,
    /// Path to the source media file (relative to the project file).
    pub source_media: Option<PathBuf>,
    pub words: Vec<Word>,
    pub settings: ProjectSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Words to flag as filler (e.g. "um", "uh", "like").
    pub filler_words: Vec<String>,
    /// Minimum pause duration (µs) to flag as a gap.
    pub pause_threshold_us: i64,
    /// Export format: "srt", "vtt", or "script".
    pub export_format: String,
    /// Per-orientation caption profiles. `None` means "inherit from
    /// app-level settings" (v1.0.0 projects load this way). On save,
    /// the current profiles are crystallized into `Some(_)` so the
    /// project file is self-describing going forward.
    #[serde(default)]
    pub caption_profiles: Option<CaptionProfileSet>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            filler_words: vec![
                "um".into(),
                "uh".into(),
                "like".into(),
                "you know".into(),
                "so".into(),
                "actually".into(),
            ],
            pause_threshold_us: 1_000_000, // 1 second
            export_format: "srt".into(),
            caption_profiles: None,
        }
    }
}

impl ToasterProject {
    /// Create a new empty project with sensible defaults.
    pub fn new(name: &str) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            version: PROJECT_VERSION.into(),
            name: name.into(),
            created_at: now.clone(),
            modified_at: now,
            source_media: None,
            words: Vec::new(),
            settings: ProjectSettings::default(),
        }
    }

    /// Save the project to a `.toaster` file (pretty-printed JSON).
    ///
    /// Updates `modified_at` before writing.
    pub fn save(&mut self, path: &Path) -> Result<(), String> {
        self.modified_at = Utc::now().to_rfc3339();
        self.version = PROJECT_VERSION.into();

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize project: {e}"))?;

        std::fs::write(path, json).map_err(|e| format!("Failed to write project file: {e}"))
    }

    /// Load a project from a `.toaster` file.
    ///
    /// Validates that the version field is present and matches the
    /// expected major version.
    pub fn load(path: &Path) -> Result<Self, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read project file: {e}"))?;

        let project: Self =
            serde_json::from_str(&data).map_err(|e| format!("Failed to parse project: {e}"))?;

        // Validate major version compatibility
        let major = project
            .version
            .split('.')
            .next()
            .unwrap_or("0")
            .parse::<u32>()
            .map_err(|_| format!("Invalid version string: {}", project.version))?;

        let expected_major = PROJECT_VERSION
            .split('.')
            .next()
            .unwrap_or("0")
            .parse::<u32>()
            .unwrap_or(0);

        if major != expected_major {
            return Err(format!(
                "Unsupported project version {} (expected {}.x.x)",
                project.version, expected_major
            ));
        }

        Ok(project)
    }

    /// Replace the word list (e.g. after syncing from editor state).
    pub fn set_words(&mut self, words: Vec<Word>) {
        self.words = words;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_words() -> Vec<Word> {
        vec![
            Word {
                text: "Hello".into(),
                start_us: 0,
                end_us: 1_000_000,
                deleted: false,
                silenced: false,
                confidence: 0.95,
                speaker_id: 0,
            },
            Word {
                text: "world".into(),
                start_us: 1_000_000,
                end_us: 2_000_000,
                deleted: true,
                silenced: false,
                confidence: 0.88,
                speaker_id: 1,
            },
        ]
    }

    fn temp_project_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("toaster_test_{name}.toaster"))
    }

    #[test]
    fn new_project_has_defaults() {
        let project = ToasterProject::new("My Project");
        assert_eq!(project.name, "My Project");
        assert_eq!(project.version, PROJECT_VERSION);
        assert!(project.words.is_empty());
        assert!(project.source_media.is_none());
        assert!(!project.created_at.is_empty());
        assert!(!project.modified_at.is_empty());
        assert_eq!(project.settings.export_format, "srt");
        assert!(!project.settings.filler_words.is_empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_project_path("round_trip");
        let mut project = ToasterProject::new("Round Trip");
        project.source_media = Some(PathBuf::from("media/video.mp4"));
        project.set_words(make_words());
        project.settings.export_format = "vtt".into();

        project.save(&path).expect("save should succeed");
        let loaded = ToasterProject::load(&path).expect("load should succeed");

        assert_eq!(loaded.name, "Round Trip");
        assert_eq!(loaded.version, PROJECT_VERSION);
        assert_eq!(loaded.source_media, Some(PathBuf::from("media/video.mp4")));
        assert_eq!(loaded.settings.export_format, "vtt");
        assert_eq!(loaded.words.len(), 2);

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_preserves_word_fields() {
        let path = temp_project_path("word_fields");
        let mut project = ToasterProject::new("Word Fields");
        project.set_words(make_words());
        project.save(&path).expect("save should succeed");

        let loaded = ToasterProject::load(&path).expect("load should succeed");
        let w = &loaded.words[1];
        assert_eq!(w.text, "world");
        assert_eq!(w.start_us, 1_000_000);
        assert_eq!(w.end_us, 2_000_000);
        assert!(w.deleted);
        assert!(!w.silenced);
        assert!((w.confidence - 0.88).abs() < f32::EPSILON);
        assert_eq!(w.speaker_id, 1);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn save_updates_modified_at() {
        let path = temp_project_path("modified_at");
        let mut project = ToasterProject::new("Timestamps");
        let original_modified = project.modified_at.clone();

        // Small delay so timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));
        project.save(&path).expect("save should succeed");

        assert_ne!(project.modified_at, original_modified);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_invalid_path_returns_error() {
        let result = ToasterProject::load(Path::new("nonexistent_file.toaster"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read"));
    }

    #[test]
    fn load_invalid_json_returns_error() {
        let path = temp_project_path("bad_json");
        fs::write(&path, "{ this is not valid json }").expect("write should succeed");

        let result = ToasterProject::load(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_wrong_version_returns_error() {
        let path = temp_project_path("bad_version");
        let json = serde_json::json!({
            "version": "99.0.0",
            "name": "Bad Version",
            "created_at": "2025-01-01T00:00:00Z",
            "modified_at": "2025-01-01T00:00:00Z",
            "source_media": null,
            "words": [],
            "settings": {
                "filler_words": [],
                "pause_threshold_us": 1000000,
                "export_format": "srt"
            }
        });
        fs::write(&path, json.to_string()).expect("write should succeed");

        let result = ToasterProject::load(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported project version"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn version_field_is_correct() {
        let project = ToasterProject::new("Version Check");
        assert_eq!(project.version, "1.1.0");
    }

    #[test]
    fn project_v1_0_loads_with_none_profiles() {
        // AC-003-a: v1.0.0 fixture deserializes cleanly; the new
        // `caption_profiles` field is absent → defaults to `None`.
        let path = temp_project_path("v1_0_loads");
        let json = serde_json::json!({
            "version": "1.0.0",
            "name": "v1 Project",
            "created_at": "2025-01-01T00:00:00Z",
            "modified_at": "2025-01-01T00:00:00Z",
            "source_media": null,
            "words": [],
            "settings": {
                "filler_words": ["um"],
                "pause_threshold_us": 1000000,
                "export_format": "srt"
            }
        });
        fs::write(&path, json.to_string()).expect("write should succeed");

        let loaded = ToasterProject::load(&path).expect("v1.0.0 must load");
        assert_eq!(loaded.version, "1.0.0");
        assert!(loaded.settings.caption_profiles.is_none());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn project_save_bumps_version_to_1_1_0_and_writes_profiles() {
        // AC-003-b: after save, the on-disk file has version 1.1.0
        // and carries a populated `caption_profiles` block.
        let path = temp_project_path("v1_saves_1_1_0");
        let mut project = ToasterProject::new("Upgraded");
        project.settings.caption_profiles = Some(CaptionProfileSet {
            desktop: crate::settings::default_desktop_profile(),
            mobile: crate::settings::default_mobile_profile(),
        });
        project.save(&path).expect("save should succeed");

        let raw = fs::read_to_string(&path).expect("read-back");
        let parsed: serde_json::Value =
            serde_json::from_str(&raw).expect("valid json");
        assert_eq!(parsed["version"], "1.1.0");
        assert!(parsed["settings"]["caption_profiles"].is_object());
        assert!(parsed["settings"]["caption_profiles"]["desktop"].is_object());
        assert!(parsed["settings"]["caption_profiles"]["mobile"].is_object());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn project_import_preserves_caption_profiles_when_some() {
        // AC-003-c: a project that explicitly sets caption_profiles
        // round-trips without loss.
        let path = temp_project_path("v1_preserves_profiles");
        let mut project = ToasterProject::new("With Profiles");
        let mut desktop = crate::settings::default_desktop_profile();
        desktop.font_size = 55;
        let mobile = crate::settings::default_mobile_profile();
        project.settings.caption_profiles = Some(CaptionProfileSet {
            desktop: desktop.clone(),
            mobile: mobile.clone(),
        });
        project.save(&path).expect("save should succeed");

        let loaded = ToasterProject::load(&path).expect("load should succeed");
        let set = loaded
            .settings
            .caption_profiles
            .expect("profiles round-trip");
        assert_eq!(set.desktop.font_size, 55);
        assert_eq!(set.mobile.font_size, mobile.font_size);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn project_v1_0_0_fixture_migrates_on_save() {
        // AC-008-a: open the committed v1.0.0 fixture, save, verify
        // the result is v1.1.0 with profiles populated.
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("features")
            .join("caption-profiles-persistence")
            .join("fixtures")
            .join("project_v1_0_0.toaster");
        let mut project =
            ToasterProject::load(&fixture).expect("v1.0.0 fixture must load");
        assert_eq!(project.version, "1.0.0");
        assert!(project.settings.caption_profiles.is_none());

        // Simulate the save-path crystallization.
        project.settings.caption_profiles = Some(CaptionProfileSet {
            desktop: crate::settings::default_desktop_profile(),
            mobile: crate::settings::default_mobile_profile(),
        });
        let out = temp_project_path("v1_migrate_on_save");
        project.save(&out).expect("save should succeed");

        let saved = ToasterProject::load(&out).expect("resaved load");
        assert_eq!(saved.version, "1.1.0");
        assert!(saved.settings.caption_profiles.is_some());

        let _ = fs::remove_file(&out);
    }

    #[test]
    fn set_words_replaces_list() {
        let mut project = ToasterProject::new("Set Words");
        assert!(project.words.is_empty());
        project.set_words(make_words());
        assert_eq!(project.words.len(), 2);
        project.set_words(Vec::new());
        assert!(project.words.is_empty());
    }
}
