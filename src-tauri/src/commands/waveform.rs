use log::{debug, info, warn};
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use tauri::State;

use crate::commands::editor::EditorStore;
use crate::managers::media::MediaStore;

const EXPORT_SEAM_FADE_US: i64 = 8_000;
const PREVIEW_CACHE_DIR: &str = "toaster_preview_cache";
const PREVIEW_CACHE_FILE_PREFIX: &str = "preview-";
const PREVIEW_CACHE_FILE_SUFFIX: &str = ".m4a";
const PREVIEW_TOKEN_SEPARATOR: &str = "--";
const PREVIEW_CACHE_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24);

/// A keep-segment: contiguous non-deleted region of the source media.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct KeepSegment {
    pub start_us: i64,
    pub end_us: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PreviewRenderStatus {
    Ready,
    NoSegments,
    MissingMedia,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct PreviewRenderMetadata {
    pub status: PreviewRenderStatus,
    pub preview_file_path: Option<String>,
    pub preview_url_safe_path: Option<String>,
    pub source_media_fingerprint: Option<String>,
    pub edit_version: String,
    pub generation_token: String,
    pub cache_hit: bool,
}

#[derive(Debug, Default)]
struct PreviewCacheCleanupSummary {
    scanned_files: usize,
    removed_files: usize,
    removed_stale_files: usize,
    removed_mismatched_files: usize,
    removed_empty_files: usize,
}

/// Generate waveform peaks from a WAV audio file.
///
/// Returns `peak_count` normalized peak values (0.0–1.0) suitable for rendering
/// a bar-chart waveform. Falls back gracefully if the file cannot be decoded.
#[tauri::command]
#[specta::specta]
pub fn generate_waveform_peaks(
    path: String,
    peak_count: Option<usize>,
) -> Result<Vec<f32>, String> {
    let count = peak_count.unwrap_or(300);
    if count == 0 {
        return Err("peak_count must be > 0".to_string());
    }

    let file_path = std::path::Path::new(&path);
    if !file_path.exists() {
        return Err(format!("File not found: {}", path));
    }

    // Read WAV samples via hound
    let samples = crate::audio_toolkit::read_wav_samples(file_path)
        .map_err(|e| format!("Failed to read audio: {}", e))?;

    if samples.is_empty() {
        return Ok(vec![0.0; count]);
    }

    // Downsample into peaks
    let block_size = samples.len() / count;
    if block_size == 0 {
        // Fewer samples than peaks — pad with zeros
        let mut peaks: Vec<f32> = samples.iter().map(|s| s.abs()).collect();
        peaks.resize(count, 0.0);
        return Ok(normalize_peaks(peaks));
    }

    let mut peaks = Vec::with_capacity(count);
    for i in 0..count {
        let start = i * block_size;
        let end = if i == count - 1 {
            samples.len()
        } else {
            (i + 1) * block_size
        };
        let max = samples[start..end]
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);
        peaks.push(max);
    }

    Ok(normalize_peaks(peaks))
}

fn normalize_peaks(mut peaks: Vec<f32>) -> Vec<f32> {
    let global_max = peaks.iter().copied().fold(0.01_f32, f32::max);
    for p in &mut peaks {
        *p /= global_max;
    }
    peaks
}

fn seam_fade_duration_seconds(start_us: i64, end_us: i64) -> Option<f64> {
    let duration_us = (end_us - start_us).max(0);
    let fade_us = EXPORT_SEAM_FADE_US.min(duration_us / 2);
    (fade_us > 0).then_some(fade_us as f64 / 1_000_000.0)
}

