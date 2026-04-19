// CI-only mock TranscriptionManager - avoids whisper/Vulkan dependencies.
// This file is copied over transcription.rs during CI tests.
//
// The mock must preserve the exact public signature of the real
// `TranscriptionManager` so that callers (e.g.
// `commands::transcribe_file`) which consume the returned
// `NormalizedTranscriptionResult` continue to compile — and so that the
// entire segment-timestamp pipeline (`build_words_from_segments`,
// `refine_word_boundaries`, `realign_suspicious_spans`) stays visible to CI.

use crate::managers::model::ModelManager;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use transcribe_rs::TranscriptionSegment;

// Mock adapter module — mirrors the production `adapter` module's public
// surface just enough for CI tests to exercise capability flags, language
// normalization, and canonical-result construction without pulling in the
// whisper/Vulkan-dependent production code.
pub mod adapter {
    use super::*;
    use transcribe_rs::TranscriptionResult;

    pub const ASR_INPUT_SAMPLE_RATE_HZ_DEFAULT: u32 = 16_000;

    #[derive(Debug, Clone, PartialEq)]
    pub struct CanonicalWord {
        pub text: String,
        pub start_us: i64,
        pub end_us: i64,
        pub confidence: f32,
        pub speaker_id: i32,
        pub is_non_speech: bool,
    }

    #[derive(Debug, Clone)]
    pub struct NormalizedTranscriptionResult {
        pub words: Vec<CanonicalWord>,
        pub text: String,
        pub segments: Option<Vec<TranscriptionSegment>>,
        pub language: String,
        pub word_timestamps_authoritative: bool,
    }

