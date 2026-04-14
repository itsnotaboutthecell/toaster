use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

use crate::commands::editor::EditorStore;
use crate::managers::editor::Word;
use crate::managers::transcription::TranscriptionManager;

/// Transcribe a WAV audio file and populate the editor with word-level results.
///
/// Currently supports WAV files (16kHz mono expected by whisper).
/// For other formats, the audio is read as-is and resampled internally
/// by the transcription engine.
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

    // Read audio samples from WAV file
    let samples = crate::audio_toolkit::read_wav_samples(file_path)
        .map_err(|e| format!("Failed to read audio file: {}", e))?;

    if samples.is_empty() {
        return Err("Audio file contains no samples".to_string());
    }

    // Get the transcription manager
    let tm = app
        .try_state::<Arc<TranscriptionManager>>()
        .ok_or_else(|| "Transcription manager not available".to_string())?;

    // Transcribe
    let text = tm
        .transcribe(samples.clone())
        .map_err(|e| format!("Transcription failed: {}", e))?;

    if text.is_empty() {
        return Err("Transcription produced no text".to_string());
    }

    // Split text into words and estimate timestamps.
    // Since the basic transcription API returns only text (no word timestamps),
    // we distribute timestamps evenly across the audio duration.
    let sample_rate = 16000.0_f64; // whisper expects 16kHz
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