fn build_audio_segment_filter(
    index: usize,
    segment_count: usize,
    start_us: i64,
    end_us: i64,
) -> String {
    let start_s = start_us as f64 / 1_000_000.0;
    let end_s = end_us as f64 / 1_000_000.0;
    let duration_s = ((end_us - start_us).max(0)) as f64 / 1_000_000.0;

    let mut filter = format!("[0:a]atrim=start={start_s:.6}:end={end_s:.6},asetpts=PTS-STARTPTS");

    if let Some(fade_s) = seam_fade_duration_seconds(start_us, end_us) {
        if index > 0 {
            filter.push_str(&format!(",afade=t=in:st=0:d={fade_s:.6}"));
        }
        if index + 1 < segment_count {
            let fade_out_start_s = (duration_s - fade_s).max(0.0);
            filter.push_str(&format!(
                ",afade=t=out:st={fade_out_start_s:.6}:d={fade_s:.6}"
            ));
        }
    }

    filter.push_str(&format!("[a{index}]"));
    filter
}

fn build_audio_concat_filter(segments: &[(i64, i64)]) -> String {
    let mut filter_parts = Vec::new();
    let n = segments.len();
    for (i, (start, end)) in segments.iter().enumerate() {
        filter_parts.push(build_audio_segment_filter(i, n, *start, *end));
    }
    let a_inputs: String = (0..n).map(|i| format!("[a{i}]")).collect();
    filter_parts.push(format!("{a_inputs}concat=n={n}:v=0:a=1[outa]"));
    filter_parts.join("; ")
}

