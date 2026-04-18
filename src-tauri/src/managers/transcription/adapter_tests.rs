//! Extracted from the inline `mod tests` block (monolith-split).

use super::*;

fn mkword(text: &str, s: i64, e: i64) -> CanonicalWord {
    CanonicalWord {
        text: text.to_string(),
        start_us: s,
        end_us: e,
        confidence: -1.0,
        speaker_id: -1,
        is_non_speech: false,
    }
}

fn wrap(words: Vec<CanonicalWord>) -> NormalizedTranscriptionResult {
    NormalizedTranscriptionResult {
        words,
        text: String::new(),
        segments: None,
        language: "und".to_string(),
        word_timestamps_authoritative: true,
    }
}

fn tr(segments: Vec<TranscriptionSegment>) -> TranscriptionResult {
    TranscriptionResult {
        text: String::new(),
        segments: Some(segments),
    }
}

fn seg(start: f32, end: f32, text: &str) -> TranscriptionSegment {
    TranscriptionSegment {
        start,
        end,
        text: text.to_string(),
    }
}

#[test]
fn canonical_word_validates_monotonic_non_overlap() {
    assert!(wrap(vec![mkword("a", 0, 100), mkword("b", 100, 200)])
        .validate()
        .is_ok());
    assert!(
        wrap(vec![mkword("a", 0, 100), mkword("b", 200, 300)])
            .validate()
            .is_ok(),
        "gaps are allowed"
    );
    assert!(wrap(vec![mkword("a", 0, 150), mkword("b", 100, 200)])
        .validate()
        .is_err());
}

#[test]
fn canonical_word_validates_no_zero_duration() {
    assert!(wrap(vec![mkword("x", 100, 100)]).validate().is_err());
    assert!(wrap(vec![mkword("x", 100, 50)]).validate().is_err());
}

#[test]
fn canonical_word_validates_rejects_non_speech() {
    let mut w = mkword("[MUSIC]", 0, 100);
    w.is_non_speech = true;
    assert!(wrap(vec![w]).validate().is_err());
}

#[test]
fn is_non_speech_catches_hallucinations() {
    assert!(is_non_speech_token("[MUSIC]"));
    assert!(is_non_speech_token("[Applause]"));
    assert!(is_non_speech_token("<|nospeech|>"));
    assert!(is_non_speech_token("<unk>"));
    assert!(is_non_speech_token("♪♪"));
    assert!(is_non_speech_token(" ♪ ♫ ♪ "));
    assert!(is_non_speech_token("...."));
    assert!(is_non_speech_token("----"));
    assert!(!is_non_speech_token("hello"));
    assert!(!is_non_speech_token("the music was loud"));
}

#[test]
fn whisper_adapter_strips_hallucination_patterns() {
    let raw = tr(vec![
        seg(0.0, 0.5, " hello world"),
        seg(0.5, 0.8, " [MUSIC]"),
        seg(0.8, 1.0, " ♪♪"),
        seg(1.0, 1.4, " <|nospeech|>"),
        seg(1.4, 2.0, " goodbye"),
    ]);
    let audio = AudioInfo::from_samples(32_000, 16_000, 1); // 2s
    let out = WhisperAdapter.adapt(raw, audio).expect("adapt ok");
    let texts: Vec<_> = out.words.iter().map(|w| w.text.as_str()).collect();
    assert_eq!(texts, vec!["hello", "world", "goodbye"]);
    // Authoritative by pipeline contract — see WhisperAdapter::adapt.
    assert!(out.word_timestamps_authoritative);
    for w in &out.words {
        assert!(!w.is_non_speech);
        assert!(w.start_us < w.end_us);
    }
}

#[test]
fn parakeet_adapter_preserves_native_word_times() {
    let raw = tr(vec![
        seg(0.10, 0.45, "hello"),
        seg(0.50, 0.80, "world"),
        seg(0.90, 1.20, "bye"),
    ]);
    let audio = AudioInfo::from_samples(32_000, 16_000, 1);
    let out = ParakeetAdapter.adapt(raw, audio).expect("adapt ok");
    assert!(out.word_timestamps_authoritative);
    assert_eq!(out.words.len(), 3);
    assert_eq!(out.words[0].start_us, 100_000);
    assert_eq!(out.words[0].end_us, 450_000);
    assert_eq!(out.words[1].start_us, 500_000);
    assert_eq!(out.words[1].end_us, 800_000);
    assert_eq!(out.words[2].start_us, 900_000);
    assert_eq!(out.words[2].end_us, 1_200_000);
}

#[test]
fn parakeet_adapter_strips_unk_tokens() {
    let raw = tr(vec![
        seg(0.0, 0.3, "hello"),
        seg(0.3, 0.5, "<unk>"),
        seg(0.5, 0.9, "world"),
    ]);
    let audio = AudioInfo::from_samples(16_000, 16_000, 1);
    let out = ParakeetAdapter.adapt(raw, audio).expect("adapt ok");
    let texts: Vec<_> = out.words.iter().map(|w| w.text.as_str()).collect();
    assert_eq!(texts, vec!["hello", "world"]);
}

