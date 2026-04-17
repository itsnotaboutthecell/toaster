//! FFmpeg audio extraction helpers.
//!
//! Extracted from `transcribe_file/mod.rs`. Pure, stateless FFmpeg-shell
//! helper that converts arbitrary media to a 16 kHz mono WAV suitable for
//! whisper / transcribe-rs.

use log::info;

/// FFmpeg audio extraction timeout (10 minutes).
const EXTRACT_AUDIO_TIMEOUT_SECS: u64 = 600;

/// Extract audio from any media file to a temporary 16kHz mono WAV using FFmpeg.
/// Returns the path to the temporary WAV file.
pub(super) fn extract_audio_to_wav(
    input_path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let temp_dir = std::env::temp_dir().join("toaster_audio");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let wav_path = temp_dir.join(format!(
        "extract_{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));

    info!(
        "Extracting audio from {} to {}",
        input_path.display(),
        wav_path.display()
    );

    let mut child = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            &input_path.to_string_lossy(),
            "-vn",
            "-acodec",
            "pcm_s16le",
            "-ar",
            "16000",
            "-ac",
            "1",
            wav_path.to_string_lossy().as_ref(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "FFmpeg not found. Install FFmpeg to transcribe non-WAV files. Error: {}",
                e
            )
        })?;

    let timeout = std::time::Duration::from_secs(EXTRACT_AUDIO_TIMEOUT_SECS);
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(&wav_path);
                    return Err(format!(
                        "FFmpeg audio extraction timed out after {} minutes. The input file may be too large.",
                        EXTRACT_AUDIO_TIMEOUT_SECS / 60
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&wav_path);
                return Err(format!("Error waiting for FFmpeg: {}", e));
            }
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to read FFmpeg output: {}", e))?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&wav_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("FFmpeg audio extraction failed: {}", stderr));
    }

    Ok(wav_path)
}

/// Check if a file is already a WAV file.
pub(super) fn is_wav_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("wav"))
        .unwrap_or(false)
}