fn fnv1a_64_hex(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in input.as_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        match ch {
            ' ' => out.push_str("%20"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            '/' | '-' | '_' | '.' | ':' | '~' => out.push(ch),
            c if c.is_ascii_alphanumeric() => out.push(c),
            c => {
                for byte in c.to_string().as_bytes() {
                    out.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    out
}

fn preview_cache_dir() -> PathBuf {
    std::env::temp_dir().join(PREVIEW_CACHE_DIR)
}

fn preview_generation_token(source_fingerprint: &str, edit_version: &str) -> String {
    format!("{source_fingerprint}{PREVIEW_TOKEN_SEPARATOR}{edit_version}")
}

fn preview_output_path(preview_dir: &Path, generation_token: &str) -> PathBuf {
    preview_dir.join(format!(
        "{PREVIEW_CACHE_FILE_PREFIX}{generation_token}{PREVIEW_CACHE_FILE_SUFFIX}"
    ))
}

fn parse_generation_token(generation_token: &str) -> Option<(&str, &str)> {
    generation_token
        .split_once(PREVIEW_TOKEN_SEPARATOR)
        .or_else(|| generation_token.split_once(':'))
}

fn parse_preview_cache_entry(path: &Path) -> Option<(String, String, String)> {
    let file_name = path.file_name()?.to_str()?;
    let generation_token = file_name
        .strip_prefix(PREVIEW_CACHE_FILE_PREFIX)?
        .strip_suffix(PREVIEW_CACHE_FILE_SUFFIX)?;
    let (source_fingerprint, edit_version) = parse_generation_token(generation_token)?;
    Some((
        generation_token.to_string(),
        source_fingerprint.to_string(),
        edit_version.to_string(),
    ))
}

fn cleanup_preview_cache(
    preview_dir: &Path,
    active_source_fingerprint: Option<&str>,
    active_edit_version: Option<&str>,
) -> PreviewCacheCleanupSummary {
    let mut summary = PreviewCacheCleanupSummary::default();
    let entries = match std::fs::read_dir(preview_dir) {
        Ok(entries) => entries,
        Err(_) => return summary,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        summary.scanned_files += 1;

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                warn!(
                    "Failed to read preview cache metadata for {}: {}",
                    path.display(),
                    error
                );
                continue;
            }
        };

        let is_empty = metadata.len() == 0;
        let is_stale = metadata
            .modified()
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .map(|age| age > PREVIEW_CACHE_MAX_AGE)
            .unwrap_or(false);

        let parsed_entry = parse_preview_cache_entry(&path);
        let is_mismatched = match (
            active_source_fingerprint,
            active_edit_version,
            parsed_entry.as_ref(),
        ) {
            (Some(active_source), Some(active_edit), Some((_, source, edit))) => {
                source != active_source || (source == active_source && edit != active_edit)
            }
            (Some(active_source), None, Some((_, source, _))) => source != active_source,
            _ => false,
        };

        if !(is_empty || is_stale || is_mismatched) {
            continue;
        }

        match std::fs::remove_file(&path) {
            Ok(_) => {
                summary.removed_files += 1;
                if is_empty {
                    summary.removed_empty_files += 1;
                }
                if is_stale {
                    summary.removed_stale_files += 1;
                }
                if is_mismatched {
                    summary.removed_mismatched_files += 1;
                }
            }
            Err(error) => {
                warn!(
                    "Failed to remove preview cache file {}: {}",
                    path.display(),
                    error
                );
            }
        }
    }

    summary
}

fn invalidate_preview_cache_entries(
    preview_dir: &Path,
    generation_token: Option<&str>,
    source_media_fingerprint: Option<&str>,
) -> usize {
    let entries = match std::fs::read_dir(preview_dir) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    let mut removed_files = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let should_remove = parse_preview_cache_entry(&path)
            .map(|(entry_generation_token, entry_source_fingerprint, _)| {
                if let Some(token) = generation_token {
                    entry_generation_token == token
                } else {
                    source_media_fingerprint
                        .map(|source| entry_source_fingerprint == source)
                        .unwrap_or(false)
                }
            })
            .unwrap_or(false);

        if !should_remove {
            continue;
        }

        match std::fs::remove_file(&path) {
            Ok(_) => removed_files += 1,
            Err(error) => warn!(
                "Failed to invalidate preview cache file {}: {}",
                path.display(),
                error
            ),
        }
    }

    removed_files
}

fn source_media_fingerprint(path: &Path) -> Result<String, String> {
    let metadata =
        std::fs::metadata(path).map_err(|e| format!("Cannot read media metadata: {}", e))?;
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let modified_secs = metadata
        .modified()
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let key = format!(
        "{}|{}|{}",
        canonical.to_string_lossy(),
        metadata.len(),
        modified_secs
    );
    Ok(fnv1a_64_hex(&key))
}

fn edit_version_token(segments: &[(i64, i64)]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write_u64(segments.len() as u64);
    for (start_us, end_us) in segments {
        hasher.write_i64(*start_us);
        hasher.write_i64(*end_us);
    }
    format!("{:016x}", hasher.finish())
}

#[tauri::command]
#[specta::specta]
pub fn invalidate_temp_preview_cache(
    generation_token: Option<String>,
    source_media_fingerprint: Option<String>,
    reason: Option<String>,
) -> Result<(), String> {
    let preview_dir = preview_cache_dir();
    let cleanup_summary = cleanup_preview_cache(&preview_dir, None, None);
    let removed_files = invalidate_preview_cache_entries(
        &preview_dir,
        generation_token.as_deref(),
        source_media_fingerprint.as_deref(),
    );

    if removed_files > 0 || cleanup_summary.removed_files > 0 {
        info!(
            "Preview cache invalidated: reason={} removed_files={} cleaned_files={}",
            reason.as_deref().unwrap_or("unspecified"),
            removed_files,
            cleanup_summary.removed_files
        );
    }

    Ok(())
}

/// Get the keep-segments (non-deleted contiguous regions) from the editor.
#[tauri::command]
#[specta::specta]
pub fn get_keep_segments(store: State<EditorStore>) -> Result<Vec<KeepSegment>, String> {
    let state = store.0.lock().unwrap();
    let segments = state
        .get_keep_segments()
        .into_iter()
        .map(|(start_us, end_us)| KeepSegment { start_us, end_us })
        .collect();
    Ok(segments)
}

/// Generate an FFmpeg concat filter script from keep-segments.
///
/// This produces a filter_complex command that can be run with FFmpeg CLI
/// to trim and concatenate the kept portions of the source media.
///
/// Usage: `ffmpeg -i <input> -filter_complex "<output>" -map "[outv]" -map "[outa]" <output_file>`
#[tauri::command]
#[specta::specta]
pub fn generate_ffmpeg_edit_script(
    store: State<EditorStore>,
    input_path: String,
) -> Result<String, String> {
    let state = store.0.lock().unwrap();
    let segments = state.get_keep_segments();

    if segments.is_empty() {
        return Err("No segments to export (all words deleted)".to_string());
    }

    // Build an FFmpeg command line using -ss/-to trim + concat demuxer approach
    let mut lines = Vec::new();
    lines.push(format!("# FFmpeg edit script for: {}", input_path));
    lines.push(format!("# {} segment(s) to keep\n", segments.len()));

    if segments.len() == 1 {
        // Single segment — simple trim
        let (start, end) = segments[0];
        let start_s = start as f64 / 1_000_000.0;
        let end_s = end as f64 / 1_000_000.0;
        lines.push(format!(
            "ffmpeg -i \"{}\" -ss {:.6} -to {:.6} -c copy \"output.mp4\"",
            input_path, start_s, end_s
        ));
    } else {
        // Multiple segments — filter_complex with trim + concat
        let mut filter_parts = Vec::new();
        let n = segments.len();

        for (i, (start, end)) in segments.iter().enumerate() {
            let start_s = *start as f64 / 1_000_000.0;
            let end_s = *end as f64 / 1_000_000.0;
            filter_parts.push(format!(
                "[0:v]trim=start={:.6}:end={:.6},setpts=PTS-STARTPTS[v{i}]; \
                 [0:a]atrim=start={:.6}:end={:.6},asetpts=PTS-STARTPTS[a{i}]",
                start_s, end_s, start_s, end_s
            ));
        }

        let v_inputs: String = (0..n).map(|i| format!("[v{i}]")).collect();
        let a_inputs: String = (0..n).map(|i| format!("[a{i}]")).collect();
        filter_parts.push(format!(
            "{v_inputs}concat=n={n}:v=1:a=0[outv]; {a_inputs}concat=n={n}:v=0:a=1[outa]"
        ));

        let filter = filter_parts.join("; ");
        lines.push(format!(
            "ffmpeg -i \"{}\" -filter_complex \"{}\" -map \"[outv]\" -map \"[outa]\" \"output.mp4\"",
            input_path, filter
        ));
    }

    Ok(lines.join("\n"))
}

/// Map an edit-timeline position back to the source-media position.
///
/// When words are deleted, the edited timeline is shorter than the source.
/// This maps a position on the edit timeline to the corresponding source time.
#[tauri::command]
#[specta::specta]
pub fn map_edit_to_source_time(
    store: State<EditorStore>,
    edit_time_us: i64,
) -> Result<i64, String> {
    let state = store.0.lock().unwrap();
    Ok(state.map_edit_time_to_source_time(edit_time_us))
}

/// Render (or reuse) a temporary preview audio artifact for the current edit state.
#[tauri::command]
#[specta::specta]
pub async fn render_temp_preview_audio(
    store: State<'_, EditorStore>,
    media_store: State<'_, MediaStore>,
) -> Result<PreviewRenderMetadata, String> {
    let render_started_at = Instant::now();
    let segments = {
        let state = store.0.lock().unwrap();
        state.get_keep_segments()
    };

    let edit_version = edit_version_token(&segments);

    let media_info = {
        let state = media_store.0.lock().unwrap();
        state.current().cloned()
    };

    let source_fingerprint = media_info
        .as_ref()
        .and_then(|info| source_media_fingerprint(&info.path).ok());

    if segments.is_empty() {
        return Ok(PreviewRenderMetadata {
            status: PreviewRenderStatus::NoSegments,
            preview_file_path: None,
            preview_url_safe_path: None,
            source_media_fingerprint: source_fingerprint.clone(),
            generation_token: preview_generation_token(
                source_fingerprint.as_deref().unwrap_or("no-media"),
                &edit_version,
            ),
            edit_version,
            cache_hit: false,
        });
    }

    let media = media_info.ok_or_else(|| "No media loaded for preview rendering".to_string())?;
    if !media.path.exists() {
        return Ok(PreviewRenderMetadata {
            status: PreviewRenderStatus::MissingMedia,
            preview_file_path: None,
            preview_url_safe_path: None,
            source_media_fingerprint: source_fingerprint.clone(),
            generation_token: preview_generation_token(
                source_fingerprint.as_deref().unwrap_or("missing-media"),
                &edit_version,
            ),
            edit_version,
            cache_hit: false,
        });
    }

    let source_fingerprint = source_fingerprint
        .ok_or_else(|| "Failed to compute source media fingerprint".to_string())?;
    let generation_token = preview_generation_token(&source_fingerprint, &edit_version);
    let preview_dir = preview_cache_dir();
    std::fs::create_dir_all(&preview_dir)
        .map_err(|e| format!("Failed to create preview cache dir: {}", e))?;
    let cleanup_summary =
        cleanup_preview_cache(&preview_dir, Some(&source_fingerprint), Some(&edit_version));
    if cleanup_summary.removed_files > 0 {
        debug!(
            "Preview cache cleanup removed {} file(s) before render (stale={}, mismatched={}, empty={})",
            cleanup_summary.removed_files,
            cleanup_summary.removed_stale_files,
            cleanup_summary.removed_mismatched_files,
            cleanup_summary.removed_empty_files
        );
    }

    let output_path = preview_output_path(&preview_dir, &generation_token);
    let cache_hit = output_path.exists()
        && std::fs::metadata(&output_path)
            .map(|m| m.len() > 0)
            .unwrap_or(false);

    if !cache_hit {
        let mut args: Vec<String> = vec![
            "-y".to_string(),
            "-i".to_string(),
            media.path.to_string_lossy().to_string(),
            "-vn".to_string(),
        ];

        if segments.len() == 1 {
            let (start, end) = segments[0];
            args.extend([
                "-ss".to_string(),
                format!("{:.6}", start as f64 / 1_000_000.0),
                "-to".to_string(),
                format!("{:.6}", end as f64 / 1_000_000.0),
            ]);
        } else {
            let filter = build_audio_concat_filter(&segments);
            args.extend([
                "-filter_complex".to_string(),
                filter,
                "-map".to_string(),
                "[outa]".to_string(),
            ]);
        }

        args.extend([
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "160k".to_string(),
            "-movflags".to_string(),
            "+faststart".to_string(),
            output_path.to_string_lossy().to_string(),
        ]);

        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("ffmpeg").args(&args).output()
        })
        .await
        .map_err(|e| format!("Preview render task panicked: {}", e))?
        .map_err(|e| {
            format!(
                "FFmpeg not found. Install FFmpeg to render preview audio. Error: {}",
                e
            )
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                "Preview cache miss render failed after {} ms for token {}",
                render_started_at.elapsed().as_millis(),
                generation_token
            );
            return Err(format!("FFmpeg preview render failed: {}", stderr));
        }
    }

    info!(
        "Preview cache {} for token {} in {} ms",
        if cache_hit { "hit" } else { "miss" },
        generation_token,
        render_started_at.elapsed().as_millis()
    );

    let preview_file_path = output_path.to_string_lossy().to_string();
    let preview_asset_path = format!(
        "asset://localhost/{}",
        urlencoding(&preview_file_path.replace('\\', "/"))
    );

    Ok(PreviewRenderMetadata {
        status: PreviewRenderStatus::Ready,
        preview_file_path: Some(preview_file_path),
        preview_url_safe_path: Some(preview_asset_path),
        source_media_fingerprint: Some(source_fingerprint),
        edit_version,
        generation_token,
        cache_hit,
    })
}

