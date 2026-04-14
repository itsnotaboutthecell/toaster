use log::info;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use transcribe_rs::TranscriptionSegment;

use crate::commands::editor::EditorStore;
use crate::managers::editor::Word;
use crate::managers::transcription::TranscriptionManager;

/// Build word-level timestamps from transcription segments.
///
/// Each segment has a start/end time and text. We split each segment's text
/// into words and distribute timestamps proportionally by character length.
/// This produces timestamps that are accurate to within a segment (~30s chunks
/// from Whisper), with proportional distribution within each segment being
/// much better than global even distribution.
fn build_words_from_segments(full_text: &str, segments: &[TranscriptionSegment]) -> Vec<Word> {
    let mut words = Vec::new();

    // The filtered text may differ from segment text (due to filler filtering,
    // custom word correction). We'll use the final text's words and match them
    // against segment boundaries for the best timestamp assignment.
    let final_words: Vec<&str> = full_text.split_whitespace().collect();

    if final_words.is_empty() || segments.is_empty() {
        return words;
    }

    // Build a flat list of (word, start_us, end_us) from segments first
    let mut segment_words: Vec<(String, i64, i64)> = Vec::new();
    for seg in segments {
        let seg_text = seg.text.trim();
        if seg_text.is_empty() {
            continue;
        }
        let seg_start_us = (seg.start as f64 * 1_000_000.0) as i64;
        let seg_end_us = (seg.end as f64 * 1_000_000.0) as i64;
        let seg_duration_us = seg_end_us - seg_start_us;

        let seg_words: Vec<&str> = seg_text.split_whitespace().collect();
        if seg_words.is_empty() {
            continue;
        }

        // Total character count for proportional distribution
        let total_chars: usize = seg_words.iter().map(|w| w.len().max(1)).sum();

        let mut cursor_us = seg_start_us;
        for (j, sw) in seg_words.iter().enumerate() {
            let char_fraction = sw.len().max(1) as f64 / total_chars as f64;
            let word_duration_us = (seg_duration_us as f64 * char_fraction) as i64;

            let word_start = cursor_us;
            let word_end = if j == seg_words.len() - 1 {
                seg_end_us // last word gets the remainder to avoid gaps
            } else {
                cursor_us + word_duration_us
            };

            segment_words.push((sw.to_string(), word_start, word_end));
            cursor_us = word_end;
        }
    }

    // Now match filtered final_words against segment_words.
    // The final text may have had filler words removed or words corrected,
    // so we do a greedy forward match. If a final word matches a segment word,
    // use that segment word's timestamps. If not, interpolate.
    let mut seg_idx = 0;
    for fw in &final_words {
        let fw_lower = fw.to_lowercase();

        // Try to find a matching segment word from current position forward.
        // Use a large lookahead (20 words) to tolerate filler removal, stutters,
        // and word corrections that can shift alignment significantly.
        let mut found = false;
        let search_limit = (seg_idx + 20).min(segment_words.len());
        for k in seg_idx..search_limit {
            let seg_word_lower = segment_words[k].0.to_lowercase();
            // Fuzzy match: segment text might have punctuation attached
            if seg_word_lower == fw_lower
                || seg_word_lower.starts_with(&fw_lower)
                || fw_lower.starts_with(&seg_word_lower)
                || seg_word_lower.trim_matches(|c: char| !c.is_alphanumeric()) == fw_lower
            {
                words.push(Word {
                    text: fw.to_string(),
                    start_us: segment_words[k].1,
                    end_us: segment_words[k].2,
                    deleted: false,
                    silenced: false,
                    confidence: -1.0,
                    speaker_id: -1,
                });
                seg_idx = k + 1;
                found = true;
                break;
            }
        }

        if !found {
            // No match found — interpolate from nearest segment word and advance
            // the pointer so subsequent words don't all pile up at the same position
            let (start, end) = if seg_idx < segment_words.len() {
                let ts = (segment_words[seg_idx].1, segment_words[seg_idx].2);
                seg_idx += 1; // advance past this word to prevent repeated timestamps
                ts
            } else if !segment_words.is_empty() {
                let last = segment_words.last().unwrap();
                (last.1, last.2)
            } else {
                (0, 0)
            };
            info!(
                "build_words_from_segments: no match for '{}' at seg_idx={}, using interpolated {}-{}us",
                fw, seg_idx, start, end
            );
            words.push(Word {
                text: fw.to_string(),
                start_us: start,
                end_us: end,
                deleted: false,
                silenced: false,
                confidence: -1.0,
                speaker_id: -1,
            });
        }
    }

    words
}

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

    // Transcribe — now returns segments with real timestamps
    let (text, segments) = tm
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

    // Build words with real timestamps from transcription segments.
    // Segments have start/end in seconds — we distribute words within each
    // segment proportionally by character count (a reasonable proxy for speech
    // duration). This is far more accurate than the previous approach of
    // dividing total audio duration evenly across all words.
    let words: Vec<Word> = if let Some(ref segs) = segments {
        build_words_from_segments(&text, segs)
    } else {
        // Fallback: no segments available (some engines don't provide them).
        // Distribute words evenly across total duration (legacy behavior).
        let sample_rate = 16000.0_f64;
        let total_duration_us = ((samples.len() as f64 / sample_rate) * 1_000_000.0) as i64;
        let raw_words: Vec<&str> = text.split_whitespace().collect();
        let word_duration_us = if raw_words.is_empty() {
            0
        } else {
            total_duration_us / raw_words.len() as i64
        };
        raw_words
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
            .collect()
    };

    if words.is_empty() {
        return Err("No words in transcription".to_string());
    }

    // Populate the editor
    let mut state = editor_store.0.lock().unwrap();
    state.set_words(words.clone());

    Ok(state.get_words().to_vec())
}
