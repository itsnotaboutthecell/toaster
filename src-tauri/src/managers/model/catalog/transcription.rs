//! Static transcription-model catalog entries.
//!
//! Returned as a flat `Vec<ModelInfo>` (all with `category = Transcription`);
//! the aggregator in `catalog::mod` folds these plus `post_processor::entries()`
//! into the `HashMap` consumed by `ModelManager::new`.

use std::collections::HashMap;

use super::super::{EngineType, ModelCategory, ModelInfo};

pub(super) fn entries() -> Vec<ModelInfo> {
    let mut available_models: HashMap<String, ModelInfo> = HashMap::new();

    // Whisper supported languages (99 languages from tokenizer)
    // Including zh-Hans and zh-Hant variants to match frontend language codes
    let whisper_languages: Vec<String> = vec![
        "en", "zh", "zh-Hans", "zh-Hant", "de", "es", "ru", "ko", "fr", "ja", "pt", "tr", "pl",
        "ca", "nl", "ar", "sv", "it", "id", "hi", "fi", "vi", "he", "uk", "el", "ms", "cs", "ro",
        "da", "hu", "ta", "no", "th", "ur", "hr", "bg", "lt", "la", "mi", "ml", "cy", "sk", "te",
        "fa", "lv", "bn", "sr", "az", "sl", "kn", "et", "mk", "br", "eu", "is", "hy", "ne", "mn",
        "bs", "kk", "sq", "sw", "gl", "mr", "pa", "si", "km", "sn", "yo", "so", "af", "oc", "ka",
        "be", "tg", "sd", "gu", "am", "yi", "lo", "uz", "fo", "ht", "ps", "tk", "nn", "mt", "sa",
        "lb", "my", "bo", "tl", "mg", "as", "tt", "haw", "ln", "ha", "ba", "jw", "su", "yue",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    // TODO this should be read from a JSON file or something..
    available_models.insert(
        "small".to_string(),
        ModelInfo {
            id: "small".to_string(),
            name: "Whisper Small".to_string(),
            description: "Fast and fairly accurate.".to_string(),
            filename: "ggml-small.bin".to_string(),
            url: Some("https://blob.handy.computer/ggml-small.bin".to_string()),
            sha256: Some(
                "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b".to_string(),
            ),
            size_mb: 465,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: false,
            engine_type: EngineType::Whisper,
            accuracy_score: 0.60,
            speed_score: 0.85,
            supports_translation: true,
            is_recommended: false,
            supported_languages: whisper_languages.clone(),
            supports_language_selection: true,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    // Add downloadable models
    available_models.insert(
        "medium".to_string(),
        ModelInfo {
            id: "medium".to_string(),
            name: "Whisper Medium".to_string(),
            description: "Good accuracy, medium speed".to_string(),
            filename: "whisper-medium-q4_1.bin".to_string(),
            url: Some("https://blob.handy.computer/whisper-medium-q4_1.bin".to_string()),
            sha256: Some(
                "79283fc1f9fe12ca3248543fbd54b73292164d8df5a16e095e2bceeaaabddf57".to_string(),
            ),
            size_mb: 469,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: false,
            engine_type: EngineType::Whisper,
            accuracy_score: 0.75,
            speed_score: 0.60,
            supports_translation: true,
            is_recommended: false,
            supported_languages: whisper_languages.clone(),
            supports_language_selection: true,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    available_models.insert(
        "turbo".to_string(),
        ModelInfo {
            id: "turbo".to_string(),
            name: "Whisper Turbo".to_string(),
            description: "Balanced accuracy and speed.".to_string(),
            filename: "ggml-large-v3-turbo.bin".to_string(),
            url: Some("https://blob.handy.computer/ggml-large-v3-turbo.bin".to_string()),
            sha256: Some(
                "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69".to_string(),
            ),
            size_mb: 1549,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: false,
            engine_type: EngineType::Whisper,
            accuracy_score: 0.80,
            speed_score: 0.40,
            supports_translation: false, // Turbo doesn't support translation
            is_recommended: false,
            supported_languages: whisper_languages.clone(),
            supports_language_selection: true,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    available_models.insert(
        "large".to_string(),
        ModelInfo {
            id: "large".to_string(),
            name: "Whisper Large".to_string(),
            description: "Good accuracy, but slow.".to_string(),
            filename: "ggml-large-v3-q5_0.bin".to_string(),
            url: Some("https://blob.handy.computer/ggml-large-v3-q5_0.bin".to_string()),
            sha256: Some(
                "d75795ecff3f83b5faa89d1900604ad8c780abd5739fae406de19f23ecd98ad1".to_string(),
            ),
            size_mb: 1031,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: false,
            engine_type: EngineType::Whisper,
            accuracy_score: 0.85,
            speed_score: 0.30,
            supports_translation: true,
            is_recommended: false,
            supported_languages: whisper_languages.clone(),
            supports_language_selection: true,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    available_models.insert(
        "breeze-asr".to_string(),
        ModelInfo {
            id: "breeze-asr".to_string(),
            name: "Breeze ASR".to_string(),
            description: "Optimized for Taiwanese Mandarin. Code-switching support.".to_string(),
            filename: "breeze-asr-q5_k.bin".to_string(),
            url: Some("https://blob.handy.computer/breeze-asr-q5_k.bin".to_string()),
            sha256: Some(
                "8efbf0ce8a3f50fe332b7617da787fb81354b358c288b008d3bdef8359df64c6".to_string(),
            ),
            size_mb: 1030,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: false,
            engine_type: EngineType::Whisper,
            accuracy_score: 0.85,
            speed_score: 0.35,
            supports_translation: false,
            is_recommended: false,
            supported_languages: whisper_languages,
            supports_language_selection: true,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    // Add NVIDIA Parakeet models(directory-based)
    available_models.insert(
        "parakeet-tdt-0.6b-v2".to_string(),
        ModelInfo {
            id: "parakeet-tdt-0.6b-v2".to_string(),
            name: "Parakeet V2".to_string(),
            description: "English only. The best model for English speakers.".to_string(),
            filename: "parakeet-tdt-0.6b-v2-int8".to_string(), // Directory name
            url: Some("https://blob.handy.computer/parakeet-v2-int8.tar.gz".to_string()),
            sha256: Some(
                "ac9b9429984dd565b25097337a887bb7f0f8ac393573661c651f0e7d31563991".to_string(),
            ),
            size_mb: 451,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::Parakeet,
            accuracy_score: 0.85,
            speed_score: 0.85,
            supports_translation: false,
            is_recommended: false,
            supported_languages: vec!["en".to_string()],
            supports_language_selection: false,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    // Parakeet V3 supported languages(25 EU languages + Russian/Ukrainian):
    // bg, hr, cs, da, nl, en, et, fi, fr, de, el, hu, it, lv, lt, mt, pl, pt, ro, sk, sl, es, sv, ru, uk
    let parakeet_v3_languages: Vec<String> = vec![
        "bg", "hr", "cs", "da", "nl", "en", "et", "fi", "fr", "de", "el", "hu", "it", "lv", "lt",
        "mt", "pl", "pt", "ro", "sk", "sl", "es", "sv", "ru", "uk",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    available_models.insert(
        "parakeet-tdt-0.6b-v3".to_string(),
        ModelInfo {
            id: "parakeet-tdt-0.6b-v3".to_string(),
            name: "Parakeet V3".to_string(),
            description: "Fast and accurate. Supports 25 European languages.".to_string(),
            filename: "parakeet-tdt-0.6b-v3-int8".to_string(), // Directory name
            url: Some("https://blob.handy.computer/parakeet-v3-int8.tar.gz".to_string()),
            sha256: Some(
                "43d37191602727524a7d8c6da0eef11c4ba24320f5b4730f1a2497befc2efa77".to_string(),
            ),
            size_mb: 456,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::Parakeet,
            accuracy_score: 0.80,
            speed_score: 0.85,
            supports_translation: false,
            is_recommended: true,
            supported_languages: parakeet_v3_languages,
            supports_language_selection: false,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    available_models.insert(
        "moonshine-base".to_string(),
        ModelInfo {
            id: "moonshine-base".to_string(),
            name: "Moonshine Base".to_string(),
            description: "Very fast, English only. Handles accents well.".to_string(),
            filename: "moonshine-base".to_string(),
            url: Some("https://blob.handy.computer/moonshine-base.tar.gz".to_string()),
            sha256: Some(
                "04bf6ab012cfceebd4ac7cf88c1b31d027bbdd3cd704649b692e2e935236b7e8".to_string(),
            ),
            size_mb: 55,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::Moonshine,
            accuracy_score: 0.70,
            speed_score: 0.90,
            supports_translation: false,
            is_recommended: false,
            supported_languages: vec!["en".to_string()],
            supports_language_selection: false,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    available_models.insert(
        "moonshine-tiny-streaming-en".to_string(),
        ModelInfo {
            id: "moonshine-tiny-streaming-en".to_string(),
            name: "Moonshine V2 Tiny".to_string(),
            description: "Ultra-fast, English only".to_string(),
            filename: "moonshine-tiny-streaming-en".to_string(),
            url: Some("https://blob.handy.computer/moonshine-tiny-streaming-en.tar.gz".to_string()),
            sha256: Some(
                "465addcfca9e86117415677dfdc98b21edc53537210333a3ecdb58509a80abaf".to_string(),
            ),
            size_mb: 31,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::MoonshineStreaming,
            accuracy_score: 0.55,
            speed_score: 0.95,
            supports_translation: false,
            is_recommended: false,
            supported_languages: vec!["en".to_string()],
            supports_language_selection: false,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    available_models.insert(
        "moonshine-small-streaming-en".to_string(),
        ModelInfo {
            id: "moonshine-small-streaming-en".to_string(),
            name: "Moonshine V2 Small".to_string(),
            description: "Fast, English only. Good balance of speed and accuracy.".to_string(),
            filename: "moonshine-small-streaming-en".to_string(),
            url: Some(
                "https://blob.handy.computer/moonshine-small-streaming-en.tar.gz".to_string(),
            ),
            sha256: Some(
                "dbb3e1c1832bd88a4ac712f7449a136cc2c9a18c5fe33a12ed1b7cb1cfe9cdd5".to_string(),
            ),
            size_mb: 99,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::MoonshineStreaming,
            accuracy_score: 0.65,
            speed_score: 0.90,
            supports_translation: false,
            is_recommended: false,
            supported_languages: vec!["en".to_string()],
            supports_language_selection: false,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    available_models.insert(
        "moonshine-medium-streaming-en".to_string(),
        ModelInfo {
            id: "moonshine-medium-streaming-en".to_string(),
            name: "Moonshine V2 Medium".to_string(),
            description: "English only. High quality.".to_string(),
            filename: "moonshine-medium-streaming-en".to_string(),
            url: Some(
                "https://blob.handy.computer/moonshine-medium-streaming-en.tar.gz".to_string(),
            ),
            sha256: Some(
                "07a66f3bff1c77e75a2f637e5a263928a08baae3c29c4c053fc968a9a9373d13".to_string(),
            ),
            size_mb: 192,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::MoonshineStreaming,
            accuracy_score: 0.75,
            speed_score: 0.80,
            supports_translation: false,
            is_recommended: false,
            supported_languages: vec!["en".to_string()],
            supports_language_selection: false,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    // SenseVoice supported languages
    let sense_voice_languages: Vec<String> =
        vec!["zh", "zh-Hans", "zh-Hant", "en", "yue", "ja", "ko"]
            .into_iter()
            .map(String::from)
            .collect();

    available_models.insert(
        "sense-voice-int8".to_string(),
        ModelInfo {
            id: "sense-voice-int8".to_string(),
            name: "SenseVoice".to_string(),
            description: "Very fast. Chinese, English, Japanese, Korean, Cantonese.".to_string(),
            filename: "sense-voice-int8".to_string(),
            url: Some("https://blob.handy.computer/sense-voice-int8.tar.gz".to_string()),
            sha256: Some(
                "171d611fe5d353a50bbb741b6f3ef42559b1565685684e9aa888ef563ba3e8a4".to_string(),
            ),
            size_mb: 152,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::SenseVoice,
            accuracy_score: 0.65,
            speed_score: 0.95,
            supports_translation: false,
            is_recommended: false,
            supported_languages: sense_voice_languages,
            supports_language_selection: true,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    // GigaAM v3 supported languages
    let gigaam_languages: Vec<String> = vec!["ru"].into_iter().map(String::from).collect();

    available_models.insert(
        "gigaam-v3-e2e-ctc".to_string(),
        ModelInfo {
            id: "gigaam-v3-e2e-ctc".to_string(),
            name: "GigaAM v3".to_string(),
            description: "Russian speech recognition. Fast and accurate.".to_string(),
            filename: "giga-am-v3-int8".to_string(),
            url: Some("https://blob.handy.computer/giga-am-v3-int8.tar.gz".to_string()),
            sha256: Some(
                "d872462268430db140b69b72e0fc4b787b194c1dbe51b58de39444d55b6da45b".to_string(),
            ),
            size_mb: 151,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::GigaAM,
            accuracy_score: 0.85,
            speed_score: 0.75,
            supports_translation: false,
            is_recommended: false,
            supported_languages: gigaam_languages,
            supports_language_selection: false,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    // Canary 180m Flashsupported languages (4 languages)
    let canary_flash_languages: Vec<String> = vec!["en", "de", "es", "fr"]
        .into_iter()
        .map(String::from)
        .collect();

    available_models.insert(
        "canary-180m-flash".to_string(),
        ModelInfo {
            id: "canary-180m-flash".to_string(),
            name: "Canary 180M Flash".to_string(),
            description: "Very fast. English, German, Spanish, French. Supports translation."
                .to_string(),
            filename: "canary-180m-flash".to_string(),
            url: Some("https://blob.handy.computer/canary-180m-flash.tar.gz".to_string()),
            sha256: Some(
                "6d9cfca6118b296e196eaedc1c8fa9788305a7b0f1feafdb6dc91932ab6e53f7".to_string(),
            ),
            size_mb: 146,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::Canary,
            accuracy_score: 0.75,
            speed_score: 0.85,
            supports_translation: true,
            is_recommended: false,
            supported_languages: canary_flash_languages,
            supports_language_selection: true,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    // Canary 1B v2supported languages (25 EU languages)
    let canary_1b_languages: Vec<String> = vec![
        "bg", "hr", "cs", "da", "nl", "en", "et", "fi", "fr", "de", "el", "hu", "it", "lv", "lt",
        "mt", "pl", "pt", "ro", "sk", "sl", "es", "sv", "ru", "uk",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    available_models.insert(
        "canary-1b-v2".to_string(),
        ModelInfo {
            id: "canary-1b-v2".to_string(),
            name: "Canary 1B v2".to_string(),
            description: "Accurate multilingual. 25 European languages. Supports translation."
                .to_string(),
            filename: "canary-1b-v2".to_string(),
            url: Some("https://blob.handy.computer/canary-1b-v2.tar.gz".to_string()),
            sha256: Some(
                "02305b2a25f9cf3e7deaffa7f94df00efa44f442cd55c101c2cb9c000f904666".to_string(),
            ),
            size_mb: 691,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::Canary,
            accuracy_score: 0.85,
            speed_score: 0.70,
            supports_translation: true,
            is_recommended: false,
            supported_languages: canary_1b_languages,
            supports_language_selection: true,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    let cohere_languages: Vec<String> = vec![
        "en", "fr", "de", "it", "es", "pt", "el", "nl", "pl", "zh", "zh-Hans", "zh-Hant", "ja",
        "ko", "vi", "ar",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    available_models.insert(
        "cohere-int8".to_string(),
        ModelInfo {
            id: "cohere-int8".to_string(),
            name: "Cohere".to_string(),
            description: "A large, slower, but very accurate multilingual model.".to_string(),
            filename: "cohere-int8".to_string(),
            url: Some("https://blob.handy.computer/cohere-int8.tar.gz".to_string()),
            sha256: Some(
                "ea2257d52434f3644574f187dcdcf666e302cd11b92866116ab8e14cd9c887f0".to_string(),
            ),
            size_mb: 1708,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: true,
            engine_type: EngineType::Cohere,
            accuracy_score: 0.90,
            speed_score: 0.60,
            supports_translation: false,
            is_recommended: false,
            supported_languages: cohere_languages,
            supports_language_selection: true,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        },
    );

    available_models.into_values().collect()
}
