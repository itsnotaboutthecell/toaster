use log::info;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

use crate::commands::editor::EditorStore;
use crate::managers::editor::Word;
use crate::managers::transcription::TranscriptionManager;

/// Extract audio from any media file to a temporary 16kHz mono WAV using FFmpeg.
/// Returns the path to the temporary WAV file.
fn extract_audio_to_wav(input_path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let temp_dir = std::env::temp_dir().join("toaster_audio");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let wav_path = temp_dir.join(format!(
        "extract_{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    info!("Extracting audio from {} to {}", input_path.display(), wav_path.display());

    let output = std::process::Command::new("ffmpeg")
        .args([
            "-y",                                    // overwrite
            "-i", &input_path.to_string_lossy(),     // input file
            "-vn",                                   // no video
            "-acodec", "pcm_s16le",                  // 16-bit PCM
            "-ar", "16000",                          // 16kHz sample rate
            "-ac", "1",                              // mono
            &wav_path.to_string_lossy().to_string(), // output
        ])
        .output()
        .map_err(|e| format!(
            "FFmpeg not found. Install FFmpeg to transcribe non-WAV files. Error: {}", e
        ))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("FFmpeg audio extraction failed: {}", stderr));
    }

    Ok(wav_path)
}

/// Check if a file is already a WAV file.
fn is_wav_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("wav"))
        .unwrap_or(false)
}

/// Transcribe any audio or video file and populate the editor with word-level results.
///
/// For WAV files, reads samples directly. For all other formats (MP4, MP3, etc.),
/// uses FFmpeg to extract audio to a temporary 16kHz mono WAV first.
#[tauri::command]
#[specta::specta]
pub async fn transcribe_media_file(
    app: AppHandle,
    editor_store: State<'_, EditorStore>,
    path: String,
) -> Result<Vec<Word>, String> {
    let file_path = std::path::Path::new(&path);

    if !file_path.exists() {
        return Err(format!("File not found: {}", path));
    }

    // For non-WAV files, extract audio via FFmpeg first
    let (wav_path, is_temp) = if is_wav_file(file_path) {
        (file_path.to_path_buf(), false)
    } else {
        (extract_audio_to_wav(file_path)?, true)
    };

    // Read audio samples from WAV file
    let samples = crate::audio_toolkit::read_wav_samples(&wav_path)
        .map_err(|e| format!("Failed to read audio: {}", e))?;

    // Clean up temp file
    if is_temp {
        let _ = std::fs::remove_file(&wav_path);
    }

    if samples.is_empty() {
        return Err("Audio file contains no samples".to_string());
    }

    // Get the transcription manager
    let tm = app
        .try_state::<Arc<TranscriptionManager>>()
        .ok_or_else(|| "Transcription manager not available".to_string())?;

    info!("Transcribing {} samples...", samples.len());

    // Ensure model is loaded before transcribing
    if !tm.is_model_loaded() {
        info!("Model not loaded — initiating auto-load...");
        tm.initiate_model_load();
        // Wait for model to finish loading (initiate_model_load is async)
        // The transcribe() call below will wait on the loading condvar
    }

    // Transcribe
    let text = tm
        .transcribe(samples.clone())
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not loaded") {
                "No transcription model loaded. Go to Settings → Models, download a model, then try again.".to_string()
            } else {
                format!("Transcription failed: {}", msg)
            }
        })?;

    if text.is_empty() {
        return Err("Transcription produced no text".to_string());
    }

    // Split text into words and estimate timestamps.
    let sample_rate = 16000.0_f64;
    let total_duration_us = ((samples.len() as f64 / sample_rate) * 1_000_000.0) as i64;

    let raw_words: Vec<&str> = text.split_whitespace().collect();
    if raw_words.is_empty() {
        return Err("No words in transcription".to_string());
    }

    let word_duration_us = total_duration_us / raw_words.len() as i64;
    let words: Vec<Word> = raw_words
        .iter()
        .enumerate()
        .map(|(i, w)| Word {
            text: w.to_string(),
            start_us: i as i64 * word_duration_us,
            end_us: (i as i64 + 1) * word_duration_us,
            deleted: false,
            silenced: false,
            confidence: -1.0,
            speaker_id: -1,
        })
        .collect();

    // Populate the editor
    let mut state = editor_store.0.lock().unwrap();
    state.set_words(words.clone());

    Ok(state.get_words().to_vec())
}
