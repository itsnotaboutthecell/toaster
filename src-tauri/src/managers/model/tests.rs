//! Extracted from the inline mod tests block (monolith-split).

use super::*;

use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_discover_custom_whisper_models() {
    let temp_dir = TempDir::new().unwrap();
    let models_dir = temp_dir.path().to_path_buf();

    // Create test .bin files
    let mut custom_file = File::create(models_dir.join("my-custom-model.bin")).unwrap();
    custom_file.write_all(b"fake model data").unwrap();

    let mut another_file = File::create(models_dir.join("whisper_medical_v2.bin")).unwrap();
    another_file.write_all(b"another fake model").unwrap();

    // Create files that should be ignored
    File::create(models_dir.join(".hidden-model.bin")).unwrap(); // Hidden file
    File::create(models_dir.join("readme.txt")).unwrap(); // Non-.bin file
    File::create(models_dir.join("ggml-small.bin")).unwrap(); // Predefined filename
    fs::create_dir(models_dir.join("some-directory.bin")).unwrap(); // Directory

    // Set up available_models with a predefined Whisper model
    let mut models = HashMap::new();
    models.insert(
        "small".to_string(),
        ModelInfo {
            id: "small".to_string(),
            name: "Whisper Small".to_string(),
            description: "Test".to_string(),
            filename: "ggml-small.bin".to_string(),
            url: Some("https://example.com".to_string()),
            sha256: None,
            size_mb: 100,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: false,
            engine_type: EngineType::Whisper,
            accuracy_score: 0.5,
            speed_score: 0.5,
            supports_translation: true,
            is_recommended: false,
            supported_languages: vec!["en".to_string()],
            supports_language_selection: true,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    // Discover custom models
    super::catalog::discover_custom_whisper_models(&models_dir, &mut models).unwrap();

    // Should have discovered 2 custom models (my-custom-model and whisper_medical_v2)
    assert!(models.contains_key("my-custom-model"));
    assert!(models.contains_key("whisper_medical_v2"));

    // Verify custom model properties
    let custom = models.get("my-custom-model").unwrap();
    assert_eq!(custom.name, "My Custom Model");
    assert_eq!(custom.filename, "my-custom-model.bin");
    assert!(custom.url.is_none()); // Custom models have no URL
    assert!(custom.is_downloaded);
    assert!(custom.is_custom);
    assert_eq!(custom.accuracy_score, 0.0);
    assert_eq!(custom.speed_score, 0.0);
    assert!(custom.supported_languages.is_empty());

    // Verify underscore handling
    let medical = models.get("whisper_medical_v2").unwrap();
    assert_eq!(medical.name, "Whisper Medical V2");

    // Should NOT have discovered hidden, non-.bin, predefined, or directories
    assert!(!models.contains_key(".hidden-model"));
    assert!(!models.contains_key("readme"));
    assert!(!models.contains_key("some-directory"));
}

#[test]
fn test_discover_custom_models_empty_dir() {
    let temp_dir = TempDir::new().unwrap();
    let models_dir = temp_dir.path().to_path_buf();

    let mut models = HashMap::new();
    let count_before = models.len();

    super::catalog::discover_custom_whisper_models(&models_dir, &mut models).unwrap();

    // No new models should be added
    assert_eq!(models.len(), count_before);
}

#[test]
fn test_discover_custom_models_nonexistent_dir() {
    let models_dir = PathBuf::from("/nonexistent/path/that/does/not/exist");

    let mut models = HashMap::new();
    let count_before = models.len();

    // Should not error, just return Ok
    let result = super::catalog::discover_custom_whisper_models(&models_dir, &mut models);
    assert!(result.is_ok());
    assert_eq!(models.len(), count_before);
}

// ── SHA256 verification tests ─────────────────────────────────────────────

/// Helper: write `data` to a temp file and return (TempDir, path).
/// TempDir must be kept alive for the duration of the test.
fn write_temp_file(data: &[u8]) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("model.partial");
    let mut f = File::create(&path).unwrap();
    f.write_all(data).unwrap();
    (dir, path)
}

#[test]
fn test_verify_sha256_skipped_when_none() {
    // Custom models have no expected hash — verification must be a no-op.
    let (_dir, path) = write_temp_file(b"anything");
    assert!(super::hash::verify_sha256(&path, None, "custom").is_ok());
    assert!(
        path.exists(),
        "file must be untouched when verification is skipped"
    );
}

#[test]
fn test_verify_sha256_passes_on_correct_hash() {
    // Compute the real hash so the test is self-consistent.
    let (_dir, path) = write_temp_file(b"hello world");
    let actual = super::hash::compute_sha256(&path).unwrap();
    assert!(
        super::hash::verify_sha256(&path, Some(&actual), "test_model").is_ok(),
        "should pass when hash matches"
    );
    assert!(
        path.exists(),
        "file must be kept on successful verification"
    );
}

#[test]
fn test_verify_sha256_fails_and_deletes_partial_on_mismatch() {
    let (_dir, path) = write_temp_file(b"this is not the real model");
    let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

    let result = super::hash::verify_sha256(&path, Some(wrong_hash), "bad_model");

    assert!(result.is_err(), "mismatch must return an error");
    assert!(
        result.unwrap_err().to_string().contains("corrupt"),
        "error message should mention corruption"
    );
    assert!(
        !path.exists(),
        "partial file must be deleted after hash mismatch"
    );
}

#[test]
fn test_verify_sha256_fails_and_deletes_partial_when_file_missing() {
    // Simulate a partial file that was already removed (e.g. disk full mid-download).
    let dir = TempDir::new().unwrap();
    let missing_path = dir.path().join("gone.partial");
    // Don't create the file — it should not exist.

    let result =
        super::hash::verify_sha256(&missing_path, Some("anyexpectedhash"), "missing_model");

    assert!(result.is_err(), "missing file must return an error");
}


#[test]
fn model_category_variants() {
    let variants = [
        ModelCategory::Transcription,
        ModelCategory::VoiceActivityDetection,
    ];
    for v in variants {
        let classified = match v {
            ModelCategory::Transcription => "transcription",
            ModelCategory::VoiceActivityDetection => "voice-activity-detection",
        };
        assert!(!classified.is_empty());
    }
    assert_eq!(ModelCategory::default(), ModelCategory::Transcription);
}


#[test]
fn per_category_metadata_populated() {
    let info = ModelInfo {
        id: "m".into(),
        name: "m".into(),
        description: String::new(),
        filename: "m.bin".into(),
        url: None,
        sha256: None,
        size_mb: 0,
        is_downloaded: false,
        is_downloading: false,
        partial_size: 0,
        is_directory: false,
        engine_type: EngineType::Whisper,
        accuracy_score: 0.0,
        speed_score: 0.0,
        supports_translation: false,
        is_recommended: false,
        supported_languages: vec![],
        supports_language_selection: false,
        is_custom: false,
        category: ModelCategory::Transcription,
        transcription_metadata: Some(TranscriptionMetadata {
            engine_type: EngineType::Whisper,
            accuracy_score: 0.9,
            speed_score: 0.5,
            supports_translation: true,
            supports_language_selection: true,
            supported_languages: vec!["en".to_string()],
        }),
    };
    assert!(info.transcription_metadata.is_some());
}

#[test]
fn legacy_model_info_json_roundtrips() {
    let legacy = r#"{
        "id":"tiny",
        "name":"Tiny",
        "description":"",
        "filename":"tiny.bin",
        "url":null,
        "sha256":null,
        "size_mb":75,
        "is_downloaded":false,
        "is_downloading":false,
        "partial_size":0,
        "is_directory":false,
        "engine_type":"Whisper",
        "accuracy_score":0.5,
        "speed_score":0.9,
        "supports_translation":true,
        "is_recommended":false,
        "supported_languages":["en"],
        "supports_language_selection":true,
        "is_custom":false
    }"#;
    let parsed: ModelInfo = serde_json::from_str(legacy).expect("legacy JSON must deserialize");
    assert!(parsed.transcription_metadata.is_none());
    assert_eq!(parsed.category, ModelCategory::Transcription);
    let out = serde_json::to_string(&parsed).unwrap();
    let roundtrip: ModelInfo = serde_json::from_str(&out).unwrap();
    assert_eq!(roundtrip.id, "tiny");
}