#[test]
fn whisper_language_normalization() {
    let w = WhisperAdapter;
    assert_eq!(w.normalize_language("auto"), None);
    assert_eq!(w.normalize_language(""), None);
    assert_eq!(w.normalize_language("zh-Hans").as_deref(), Some("zh"));
    assert_eq!(w.normalize_language("zh-Hant").as_deref(), Some("zh"));
    assert_eq!(w.normalize_language("en").as_deref(), Some("en"));
}

#[test]
fn moonshine_ignores_language() {
    assert_eq!(MoonshineAdapter.normalize_language("en"), None);
    assert_eq!(MoonshineAdapter.normalize_language("auto"), None);
}

#[test]
fn sense_voice_language_whitelist() {
    let sv = SenseVoiceAdapter;
    assert_eq!(sv.normalize_language("zh-Hant").as_deref(), Some("zh"));
    assert_eq!(sv.normalize_language("ja").as_deref(), Some("ja"));
    assert_eq!(sv.normalize_language("fr"), None);
}

#[test]
fn adapter_for_engine_returns_matching_impl() {
    let w = adapter_for_engine(&EngineType::Whisper);
    assert!(w.capabilities().supports_prompt_injection);
    let p = adapter_for_engine(&EngineType::Parakeet);
    assert!(!p.capabilities().supports_prompt_injection);
    assert!(p.capabilities().has_pre_speech_padding);
    assert!(p.capabilities().supports_fuzzy_word_correction);
}

#[test]
fn mock_adapter_round_trips_fixture() {
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mock_transcription_sample.json");
    let raw = std::fs::read_to_string(&fixture).expect("fixture exists");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json parses");
    let segments: Vec<TranscriptionSegment> = v["segments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            seg(
                s["start"].as_f64().unwrap() as f32,
                s["end"].as_f64().unwrap() as f32,
                s["text"].as_str().unwrap(),
            )
        })
        .collect();
    let result = MockAdapter
        .adapt(tr(segments), AudioInfo::from_samples(16_000 * 6, 16_000, 1))
        .expect("adapt ok");
    result.validate().expect("invariants hold");
    assert!(!result.words.is_empty());
}

#[test]
fn audio_info_from_samples_computes_duration() {
    let info = AudioInfo::from_samples(32_000, 16_000, 1);
    assert_eq!(info.duration_us, 2_000_000);
}

// ── p3-abandon-even-dist-fallback ──────────────────────────────────────
//
// Adapters must refuse to produce a `NormalizedTranscriptionResult` from
// an engine that emitted text but no segment-level timings. Previously
// `commands::transcribe_file` papered over this by synthesizing
// equal-duration word timestamps downstream; that fallback has been
// removed. These tests lock in the new contract per engine.

fn raw_text_no_segments(text: &str) -> TranscriptionResult {
    TranscriptionResult {
        text: text.to_string(),
        segments: None,
    }
}

fn raw_text_empty_segments(text: &str) -> TranscriptionResult {
    TranscriptionResult {
        text: text.to_string(),
        segments: Some(Vec::new()),
    }
}

#[test]
fn every_adapter_errs_when_engine_returns_text_without_segments() {
    let audio = AudioInfo::from_samples(32_000, 16_000, 1);
    let adapters: Vec<(&str, &dyn TranscriptionModelAdapter)> = vec![
        ("Whisper", &WhisperAdapter),
        ("Parakeet", &ParakeetAdapter),
        ("Moonshine", &MoonshineAdapter),
        ("SenseVoice", &SenseVoiceAdapter),
        ("GigaAM", &GigaAmAdapter),
        ("Canary", &CanaryAdapter),
        ("Cohere", &CohereAdapter),
        ("Mock", &MockAdapter),
    ];
    for (name, a) in adapters {
        let err = a
            .adapt(raw_text_no_segments("hello world"), audio)
            .expect_err(&format!(
                "{name}: adapter must Err when engine emits text but no segments"
            ));
        let msg = err.to_string();
        assert!(
            msg.contains(name),
            "{name}: error message must name the offending engine, got {msg:?}"
        );
        assert!(
            msg.contains("equal-duration") || msg.contains("no segment"),
            "{name}: error must explain the contract violation, got {msg:?}"
        );

        // Same contract for an empty segments vec.
        assert!(
            a.adapt(raw_text_empty_segments("hello world"), audio)
                .is_err(),
            "{name}: empty segments vec with non-empty text must also Err"
        );
    }
}

#[test]
fn adapters_accept_empty_text_and_empty_segments() {
    // True silence: no text, no segments. Adapters should return an
    // empty-word result, not Err.
    let audio = AudioInfo::from_samples(32_000, 16_000, 1);
    let silent = TranscriptionResult {
        text: String::new(),
        segments: None,
    };
    let out = WhisperAdapter
        .adapt(silent, audio)
        .expect("silence is not a contract violation");
    assert!(out.words.is_empty());
}