    impl NormalizedTranscriptionResult {
        pub fn validate(&self) -> Result<()> {
            for (i, w) in self.words.iter().enumerate() {
                if w.is_non_speech {
                    return Err(anyhow::anyhow!("non-speech at {}", i));
                }
                if w.start_us >= w.end_us {
                    return Err(anyhow::anyhow!("zero/neg duration at {}", i));
                }
                if i > 0 && w.start_us < self.words[i - 1].end_us {
                    return Err(anyhow::anyhow!("overlap at {}", i));
                }
            }
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ModelCapabilities {
        pub supports_prompt_injection: bool,
        pub supports_fuzzy_word_correction: bool,
        pub has_pre_speech_padding: bool,
        pub native_input_sample_rate_hz: u32,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct AudioInfo {
        pub duration_us: i64,
    }

    pub trait TranscriptionModelAdapter: Send + Sync {
        fn capabilities(&self) -> ModelCapabilities;
        fn normalize_language(&self, raw: &str) -> Option<String>;
        fn adapt(
            &self,
            raw: TranscriptionResult,
            audio_info: AudioInfo,
        ) -> Result<NormalizedTranscriptionResult>;
    }

    pub struct MockAdapter;

    impl TranscriptionModelAdapter for MockAdapter {
        fn capabilities(&self) -> ModelCapabilities {
            ModelCapabilities {
                supports_prompt_injection: false,
                supports_fuzzy_word_correction: true,
                has_pre_speech_padding: false,
                native_input_sample_rate_hz: ASR_INPUT_SAMPLE_RATE_HZ_DEFAULT,
            }
        }

        fn normalize_language(&self, raw: &str) -> Option<String> {
            if raw == "auto" || raw.is_empty() {
                None
            } else {
                Some(raw.to_string())
            }
        }

        fn adapt(
            &self,
            raw: TranscriptionResult,
            audio_info: AudioInfo,
        ) -> Result<NormalizedTranscriptionResult> {
            let text_for_result = raw.text.clone();
            let segments_for_result = raw.segments.clone();
            let segs = raw.segments.unwrap_or_default();
            let mut words: Vec<CanonicalWord> = Vec::with_capacity(segs.len());
            let mut cursor: i64 = 0;
            for s in &segs {
                let text = s.text.trim();
                if text.is_empty() {
                    continue;
                }
                let start_us = (s.start as f64 * 1_000_000.0).round() as i64;
                let end_us = (s.end as f64 * 1_000_000.0).round() as i64;
                // One CanonicalWord per whitespace token; split evenly if
                // there's more than one, otherwise preserve native times.
                let tokens: Vec<&str> = text.split_whitespace().collect();
                if tokens.len() == 1 {
                    let ws = start_us.max(cursor);
                    let mut we = end_us;
                    if we <= ws {
                        we = ws + 1_000;
                    }
                    words.push(CanonicalWord {
                        text: tokens[0].to_string(),
                        start_us: ws,
                        end_us: we,
                        confidence: -1.0,
                        speaker_id: -1,
                        is_non_speech: false,
                    });
                    cursor = we;
                } else {
                    let dur = (end_us - start_us).max(tokens.len() as i64 * 1_000);
                    let per = dur / tokens.len() as i64;
                    for (i, t) in tokens.iter().enumerate() {
                        let ws = (start_us + per * i as i64).max(cursor);
                        let we = if i == tokens.len() - 1 {
                            end_us.max(ws + 1_000)
                        } else {
                            (start_us + per * (i as i64 + 1)).max(ws + 1_000)
                        };
                        words.push(CanonicalWord {
                            text: t.to_string(),
                            start_us: ws,
                            end_us: we,
                            confidence: -1.0,
                            speaker_id: -1,
                            is_non_speech: false,
                        });
                        cursor = we;
                    }
                }
            }
            let word_level = segs.iter().all(|s| s.text.trim().split_whitespace().count() == 1);
            let _ = audio_info;
            Ok(NormalizedTranscriptionResult {
                words,
                text: text_for_result,
                segments: segments_for_result,
                language: "und".to_string(),
                word_timestamps_authoritative: word_level,
            })
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ModelStateEvent {
    pub event_type: String,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
    pub error: Option<String>,
}

/// RAII guard that is a no-op in the mock — mirrors the real `LoadingGuard`.
pub struct LoadingGuard;

/// Serde-friendly mirror of `transcribe_rs::TranscriptionSegment`, used to
/// deserialize fixture JSON without requiring upstream to derive `Deserialize`.
#[derive(Deserialize)]
struct FixtureSegment {
    start: f32,
    end: f32,
    text: String,
}

#[derive(Deserialize)]
struct FixtureFile {
    text: String,
    segments: Vec<FixtureSegment>,
}

/// Configures what the mock returns from `transcribe`.
#[derive(Clone, Debug)]
pub enum MockTranscription {
    /// Inline text. Segments are **mock-synthesized, equal-duration** — the
    /// text is split on sentence punctuation and time is distributed evenly
    /// across the audio buffer duration. *Not representative of any real ASR
    /// engine*: do NOT use this variant in tests that assert on timing
    /// precision (use `Fixture` instead).
    FixedText(String),
    /// Pre-paired text + segments sourced from a fixture. Use this for tests
    /// that exercise timing behavior (word boundary refinement, span
    /// realignment, keep-segment mapping, etc.).
    Fixture {
        text: String,
        segments: Vec<TranscriptionSegment>,
    },
    /// Empty transcription (historical default — preserves prior behavior for
    /// call sites that were content with `Ok(String::new())`).
    Empty,
}

impl MockTranscription {
    /// Inline fixed text; segments will be equal-duration synthesized at
    /// `transcribe()` time. See variant docs for the precision caveat.
    pub fn fixed_text(text: impl Into<String>) -> Self {
        Self::FixedText(text.into())
    }

    /// Load paired text + segments from a JSON fixture:
    /// `{ "text": "...", "segments": [{ "start": f32, "end": f32, "text": "..." }, ...] }`.
    pub fn from_fixture(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading mock fixture {}", path.display()))?;
        let file: FixtureFile = serde_json::from_str(&raw)
            .with_context(|| format!("parsing mock fixture {}", path.display()))?;
        let segments = file
            .segments
            .into_iter()
            .map(|s| TranscriptionSegment {
                start: s.start,
                end: s.end,
                text: s.text,
            })
            .collect();
        Ok(Self::Fixture {
            text: file.text,
            segments,
        })
    }
}

impl Default for MockTranscription {
    fn default() -> Self {
        Self::Empty
    }
}

#[derive(Clone)]
pub struct TranscriptionManager {
    #[allow(dead_code)]
    app_handle: AppHandle,
    mock: Arc<Mutex<MockTranscription>>,
}

impl TranscriptionManager {
    pub fn new(app_handle: &AppHandle, _model_manager: Arc<ModelManager>) -> Result<Self> {
        Ok(Self {
            app_handle: app_handle.clone(),
            mock: Arc::new(Mutex::new(MockTranscription::default())),
        })
    }

    /// Test hook: swap the configured mock response. No-op outside tests; the
    /// production build uses the real `TranscriptionManager`.
    #[allow(dead_code)]
    pub fn set_mock(&self, mock: MockTranscription) {
        *crate::lock_recovery::recover_lock(self.mock.lock()) = mock;
    }

    pub fn is_model_loaded(&self) -> bool {
        false
    }

    pub fn try_start_loading(&self) -> Option<LoadingGuard> {
        Some(LoadingGuard)
    }

    pub fn unload_model(&self) -> Result<()> {
        Ok(())
    }

    pub fn maybe_unload_immediately(&self, _context: &str) {}

    pub fn load_model(&self, _model_id: &str) -> Result<()> {
        Ok(())
    }

    pub fn initiate_model_load(&self) {}

    pub fn get_current_model(&self) -> Option<String> {
        None
    }

    pub fn transcribe(
        &self,
        audio: Vec<f32>,
    ) -> Result<adapter::NormalizedTranscriptionResult> {
        let mock = crate::lock_recovery::recover_lock(self.mock.lock()).clone();
        let (text, segments) = match mock {
            MockTranscription::Empty => (String::new(), None),
            MockTranscription::Fixture { text, segments } => (text, Some(segments)),
            MockTranscription::FixedText(text) => {
                let segments = synthesize_equal_duration_segments(&text, audio.len());
                (text, Some(segments))
            }
        };
        // The mock preserves raw segments on the normalized result so the
        // downstream `build_words_from_segments` pipeline has the same shape
        // as production. Word-level canonicalization isn't exercised in CI.
        Ok(adapter::NormalizedTranscriptionResult {
            words: Vec::new(),
            text,
            segments,
            language: "und".to_string(),
            word_timestamps_authoritative: false,
        })
    }
}

/// Split `text` on sentence punctuation and distribute `audio.len()` samples
/// (@16 kHz) evenly across the resulting segments.
///
/// **Mock-synthesized, equal-duration — not representative of any real ASR
/// engine.** Any test that asserts on precise per-word or per-segment timing
/// MUST use `MockTranscription::from_fixture` instead.
fn synthesize_equal_duration_segments(text: &str, audio_samples: usize) -> Vec<TranscriptionSegment> {
    const SAMPLE_RATE: f32 = 16_000.0;
    let total_duration = (audio_samples as f32 / SAMPLE_RATE).max(0.0);

    let pieces = split_on_sentence_punctuation(text);
    if pieces.is_empty() {
        return Vec::new();
    }
    let per = total_duration / pieces.len() as f32;
    pieces
        .into_iter()
        .enumerate()
        .map(|(i, piece)| TranscriptionSegment {
            start: per * i as f32,
            end: per * (i as f32 + 1.0),
            text: piece,
        })
        .collect()
}

fn split_on_sentence_punctuation(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
            current.clear();
        }
    }
    let tail = current.trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    out
}

/// No-op in CI mock.
pub fn apply_accelerator_settings(_app: &tauri::AppHandle) {}

#[derive(Serialize, Clone, Debug, Type)]
pub struct GpuDeviceOption {
    pub id: i32,
    pub name: String,
    pub total_vram_mb: usize,
}

#[derive(Serialize, Clone, Debug, Type)]
pub struct AvailableAccelerators {
    pub whisper: Vec<String>,
    pub ort: Vec<String>,
    pub gpu_devices: Vec<GpuDeviceOption>,
}

/// Returns empty lists in CI mock.
pub fn get_available_accelerators() -> AvailableAccelerators {
    AvailableAccelerators {
        whisper: vec![],
        ort: vec![],
        gpu_devices: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("mock_transcription_sample.json")
    }

    fn assert_monotonic(segments: &[TranscriptionSegment]) {
        for (i, seg) in segments.iter().enumerate() {
            assert!(
                seg.start <= seg.end,
                "segment {}: start {} > end {}",
                i,
                seg.start,
                seg.end
            );
            if i > 0 {
                let prev = &segments[i - 1];
                assert!(
                    prev.end <= seg.start + 1e-6,
                    "segment {} overlaps previous: prev.end={} seg.start={}",
                    i,
                    prev.end,
                    seg.start
                );
            }
        }
    }

    #[test]
    fn fixture_returns_non_empty_monotonic_segments() {
        let mock = MockTranscription::from_fixture(fixture_path())
            .expect("fixture must load");
        let (text, segments) = match mock {
            MockTranscription::Fixture { text, segments } => (text, segments),
            _ => panic!("from_fixture must produce Fixture variant"),
        };
        assert!(!text.is_empty(), "fixture text must be non-empty");
        assert!(!segments.is_empty(), "fixture must provide segments");
        assert_monotonic(&segments);
    }

    #[test]
    fn fixed_text_synthesizes_equal_duration_segments() {
        // 16 kHz * 4 seconds = 64_000 samples; three sentences → 3 segments of ~4/3s each.
        let samples = vec![0.0_f32; 64_000];
        let text = "One. Two. Three.";
        let segments = synthesize_equal_duration_segments(text, samples.len());
        assert_eq!(segments.len(), 3);
        assert_monotonic(&segments);
        let per = 4.0_f32 / 3.0;
        for (i, seg) in segments.iter().enumerate() {
            let expected_start = per * i as f32;
            let expected_end = per * (i as f32 + 1.0);
            assert!((seg.start - expected_start).abs() < 1e-3);
            assert!((seg.end - expected_end).abs() < 1e-3);
        }
    }

    #[test]
    fn mock_adapter_round_trips_fixture() {
        use adapter::{AudioInfo, MockAdapter, TranscriptionModelAdapter};
        use transcribe_rs::TranscriptionResult;

        let mock = MockTranscription::from_fixture(fixture_path()).expect("fixture loads");
        let segments = match mock {
            MockTranscription::Fixture { segments, .. } => segments,
            _ => panic!("expected Fixture"),
        };
        let raw = TranscriptionResult {
            text: String::new(),
            segments: Some(segments),
        };
        let audio = AudioInfo {
            duration_us: 6_000_000,
        };
        let out = MockAdapter.adapt(raw, audio).expect("adapt ok");
        out.validate().expect("invariants hold");
        assert!(!out.words.is_empty());
    }
}