/// Export the edited media by running FFmpeg with trim/atrim filters.
///
/// Uses the keep-segments from the editor to produce an output file
/// with deleted segments removed. Supports both audio-only and video+audio.
#[tauri::command]
#[specta::specta]
pub async fn export_edited_media(
    store: State<'_, EditorStore>,
    input_path: String,
    output_path: String,
) -> Result<String, String> {
    let segments = {
        let state = store.0.lock().unwrap();
        state.get_keep_segments()
    };

    if segments.is_empty() {
        return Err("No segments to export (all words deleted)".to_string());
    }

    let input = std::path::Path::new(&input_path);
    if !input.exists() {
        return Err(format!("Input file not found: {}", input_path));
    }

    // Detect if input has video by checking extension
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let has_video = matches!(ext.as_str(), "mp4" | "mkv" | "mov" | "avi" | "webm" | "flv");

    let mut args: Vec<String> = vec!["-y".to_string(), "-i".to_string(), input_path.clone()];

    if segments.len() == 1 {
        // Single segment — simple trim with re-encode for sample-accurate cuts
        let (start, end) = segments[0];
        let start_s = start as f64 / 1_000_000.0;
        let end_s = end as f64 / 1_000_000.0;
        args.extend([
            "-ss".to_string(),
            format!("{:.6}", start_s),
            "-to".to_string(),
            format!("{:.6}", end_s),
        ]);
        // Re-encode audio for sample-accurate cut (stream copy can only cut on keyframes)
        if has_video {
            args.extend(["-c:v".to_string(), "copy".to_string()]);
        }
        args.extend([
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "192k".to_string(),
        ]);
    } else {
        // Multiple segments — filter_complex with trim/atrim + concat
        if has_video {
            let mut filter_parts = Vec::new();
            let n = segments.len();
            for (i, (start, end)) in segments.iter().enumerate() {
                let start_s = *start as f64 / 1_000_000.0;
                let end_s = *end as f64 / 1_000_000.0;
                filter_parts.push(format!(
                    "[0:v]trim=start={start_s:.6}:end={end_s:.6},setpts=PTS-STARTPTS[v{i}]"
                ));
                filter_parts.push(build_audio_segment_filter(i, n, *start, *end));
            }
            let v_inputs: String = (0..n).map(|i| format!("[v{i}]")).collect();
            let a_inputs: String = (0..n).map(|i| format!("[a{i}]")).collect();
            filter_parts.push(format!(
                "{v_inputs}concat=n={n}:v=1:a=0[outv]; {a_inputs}concat=n={n}:v=0:a=1[outa]"
            ));
            let filter = filter_parts.join("; ");
            args.extend([
                "-filter_complex".to_string(),
                filter,
                "-map".to_string(),
                "[outv]".to_string(),
                "-map".to_string(),
                "[outa]".to_string(),
            ]);
        } else {
            let filter = build_audio_concat_filter(&segments);
            args.extend([
                "-filter_complex".to_string(),
                filter,
                "-map".to_string(),
                "[outa]".to_string(),
            ]);
        }
    }

    args.push(output_path.clone());

    log::info!("Running FFmpeg export: ffmpeg {}", args.join(" "));

    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("ffmpeg").args(&args).output()
    })
    .await
    .map_err(|e| format!("Export task panicked: {}", e))?
    .map_err(|e| {
        format!(
            "FFmpeg not found. Install FFmpeg to export edited media. Error: {}",
            e
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("FFmpeg export failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    log::info!("FFmpeg export complete: {}", output_path);
    Ok(format!("Export complete: {}\n{}", output_path, stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_peaks_scales_to_one() {
        let peaks = vec![0.0, 0.5, 1.0, 0.25];
        let result = normalize_peaks(peaks);
        assert!((result[2] - 1.0).abs() < 0.001);
        assert!((result[1] - 0.5).abs() < 0.001);
    }

    #[test]
    fn normalize_peaks_all_zero() {
        let peaks = vec![0.0, 0.0, 0.0];
        let result = normalize_peaks(peaks);
        // global_max floor is 0.01, so all are 0/0.01 = 0
        assert!(result.iter().all(|&p| p < 0.01));
    }

    #[test]
    fn audio_segment_filter_adds_micro_fades_at_joins() {
        let filter = build_audio_segment_filter(1, 3, 1_000_000, 2_000_000);
        assert!(filter.contains("afade=t=in:st=0:d=0.008000"));
        assert!(filter.contains("afade=t=out:st=0.992000:d=0.008000"));
        assert!(filter.ends_with("[a1]"));
    }

    #[test]
    fn audio_segment_filter_scales_fade_for_short_segments() {
        let filter = build_audio_segment_filter(1, 3, 0, 6_000);
        assert!(filter.contains("afade=t=in:st=0:d=0.003000"));
        assert!(filter.contains("afade=t=out:st=0.003000:d=0.003000"));
    }
}
