use log::{debug, info, warn};
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use tauri::{AppHandle, State};

use crate::commands::editor::EditorStore;
use crate::managers::editor::{EditorState, TimingContractSnapshot};
use crate::managers::media::MediaStore;

const EXPORT_SEAM_FADE_US: i64 = 10_000;
const PREVIEW_SEAM_FADE_US: i64 = 0;
const FIRST_BOUNDARY_FADE_US: i64 = 2_000;
const PREVIEW_CACHE_DIR: &str = "toaster_preview_cache";
const PREVIEW_CACHE_FILE_PREFIX: &str = "preview-";
const PREVIEW_CACHE_FILE_SUFFIX: &str = ".m4a";
const PREVIEW_TOKEN_SEPARATOR: &str = "--";
const PREVIEW_CACHE_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24);
/// FFmpeg preview render timeout (10 minutes).
const PREVIEW_RENDER_TIMEOUT: Duration = Duration::from_secs(600);
/// FFmpeg export timeout (30 minutes).
const EXPORT_TIMEOUT: Duration = Duration::from_secs(1800);

/// Audio post-processing options applied to the entire export output.
#[derive(Debug, Clone, Default)]
struct ExportAudioOptions {
    normalize_audio: bool,
    volume_db: f32,
    fade_in_ms: u32,
    fade_out_ms: u32,
}

/// Escape a file pathfor use inside an FFmpeg filter expression.
///
/// FFmpeg filter syntax treats `:`, `\`, and `'` as special characters.
/// On Windows the path separator `\` must be escaped or converted.
fn escape_srt_path_for_ffmpeg(path: &str) -> String {
    path.replace('\\', "/").replace(':', "\\:")
}

/// Probe the height (in pixels) of the first video stream using ffprobe.
/// Returns `None` when ffprobe is unavailable or the file has no video stream.
fn is_valid_hex_color(s: &str) -> bool {
    let h = s.trim_start_matches('#');
    (h.len() == 6 || h.len() == 8) && h.chars().all(|c| c.is_ascii_hexdigit())
}

fn probe_video_dimensions(path: &str) -> Option<(u32, u32)> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
            path,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let parts: Vec<&str> = raw.split('x').collect();
    if parts.len() == 2 {
        let w = parts[0].parse::<u32>().ok()?;
        let h = parts[1].parse::<u32>().ok()?;
        Some((w, h))
    } else {
        None
    }
}

/// Build the FFmpeg audio post-processing filter chain (volume, fade, normalize)
/// applied to the final mixed output. Returns `None` when no processing is needed.
fn build_audio_post_filters(opts: &ExportAudioOptions, total_duration_s: f64) -> Option<String> {
    let mut filters = Vec::new();

    if opts.volume_db.abs() > f32::EPSILON {
        filters.push(format!("volume={:.1}dB", opts.volume_db));
    }

    if opts.fade_in_ms > 0 {
        let d = opts.fade_in_ms as f64 / 1000.0;
        filters.push(format!("afade=t=in:st=0:d={d:.3}"));
    }

    if opts.fade_out_ms > 0 {
        let d = opts.fade_out_ms as f64 / 1000.0;
        let st = (total_duration_s - d).max(0.0);
        filters.push(format!("afade=t=out:st={st:.3}:d={d:.3}"));
    }

    if opts.normalize_audio {
        filters.push("loudnorm=I=-16:TP=-1.5:LRA=11".to_string());
    }

    if filters.is_empty() {
        None
    } else {
        Some(filters.join(","))
    }
}

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

/// Minimum segment duration (in µs) that is eligible for fades.
/// Segments shorter than this skip fades entirely to avoid distortion.
const MIN_FADEABLE_SEGMENT_US: i64 = 100;

fn seam_fade_duration_seconds(start_us: i64, end_us: i64, seam_fade_us: i64) -> Option<f64> {
    let duration_us = (end_us - start_us).max(0);
    if duration_us < MIN_FADEABLE_SEGMENT_US {
        return None;
    }
    let fade_us = seam_fade_us.min(duration_us / 2);
    (fade_us > 0).then_some(fade_us as f64 / 1_000_000.0)
}

fn build_audio_segment_filter(
    index: usize,
    segment_count: usize,
    start_us: i64,
    end_us: i64,
    seam_fade_us: i64,
) -> String {
    let start_s = start_us as f64 / 1_000_000.0;
    let end_s = end_us as f64 / 1_000_000.0;
    let duration_s = ((end_us - start_us).max(0)) as f64 / 1_000_000.0;

    let mut filter = format!("[0:a]atrim=start={start_s:.6}:end={end_s:.6},asetpts=PTS-STARTPTS");

    let fade_in_us = if index == 0 && start_us > 0 {
        FIRST_BOUNDARY_FADE_US
    } else if index > 0 {
        seam_fade_us
    } else {
        0
    };
    if let Some(fade_in_s) = seam_fade_duration_seconds(start_us, end_us, fade_in_us) {
        filter.push_str(&format!(",afade=t=in:st=0:d={fade_in_s:.6}"));
    }

    if index + 1 < segment_count {
        if let Some(fade_out_s) = seam_fade_duration_seconds(start_us, end_us, seam_fade_us) {
            let fade_out_start_s = (duration_s - fade_out_s).max(0.0);
            filter.push_str(&format!(
                ",afade=t=out:st={fade_out_start_s:.6}:d={fade_out_s:.6}"
            ));
        }
    }

    filter.push_str(&format!("[a{index}]"));
    filter
}

fn build_audio_concat_filter(segments: &[(i64, i64)]) -> String {
    build_audio_concat_filter_with_fade(segments, EXPORT_SEAM_FADE_US)
}

fn build_audio_concat_filter_with_fade(segments: &[(i64, i64)], seam_fade_us: i64) -> String {
    let mut filter_parts = Vec::new();
    let n = segments.len();
    for (i, (start, end)) in segments.iter().enumerate() {
        filter_parts.push(build_audio_segment_filter(i, n, *start, *end, seam_fade_us));
    }
    let a_inputs: String = (0..n).map(|i| format!("[a{i}]")).collect();
    filter_parts.push(format!("{a_inputs}concat=n={n}:v=0:a=1[outa]"));
    filter_parts.join("; ")
}

/// Canonical keep-segments for preview/export paths.
///
/// Uses the timing contract snapshot as the source of truth and normalizes
/// bounds/order so preview and export consume identical segment semantics.
fn settings_experimental_simplify_mode_enabled(app: &AppHandle) -> bool {
    crate::settings::get_settings(app).experimental_simplify_mode
}

fn contract_keep_segments_for_media(snapshot: &TimingContractSnapshot) -> Vec<(i64, i64)> {
    if snapshot.keep_segments_valid {
        snapshot
            .keep_segments
            .iter()
            .map(|seg| (seg.start_us, seg.end_us))
            .collect()
    } else {
        warn!(
            "Timing contract invalid (revision {}): {}. Falling back to quantized segments for media paths.",
            snapshot.timeline_revision,
            snapshot.warning.as_deref().unwrap_or("unknown warning")
        );
        snapshot
            .quantized_keep_segments
            .iter()
            .map(|seg| (seg.start_us, seg.end_us))
            .collect()
    }
}

fn select_raw_keep_segments_for_media(
    snapshot: &TimingContractSnapshot,
    legacy_segments: &[(i64, i64)],
    experimental_simplify_mode: bool,
) -> Vec<(i64, i64)> {
    let mut raw = contract_keep_segments_for_media(snapshot);
    if raw.is_empty() && !experimental_simplify_mode {
        raw = legacy_segments.to_vec();
    }

    raw
}

fn canonical_keep_segments_for_media(
    state: &EditorState,
    experimental_simplify_mode: bool,
) -> Vec<(i64, i64)> {
    let snapshot = state.timing_contract_snapshot();
    let legacy_segments = state.get_keep_segments();
    let mut raw =
        select_raw_keep_segments_for_media(&snapshot, &legacy_segments, experimental_simplify_mode);

    if raw.is_empty() {
        if experimental_simplify_mode {
            debug!(
                "Experimental simplify mode kept contract-only segment selection at revision {} (no legacy fallback segments available)",
                snapshot.timeline_revision
            );
        }
        return raw;
    }

    raw.sort_by_key(|(start_us, _)| *start_us);

    let source_start = snapshot.source_start_us.max(0);
    let source_end = snapshot.source_end_us.max(source_start);
    let mut cursor = source_start;
    let mut normalized = Vec::with_capacity(raw.len());

    for (start_us, end_us) in raw {
        let clamped_start = start_us.clamp(cursor, source_end);
        let clamped_end = end_us.clamp(clamped_start, source_end);
        if clamped_end > clamped_start {
            normalized.push((clamped_start, clamped_end));
            cursor = clamped_end;
        }
    }

    // Trim outer boundaries to reduce leading/trailing silence padding
    // from ASR word timestamps (Parakeet includes significant pre-speech padding).
    // Use aggressive trim (up to 50% of first/last segment, capped at 300ms)
    // since the outer edges are most likely to have dead air.
    const MAX_OUTER_TRIM_US: i64 = 300_000; // cap at 300ms
    if !normalized.is_empty() {
        let first = &mut normalized[0];
        let seg_dur = first.1 - first.0;
        let trim = (seg_dur / 2).min(MAX_OUTER_TRIM_US);
        first.0 += trim;

        let last = normalized.last_mut().unwrap();
        let seg_dur = last.1 - last.0;
        let trim = (seg_dur / 2).min(MAX_OUTER_TRIM_US);
        last.1 -= trim;
    }

    normalized
}

fn map_edit_time_to_source_time_from_segments(edit_time_us: i64, segments: &[(i64, i64)]) -> i64 {
    let mut elapsed: i64 = 0;

    for (start, end) in segments {
        let duration = end - start;
        if elapsed + duration > edit_time_us {
            return start + (edit_time_us - elapsed);
        }
        elapsed += duration;
    }

    segments.last().map_or(0, |&(_, end)| end)
}

fn build_preview_render_args(
    input_path: &Path,
    output_path: &Path,
    segments: &[(i64, i64)],
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string_lossy().to_string(),
        "-vn".to_string(),
    ];

    let filter = build_audio_concat_filter_with_fade(segments, PREVIEW_SEAM_FADE_US);
    args.extend([
        "-filter_complex".to_string(),
        filter,
        "-map".to_string(),
        "[outa]".to_string(),
    ]);

    args.extend([
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        "160k".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        output_path.to_string_lossy().to_string(),
    ]);

    args
}

fn extend_single_segment_export_args(
    args: &mut Vec<String>,
    start_us: i64,
    end_us: i64,
    has_video: bool,
) {
    let start_s = start_us as f64 / 1_000_000.0;
    let end_s = end_us as f64 / 1_000_000.0;
    args.extend([
        "-ss".to_string(),
        format!("{start_s:.6}"),
        "-to".to_string(),
        format!("{end_s:.6}"),
    ]);

    // Re-encode video for timing-accurate cuts (stream copy can drift to keyframe boundaries).
    if has_video {
        args.extend(["-c:v".to_string(), "libx264".to_string()]);
    }

    args.extend([
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        "192k".to_string(),
    ]);
}

/// Build the ASS force_style string for FFmpeg subtitle burn-in.
///
/// FFmpeg ASS uses `&HAABBGGRR&` color format (AA = alpha inverted, BGR order).
/// Alpha: 00 = fully opaque, FF = fully transparent (inverted from CSS convention).
/// BorderStyle=3 means OutlineColour controls the opaque box fill, not BackColour.
fn build_caption_style(
    text_color: &str,
    bg_color: &str,
    font_size: u32,
    position: u32,
    video_height: u32,
) -> String {
    let hex = text_color.trim_start_matches('#');
    let r = hex.get(0..2).unwrap_or("FF");
    let g = hex.get(2..4).unwrap_or("FF");
    let b = hex.get(4..6).unwrap_or("FF");
    let primary = format!("&H00{b}{g}{r}&");

    let bg_hex = bg_color.trim_start_matches('#');
    let br = bg_hex.get(0..2).unwrap_or("00");
    let bg = bg_hex.get(2..4).unwrap_or("00");
    let bb = bg_hex.get(4..6).unwrap_or("00");
    let css_alpha = u8::from_str_radix(bg_hex.get(6..8).unwrap_or("B3"), 16).unwrap_or(0xB3);
    let ass_alpha = 255 - css_alpha;
    let back = format!("&H{ass_alpha:02X}{bb}{bg}{br}&");

    let margin_v = ((100 - position) as f32 / 100.0 * video_height as f32) as u32;

    format!(
        "FontSize={},PrimaryColour={},OutlineColour={},BackColour=&H80000000&,BorderStyle=3,Outline=4,Shadow=0,Alignment=2,MarginV={}",
        font_size, primary, back, margin_v,
    )
}

#[allow(clippy::too_many_arguments)] // ffmpeg arg builder — each parameter is semantically distinct.
fn build_export_args(
    input_path: &str,
    output_path: &str,
    segments: &[(i64, i64)],
    has_video: bool,
    audio_opts: &ExportAudioOptions,
    srt_path: Option<&str>,
    caption_style: &str,
    video_size: Option<(u32, u32)>,
) -> Vec<String> {
    let mut args: Vec<String> = vec!["-y".to_string(), "-i".to_string(), input_path.to_string()];

    let total_duration_s: f64 = segments
        .iter()
        .map(|(s, e)| (e - s).max(0) as f64 / 1_000_000.0)
        .sum();

    if segments.len() == 1 {
        // Single segment — simple trim with re-encode for sample-accurate cuts
        let (start, end) = segments[0];
        extend_single_segment_export_args(&mut args, start, end, has_video);
        if let Some(post_filter) = build_audio_post_filters(audio_opts, total_duration_s) {
            args.extend(["-af".to_string(), post_filter]);
        }
        // Burn-in subtitles via -vf for single-segment video exports
        if has_video {
            if let Some(srt) = srt_path {
                let escaped = escape_srt_path_for_ffmpeg(srt);
                let size_param = match video_size {
                    Some((w, h)) => format!(":original_size={w}x{h}"),
                    None => String::new(),
                };
                args.extend([
                    "-vf".to_string(),
                    format!("subtitles='{escaped}':force_style='{caption_style}'{size_param}"),
                ]);
            }
        }
    } else {
        // Multiple segments — filter_complex with trim/atrim + concat
        let post_filters = build_audio_post_filters(audio_opts, total_duration_s);

        if has_video {
            let mut filter_parts = Vec::new();
            let n = segments.len();
            for (i, (start, end)) in segments.iter().enumerate() {
                let start_s = *start as f64 / 1_000_000.0;
                let end_s = *end as f64 / 1_000_000.0;
                filter_parts.push(format!(
                    "[0:v]trim=start={start_s:.6}:end={end_s:.6},setpts=PTS-STARTPTS[v{i}]"
                ));
                filter_parts.push(build_audio_segment_filter(
                    i,
                    n,
                    *start,
                    *end,
                    EXPORT_SEAM_FADE_US,
                ));
            }
            let v_inputs: String = (0..n).map(|i| format!("[v{i}]")).collect();
            let a_inputs: String = (0..n).map(|i| format!("[a{i}]")).collect();
            if let Some(ref pf) = post_filters {
                filter_parts.push(format!(
                    "{v_inputs}concat=n={n}:v=1:a=0[outv]; {a_inputs}concat=n={n}:v=0:a=1[outa_raw]; [outa_raw]{pf}[outa]"
                ));
            } else {
                filter_parts.push(format!(
                    "{v_inputs}concat=n={n}:v=1:a=0[outv]; {a_inputs}concat=n={n}:v=0:a=1[outa]"
                ));
            }

            // Burn-in subtitles: chain after [outv] in filter_complex
            let video_map_label = if let Some(srt) = srt_path {
                let escaped = escape_srt_path_for_ffmpeg(srt);
                let size_param = match video_size {
                    Some((w, h)) => format!(":original_size={w}x{h}"),
                    None => String::new(),
                };
                filter_parts.push(format!(
                    "[outv]subtitles='{escaped}':force_style='{caption_style}'{size_param}[outvs]"
                ));
                "[outvs]"
            } else {
                "[outv]"
            };

            let filter = filter_parts.join("; ");
            args.extend([
                "-filter_complex".to_string(),
                filter,
                "-map".to_string(),
                video_map_label.to_string(),
                "-map".to_string(),
                "[outa]".to_string(),
            ]);
        } else {
            let mut filter = build_audio_concat_filter(segments);
            if let Some(ref pf) = post_filters {
                filter = filter.replace("[outa]", "[outa_raw]");
                filter.push_str(&format!("; [outa_raw]{pf}[outa]"));
            }
            args.extend([
                "-filter_complex".to_string(),
                filter,
                "-map".to_string(),
                "[outa]".to_string(),
            ]);
        }
    }

    args.push(output_path.to_string());
    args
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

/// Returns the preview cache directory path.
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
                source != active_source || edit != active_edit
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
pub fn get_keep_segments(
    app: AppHandle,
    store: State<EditorStore>,
) -> Result<Vec<KeepSegment>, String> {
    let experimental_simplify_mode = settings_experimental_simplify_mode_enabled(&app);
    let state = store.0.lock().unwrap();
    let segments = canonical_keep_segments_for_media(&state, experimental_simplify_mode)
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
    app: AppHandle,
    store: State<EditorStore>,
    input_path: String,
) -> Result<String, String> {
    let experimental_simplify_mode = settings_experimental_simplify_mode_enabled(&app);
    let state = store.0.lock().unwrap();
    let segments = canonical_keep_segments_for_media(&state, experimental_simplify_mode);

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
    app: AppHandle,
    store: State<EditorStore>,
    edit_time_us: i64,
) -> Result<i64, String> {
    let experimental_simplify_mode = settings_experimental_simplify_mode_enabled(&app);
    let state = store.0.lock().unwrap();
    if experimental_simplify_mode {
        let segments = canonical_keep_segments_for_media(&state, true);
        Ok(map_edit_time_to_source_time_from_segments(
            edit_time_us,
            &segments,
        ))
    } else {
        Ok(state.map_edit_time_to_source_time(edit_time_us))
    }
}

/// Render (or reuse) a temporary preview audio artifact for the current edit state.
#[tauri::command]
#[specta::specta]
pub async fn render_temp_preview_audio(
    app: AppHandle,
    store: State<'_, EditorStore>,
    media_store: State<'_, MediaStore>,
) -> Result<PreviewRenderMetadata, String> {
    let experimental_simplify_mode = settings_experimental_simplify_mode_enabled(&app);
    let render_started_at = Instant::now();
    let segments = {
        let state = store.0.lock().unwrap();
        canonical_keep_segments_for_media(&state, experimental_simplify_mode)
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
        let args = build_preview_render_args(&media.path, &output_path, &segments);

        let render_result = tokio::time::timeout(
            PREVIEW_RENDER_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                std::process::Command::new("ffmpeg").args(&args).output()
            }),
        )
        .await;

        let output = match render_result {
            Ok(join_result) => join_result
                .map_err(|e| format!("Preview render task panicked: {}", e))?
                .map_err(|e| {
                    format!(
                        "FFmpeg not found. Install FFmpeg to render preview audio. Error: {}",
                        e
                    )
                })?,
            Err(_) => {
                return Err(format!(
                    "Preview render timed out after {} minutes. The media file may be too large.",
                    PREVIEW_RENDER_TIMEOUT.as_secs() / 60
                ));
            }
        };

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
    app: AppHandle,
    store: State<'_, EditorStore>,
    input_path: String,
    output_path: String,
    burn_captions: Option<bool>,
) -> Result<String, String> {
    let experimental_simplify_mode = settings_experimental_simplify_mode_enabled(&app);
    let (segments, words) = {
        let state = store.0.lock().unwrap();
        let segs = canonical_keep_segments_for_media(&state, experimental_simplify_mode);
        let w = state.get_words().to_vec();
        (segs, w)
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

    // Generate temp SRT for burn-in captions when requested on video exports
    let srt_temp_path = if burn_captions.unwrap_or(false) && has_video {
        let config = crate::managers::export::ExportConfig::default();
        let srt_content =
            crate::managers::export::export_srt_for_edited_timeline(&words, &segments, &config);
        let srt_file = std::path::Path::new(&output_path).with_extension("burn_captions.srt");
        std::fs::write(&srt_file, &srt_content)
            .map_err(|e| format!("Failed to write temp SRT: {}", e))?;
        Some(srt_file)
    } else {
        None
    };

    let settings = crate::settings::get_settings(&app);
    let audio_opts = ExportAudioOptions {
        normalize_audio: settings.normalize_audio_on_export,
        volume_db: settings.export_volume_db.clamp(-60.0, 24.0),
        fade_in_ms: settings.export_fade_in_ms.min(30_000),
        fade_out_ms: settings.export_fade_out_ms.min(30_000),
    };

    let caption_position = settings.caption_position.clamp(0, 100);
    let caption_font_size = settings.caption_font_size.clamp(8, 120);
    let caption_text_color = if is_valid_hex_color(&settings.caption_text_color) {
        &settings.caption_text_color
    } else {
        "#FFFFFF"
    };
    let caption_bg_color = if is_valid_hex_color(&settings.caption_bg_color) {
        &settings.caption_bg_color
    } else {
        "#000000B3"
    };

    let video_dims = probe_video_dimensions(input.to_str().unwrap_or(""));
    let video_height = match video_dims {
        Some((_, h)) => h,
        None => {
            log::warn!("Could not probe video dimensions for caption positioning, assuming 720p");
            720
        }
    };
    let caption_style = build_caption_style(
        caption_text_color,
        caption_bg_color,
        caption_font_size,
        caption_position,
        video_height,
    );

    let srt_path_str = srt_temp_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());
    let args = build_export_args(
        &input_path,
        &output_path,
        &segments,
        has_video,
        &audio_opts,
        srt_path_str.as_deref(),
        &caption_style,
        video_dims,
    );

    log::info!("Running FFmpeg export: ffmpeg {}", args.join(" "));

    let export_result = tokio::time::timeout(
        EXPORT_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("ffmpeg").args(&args).output()
        }),
    )
    .await;

    // Clean up temp SRT regardless of export outcome
    if let Some(ref srt_file) = srt_temp_path {
        let _ = std::fs::remove_file(srt_file);
    }

    let output = match export_result {
        Ok(join_result) => join_result
            .map_err(|e| format!("Export task panicked: {}", e))?
            .map_err(|e| {
                format!(
                    "FFmpeg not found. Install FFmpeg to export edited media. Error: {}",
                    e
                )
            })?,
        Err(_) => {
            return Err(format!(
                "Media export timed out after {} minutes. The media file may be too large.",
                EXPORT_TIMEOUT.as_secs() / 60
            ));
        }
    };

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
    use crate::managers::editor::{EditorState, TimingSegment, Word};
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use transcribe_rs::{
        whisper_cpp::{WhisperEngine, WhisperInferenceParams},
        TranscriptionSegment,
    };

    #[derive(Debug, serde::Serialize)]
    struct LiveValidationCriteria {
        preview_duration_tolerance_us: i64,
        export_duration_tolerance_us: i64,
        seam_max_ratio: f32,
        boundary_metric_note: String,
        asr_metric_note: String,
    }

    #[derive(Debug, serde::Serialize)]
    struct LiveValidationReport {
        media_path: String,
        preview_output_path: String,
        export_output_path: String,
        criteria: LiveValidationCriteria,
        keep_segments: Vec<(i64, i64)>,
        expected_keep_duration_us: i64,
        preview_duration_us: i64,
        export_duration_us: i64,
        preview_duration_error_us: i64,
        export_duration_error_us: i64,
        seam_discontinuity_ratios: Vec<f32>,
        duration_metric_pass: bool,
        boundary_metric_pass: bool,
        seam_metric_pass: bool,
        asr_metric_pass: bool,
        asr_leakage_oracle: AsrLeakageOracleReport,
        failure_reasons: Vec<String>,
        overall_pass: bool,
    }

    #[derive(Debug, serde::Serialize)]
    struct AsrLeakageOracleReport {
        enabled: bool,
        model_id: Option<String>,
        deleted_ranges_us: Vec<(i64, i64)>,
        deleted_phrases: Vec<String>,
        preview_leaked_deleted_phrases: Vec<String>,
        export_leaked_deleted_phrases: Vec<String>,
        preview_transcript_excerpt: Option<String>,
        export_transcript_excerpt: Option<String>,
        pass: bool,
        error: Option<String>,
    }

    fn abs_diff_i64(a: i64, b: i64) -> i64 {
        (a - b).abs()
    }

    fn default_live_media_path() -> String {
        r"C:\Users\alexm\Downloads\AddReleaseItem.mp4".to_string()
    }

    fn ffprobe_duration_us(path: &Path) -> Result<i64, String> {
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                &path.to_string_lossy(),
            ])
            .output()
            .map_err(|e| format!("ffprobe failed to start: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "ffprobe duration probe failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let seconds: f64 = raw
            .parse()
            .map_err(|e| format!("failed to parse ffprobe duration '{raw}': {e}"))?;
        Ok((seconds * 1_000_000.0).round() as i64)
    }

    fn run_ffmpeg(args: &[String]) -> Result<(), String> {
        let output = Command::new("ffmpeg")
            .args(args)
            .output()
            .map_err(|e| format!("ffmpeg failed to start: {e}"))?;

        if output.status.success() {
            return Ok(());
        }

        Err(format!(
            "ffmpeg command failed: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }

    fn deterministic_segments(source_duration_us: i64) -> Vec<(i64, i64)> {
        let mut segments = Vec::new();
        let min_segment_us = 800_000_i64;
        let max_end = source_duration_us.saturating_sub(100_000);
        let mut cursor = (source_duration_us as f64 * 0.10) as i64;
        let segment_len = ((source_duration_us as f64 * 0.16) as i64).max(min_segment_us);
        let gap_len = ((source_duration_us as f64 * 0.08) as i64).max(400_000);

        for _ in 0..3 {
            if cursor >= max_end {
                break;
            }
            let start = cursor.max(0);
            let end = (start + segment_len).min(max_end);
            if end - start >= min_segment_us {
                segments.push((start, end));
            }
            cursor = end + gap_len;
        }

        segments
    }

    fn seam_boundaries_edit_time_us(segments: &[(i64, i64)]) -> Vec<i64> {
        let mut boundaries = Vec::new();
        let mut elapsed = 0_i64;
        for (idx, (start, end)) in segments.iter().enumerate() {
            elapsed += end - start;
            if idx + 1 < segments.len() {
                boundaries.push(elapsed);
            }
        }
        boundaries
    }

    fn decode_pcm_window(
        path: &Path,
        center_s: f64,
        window_s: f64,
    ) -> Result<(Vec<f32>, usize), String> {
        let sample_rate = 48_000.0_f64;
        let start_s = (center_s - window_s / 2.0).max(0.0);
        let boundary_index = ((center_s - start_s) * sample_rate).round() as usize;
        let output = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-ss",
                &format!("{start_s:.6}"),
                "-t",
                &format!("{window_s:.6}"),
                "-i",
                &path.to_string_lossy(),
                "-vn",
                "-ac",
                "1",
                "-ar",
                "48000",
                "-f",
                "f32le",
                "pipe:1",
            ])
            .output()
            .map_err(|e| format!("ffmpeg pcm decode failed to start: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "ffmpeg pcm decode failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let mut samples = Vec::with_capacity(output.stdout.len() / 4);
        for chunk in output.stdout.chunks_exact(4) {
            samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }

        Ok((samples, boundary_index))
    }

    fn median(values: &mut [f32]) -> f32 {
        if values.is_empty() {
            return 0.0;
        }
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = values.len() / 2;
        if values.len() % 2 == 0 {
            (values[mid - 1] + values[mid]) / 2.0
        } else {
            values[mid]
        }
    }

    fn seam_discontinuity_ratio(samples: &[f32], boundary_index: usize) -> f32 {
        if samples.len() < 10 || boundary_index == 0 || boundary_index >= samples.len() {
            return 0.0;
        }

        let boundary_step = (samples[boundary_index] - samples[boundary_index - 1]).abs();
        let start = boundary_index.saturating_sub(240).max(1);
        let end = (boundary_index + 240).min(samples.len() - 1);

        let mut reference_steps = Vec::new();
        for i in start..=end {
            if i >= boundary_index.saturating_sub(2) && i <= boundary_index + 2 {
                continue;
            }
            reference_steps.push((samples[i] - samples[i - 1]).abs());
        }

        let baseline = median(&mut reference_steps).max(1e-6);
        boundary_step / baseline
    }

    fn decode_audio_for_local_asr(path: &Path) -> Result<Vec<f32>, String> {
        let output = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-i",
                &path.to_string_lossy(),
                "-vn",
                "-ac",
                "1",
                "-ar",
                "16000",
                "-f",
                "f32le",
                "pipe:1",
            ])
            .output()
            .map_err(|e| format!("ffmpeg ASR decode failed to start: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "ffmpeg ASR decode failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let mut samples = Vec::with_capacity(output.stdout.len() / 4);
        for chunk in output.stdout.chunks_exact(4) {
            samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        Ok(samples)
    }

    fn normalize_asr_text(text: &str) -> String {
        let mut normalized = String::with_capacity(text.len());
        for ch in text.chars() {
            if ch.is_alphanumeric() {
                normalized.extend(ch.to_lowercase());
            } else {
                normalized.push(' ');
            }
        }
        normalized.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn transcript_excerpt(text: &str) -> Option<String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut excerpt = String::new();
        for ch in trimmed.chars().take(220) {
            excerpt.push(ch);
        }
        if trimmed.chars().count() > 220 {
            excerpt.push_str("...");
        }
        Some(excerpt)
    }

    fn deleted_ranges_from_keep_segments(
        keep_segments: &[(i64, i64)],
        source_duration_us: i64,
    ) -> Vec<(i64, i64)> {
        let mut deleted = Vec::new();
        let mut cursor = 0_i64;
        for (start, end) in keep_segments {
            if *start > cursor {
                deleted.push((cursor, *start));
            }
            cursor = cursor.max(*end);
        }
        if cursor < source_duration_us {
            deleted.push((cursor, source_duration_us));
        }
        deleted
    }

    fn segment_deleted_overlap_ratio(
        segment_start_us: i64,
        segment_end_us: i64,
        deleted_ranges: &[(i64, i64)],
    ) -> f64 {
        let duration = (segment_end_us - segment_start_us).max(1);
        let overlap_us: i64 = deleted_ranges
            .iter()
            .map(|(del_start, del_end)| {
                let overlap_start = segment_start_us.max(*del_start);
                let overlap_end = segment_end_us.min(*del_end);
                (overlap_end - overlap_start).max(0)
            })
            .sum();
        overlap_us as f64 / duration as f64
    }

    fn collect_deleted_phrases_from_source_segments(
        source_segments: &[TranscriptionSegment],
        deleted_ranges: &[(i64, i64)],
    ) -> Vec<String> {
        let mut phrases = BTreeSet::new();
        for segment in source_segments {
            let start_us = (segment.start * 1_000_000.0).round() as i64;
            let end_us = (segment.end * 1_000_000.0).round() as i64;
            if end_us <= start_us {
                continue;
            }
            let overlap_ratio = segment_deleted_overlap_ratio(start_us, end_us, deleted_ranges);
            if overlap_ratio < 0.35 {
                continue;
            }
            let normalized = normalize_asr_text(&segment.text);
            if normalized.split_whitespace().count() >= 2 {
                phrases.insert(normalized);
            }
        }
        phrases.into_iter().collect()
    }

    fn transcript_contains_phrase(transcript: &str, phrase: &str) -> bool {
        if phrase.is_empty() {
            return false;
        }
        let transcript_tokens: Vec<&str> = transcript.split_whitespace().collect();
        let phrase_tokens: Vec<&str> = phrase.split_whitespace().collect();
        if phrase_tokens.is_empty() || phrase_tokens.len() > transcript_tokens.len() {
            return false;
        }
        transcript_tokens
            .windows(phrase_tokens.len())
            .any(|window| window == phrase_tokens.as_slice())
    }

    fn leaked_deleted_phrases(deleted_phrases: &[String], transcript: &str) -> Vec<String> {
        deleted_phrases
            .iter()
            .filter(|phrase| transcript_contains_phrase(transcript, phrase))
            .cloned()
            .collect()
    }

    fn transcribe_media_with_local_whisper(
        whisper_engine: &mut WhisperEngine,
        media_path: &Path,
    ) -> Result<(String, Option<Vec<TranscriptionSegment>>), String> {
        let samples = decode_audio_for_local_asr(media_path)?;
        if samples.is_empty() {
            return Err(format!(
                "decoded audio is empty for ASR oracle: {}",
                media_path.display()
            ));
        }

        let params = WhisperInferenceParams {
            language: None,
            translate: false,
            initial_prompt: None,
            ..Default::default()
        };

        let result = whisper_engine
            .transcribe_with(&samples, &params)
            .map_err(|e| format!("local ASR transcription failed: {e}"))?;
        Ok((result.text, result.segments))
    }

    fn resolve_live_asr_model_path() -> Result<PathBuf, String> {
        let path = std::env::var("TOASTER_LIVE_ASR_MODEL_PATH").map_err(|_| {
            "ASR oracle requires TOASTER_LIVE_ASR_MODEL_PATH to point to a local Whisper model file"
                .to_string()
        })?;
        let model_path = PathBuf::from(path);
        if !model_path.exists() {
            return Err(format!(
                "TOASTER_LIVE_ASR_MODEL_PATH not found: {}",
                model_path.display()
            ));
        }
        if !model_path.is_file() {
            return Err(format!(
                "TOASTER_LIVE_ASR_MODEL_PATH must be a model file: {}",
                model_path.display()
            ));
        }
        Ok(model_path)
    }

    fn run_asr_leakage_oracle(
        source_media_path: &Path,
        preview_path: &Path,
        export_path: &Path,
        keep_segments: &[(i64, i64)],
        source_duration_us: i64,
    ) -> AsrLeakageOracleReport {
        let mut report = AsrLeakageOracleReport {
            enabled: true,
            model_id: None,
            deleted_ranges_us: deleted_ranges_from_keep_segments(keep_segments, source_duration_us),
            deleted_phrases: Vec::new(),
            preview_leaked_deleted_phrases: Vec::new(),
            export_leaked_deleted_phrases: Vec::new(),
            preview_transcript_excerpt: None,
            export_transcript_excerpt: None,
            pass: false,
            error: None,
        };

        let model_path = match resolve_live_asr_model_path() {
            Ok(path) => path,
            Err(error) => {
                report.error = Some(error);
                return report;
            }
        };
        report.model_id = model_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_string);

        let mut whisper_engine = match WhisperEngine::load(&model_path) {
            Ok(engine) => engine,
            Err(error) => {
                report.error = Some(format!(
                    "failed to initialize local Whisper engine from {}: {error}",
                    model_path.display()
                ));
                return report;
            }
        };

        let (source_text, source_segments) =
            match transcribe_media_with_local_whisper(&mut whisper_engine, source_media_path) {
                Ok(result) => result,
                Err(error) => {
                    report.error = Some(format!("source transcription failed: {error}"));
                    return report;
                }
            };
        let source_segments = match source_segments {
            Some(segments) if !segments.is_empty() => segments,
            _ => {
                report.error = Some(
                    "ASR oracle requires timestamped source segments; current model returned none"
                        .to_string(),
                );
                return report;
            }
        };

        let deleted_phrases = collect_deleted_phrases_from_source_segments(
            &source_segments,
            &report.deleted_ranges_us,
        );
        if deleted_phrases.is_empty() {
            report.error = Some(
                "ASR oracle could not extract deleted phrases from source transcription segments"
                    .to_string(),
            );
            return report;
        }
        report.deleted_phrases = deleted_phrases;

        let (preview_text, _) =
            match transcribe_media_with_local_whisper(&mut whisper_engine, preview_path) {
                Ok(result) => result,
                Err(error) => {
                    report.error = Some(format!("preview transcription failed: {error}"));
                    return report;
                }
            };
        let (export_text, _) =
            match transcribe_media_with_local_whisper(&mut whisper_engine, export_path) {
                Ok(result) => result,
                Err(error) => {
                    report.error = Some(format!("export transcription failed: {error}"));
                    return report;
                }
            };

        let source_normalized = normalize_asr_text(&source_text);
        let preview_normalized = normalize_asr_text(&preview_text);
        let export_normalized = normalize_asr_text(&export_text);

        report.preview_transcript_excerpt = transcript_excerpt(&preview_normalized);
        report.export_transcript_excerpt = transcript_excerpt(&export_normalized);

        report.preview_leaked_deleted_phrases =
            leaked_deleted_phrases(&report.deleted_phrases, &preview_normalized);
        report.export_leaked_deleted_phrases =
            leaked_deleted_phrases(&report.deleted_phrases, &export_normalized);

        // If the source transcription has text but no deleted phrase overlaps were recovered,
        // fail explicitly so leakage checks never silently pass.
        if !source_normalized.is_empty() && report.deleted_phrases.is_empty() {
            report.error = Some(
                "ASR oracle did not recover a deleted phrase set from source transcription"
                    .to_string(),
            );
            return report;
        }

        report.pass = report.preview_leaked_deleted_phrases.is_empty()
            && report.export_leaked_deleted_phrases.is_empty();
        report
    }

    fn collect_live_validation_failure_reasons(
        preview_duration_error_us: i64,
        export_duration_error_us: i64,
        preview_duration_tolerance_us: i64,
        export_duration_tolerance_us: i64,
        boundary_metric_pass: bool,
        seam_metric_pass: bool,
        seam_ratios: &[f32],
        seam_max_ratio: f32,
        asr_leakage_oracle: &AsrLeakageOracleReport,
    ) -> Vec<String> {
        let mut reasons = Vec::new();
        if preview_duration_error_us > preview_duration_tolerance_us {
            reasons.push(format!(
                "preview duration drift exceeded tolerance: {}us > {}us",
                preview_duration_error_us, preview_duration_tolerance_us
            ));
        }
        if export_duration_error_us > export_duration_tolerance_us {
            reasons.push(format!(
                "export duration drift exceeded tolerance: {}us > {}us",
                export_duration_error_us, export_duration_tolerance_us
            ));
        }
        if !boundary_metric_pass {
            reasons.push(
                "boundary metric failed: at least one keep-segment boundary token was missing from ffmpeg trim commands"
                    .to_string(),
            );
        }
        if !seam_metric_pass {
            let observed_max = seam_ratios.iter().copied().fold(0.0_f32, f32::max);
            reasons.push(format!(
                "seam discontinuity exceeded max ratio: observed {:.4} > {:.4}",
                observed_max, seam_max_ratio
            ));
        }
        if !asr_leakage_oracle.pass {
            if let Some(error) = &asr_leakage_oracle.error {
                reasons.push(format!("ASR leakage oracle error: {error}"));
            }
            if !asr_leakage_oracle.preview_leaked_deleted_phrases.is_empty() {
                reasons.push(format!(
                    "preview leaked deleted phrases: {}",
                    asr_leakage_oracle.preview_leaked_deleted_phrases.join(", ")
                ));
            }
            if !asr_leakage_oracle.export_leaked_deleted_phrases.is_empty() {
                reasons.push(format!(
                    "export leaked deleted phrases: {}",
                    asr_leakage_oracle.export_leaked_deleted_phrases.join(", ")
                ));
            }
            if asr_leakage_oracle.error.is_none()
                && asr_leakage_oracle.preview_leaked_deleted_phrases.is_empty()
                && asr_leakage_oracle.export_leaked_deleted_phrases.is_empty()
            {
                reasons.push(
                    "ASR leakage oracle failed without explicit error or leaked phrase details"
                        .to_string(),
                );
            }
        }
        reasons
    }

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

    fn snapshot_with_segments(
        keep_segments_valid: bool,
        keep_segments: Vec<(i64, i64)>,
        quantized_keep_segments: Vec<(i64, i64)>,
    ) -> TimingContractSnapshot {
        let to_timing_segments = |segments: Vec<(i64, i64)>| {
            segments
                .into_iter()
                .map(|(start_us, end_us)| TimingSegment { start_us, end_us })
                .collect::<Vec<_>>()
        };

        TimingContractSnapshot {
            timeline_revision: 7,
            total_words: 0,
            deleted_words: 0,
            active_words: 0,
            source_start_us: 0,
            source_end_us: 3_000_000,
            total_keep_duration_us: 0,
            keep_segments: to_timing_segments(keep_segments),
            quantized_keep_segments: to_timing_segments(quantized_keep_segments),
            quantization_fps_num: 30,
            quantization_fps_den: 1,
            keep_segments_valid,
            warning: (!keep_segments_valid).then_some("contract invalid".to_string()),
        }
    }

    #[test]
    fn experimental_simplify_mode_skips_legacy_fallback_segments() {
        let snapshot = snapshot_with_segments(true, Vec::new(), Vec::new());
        let legacy = vec![(10, 20)];

        assert_eq!(
            select_raw_keep_segments_for_media(&snapshot, &legacy, false),
            legacy
        );
        assert!(select_raw_keep_segments_for_media(&snapshot, &legacy, true).is_empty());
    }

    #[test]
    fn experimental_simplify_mode_still_uses_quantized_segments_when_contract_invalid() {
        let snapshot = snapshot_with_segments(false, vec![(100, 300)], vec![(1_000, 2_000)]);
        let legacy = vec![(10, 20)];

        assert_eq!(
            select_raw_keep_segments_for_media(&snapshot, &legacy, false),
            vec![(1_000, 2_000)]
        );
        assert_eq!(
            select_raw_keep_segments_for_media(&snapshot, &legacy, true),
            vec![(1_000, 2_000)]
        );
    }

    #[test]
    fn canonical_keep_segments_match_valid_contract_segments() {
        let mut state = EditorState::new();
        state.set_words(vec![
            Word {
                text: "alpha".to_string(),
                start_us: 0,
                end_us: 1_000_000,
                deleted: false,
                silenced: false,
                confidence: 0.9,
                speaker_id: 0,
            },
            Word {
                text: "beta".to_string(),
                start_us: 1_000_000,
                end_us: 2_000_000,
                deleted: true,
                silenced: false,
                confidence: 0.9,
                speaker_id: 0,
            },
            Word {
                text: "gamma".to_string(),
                start_us: 2_000_000,
                end_us: 3_000_000,
                deleted: false,
                silenced: false,
                confidence: 0.9,
                speaker_id: 0,
            },
        ]);

        let segments = canonical_keep_segments_for_media(&state, false);
        // Outer boundary trim (50% capped at 300ms) adjusts first.start and last.end
        assert_eq!(segments, vec![(300_000, 1_000_000), (2_000_000, 2_700_000)]);
    }

    #[test]
    fn canonical_keep_segments_normalize_invalid_overlap_to_monotonic_ranges() {
        let mut state = EditorState::new();
        state.set_words(vec![
            Word {
                text: "alpha".to_string(),
                start_us: -500_000,
                end_us: 1_000_000,
                deleted: false,
                silenced: false,
                confidence: 0.8,
                speaker_id: 0,
            },
            Word {
                text: "beta".to_string(),
                start_us: 900_000,
                end_us: 1_500_000,
                deleted: false,
                silenced: false,
                confidence: 0.8,
                speaker_id: 0,
            },
        ]);

        let segments = canonical_keep_segments_for_media(&state, false);
        assert!(!segments.is_empty());
        assert!(segments
            .iter()
            .all(|(start_us, end_us)| *start_us >= 0 && end_us > start_us));
        assert!(segments.windows(2).all(|w| w[0].1 <= w[1].0));
    }

    #[test]
    fn audio_segment_filter_adds_micro_fades_at_joins() {
        let filter = build_audio_segment_filter(1, 3, 1_000_000, 2_000_000, 8_000);
        assert!(filter.contains("afade=t=in:st=0:d=0.008000"));
        assert!(filter.contains("afade=t=out:st=0.992000:d=0.008000"));
        assert!(filter.ends_with("[a1]"));
    }

    #[test]
    fn audio_segment_filter_scales_fade_for_short_segments() {
        let filter = build_audio_segment_filter(1, 3, 0, 6_000, 8_000);
        assert!(filter.contains("afade=t=in:st=0:d=0.003000"));
        assert!(filter.contains("afade=t=out:st=0.003000:d=0.003000"));
    }

    #[test]
    fn very_short_segment_fade_clamped_to_half_duration() {
        // 3ms segment (3000µs) with an 8000µs requested fade.
        // Each fade must be clamped to half the segment duration = 1500µs.
        let filter = build_audio_segment_filter(1, 3, 0, 3_000, 8_000);
        assert!(filter.contains("afade=t=in:st=0:d=0.001500"));
        assert!(filter.contains("afade=t=out:st=0.001500:d=0.001500"));
        // Fade must never exceed half the segment duration.
        let fade_d: f64 = 0.001500;
        let duration_s: f64 = 0.003;
        assert!(fade_d <= duration_s / 2.0);
    }

    #[test]
    fn ultra_short_segment_skips_fades_entirely() {
        // 50µs segment — shorter than MIN_FADEABLE_SEGMENT_US (100µs).
        let filter = build_audio_segment_filter(1, 3, 0, 50, 8_000);
        assert!(!filter.contains("afade="));
    }

    #[test]
    fn leading_deletion_segment_gets_first_boundary_fade_in() {
        let filter = build_audio_segment_filter(0, 1, 1_000_000, 2_000_000, 0);
        assert!(filter.contains("afade=t=in:st=0:d=0.002000"));
        assert!(!filter.contains("afade=t=out"));
    }

    #[test]
    fn concat_filter_without_fade_has_no_afade_nodes() {
        let filter =
            build_audio_concat_filter_with_fade(&[(0, 1_000_000), (2_000_000, 3_000_000)], 0);
        assert!(!filter.contains("afade="));
    }

    #[test]
    fn single_segment_preview_uses_filter_complex_trim_pipeline() {
        let input = Path::new("input.mp4");
        let output = Path::new("preview.m4a");
        let args = build_preview_render_args(input, output, &[(1_000_000, 2_500_000)]);

        assert!(args.windows(2).any(|w| w[0] == "-filter_complex"));
        assert!(args.windows(2).any(|w| w[0] == "-map" && w[1] == "[outa]"));
        assert!(!args.iter().any(|arg| arg == "-ss"));
        assert!(!args.iter().any(|arg| arg == "-to"));

        let filter = args
            .windows(2)
            .find(|w| w[0] == "-filter_complex")
            .map(|w| w[1].as_str())
            .expect("missing preview filter");
        assert_eq!(
            filter,
            "[0:a]atrim=start=1.000000:end=2.500000,asetpts=PTS-STARTPTS,afade=t=in:st=0:d=0.002000[a0]; [a0]concat=n=1:v=0:a=1[outa]"
        );
    }

    #[test]
    fn multi_segment_preview_uses_same_filter_complex_trim_pipeline() {
        let input = Path::new("input.mp4");
        let output = Path::new("preview.m4a");
        let segments = [(0, 1_000_000), (2_000_000, 3_500_000)];
        let args = build_preview_render_args(input, output, &segments);

        let filter = args
            .windows(2)
            .find(|w| w[0] == "-filter_complex")
            .map(|w| w[1].as_str())
            .expect("missing preview filter");

        assert_eq!(
            filter,
            "[0:a]atrim=start=0.000000:end=1.000000,asetpts=PTS-STARTPTS[a0]; [0:a]atrim=start=2.000000:end=3.500000,asetpts=PTS-STARTPTS[a1]; [a0][a1]concat=n=2:v=0:a=1[outa]"
        );
    }

    #[test]
    fn single_segment_video_export_reencodes_video() {
        let mut args = vec![];
        extend_single_segment_export_args(&mut args, 1_000_000, 2_500_000, true);
        assert!(args.windows(2).any(|w| w[0] == "-c:v" && w[1] == "libx264"));
        assert!(!args.iter().any(|arg| arg == "copy"));
    }

    #[test]
    fn single_segment_audio_only_export_omits_video_codec() {
        let mut args = vec![];
        extend_single_segment_export_args(&mut args, 1_000_000, 2_500_000, false);
        assert!(!args.iter().any(|arg| arg == "-c:v"));
        assert!(args.windows(2).any(|w| w[0] == "-c:a" && w[1] == "aac"));
    }

    #[test]
    fn deleted_ranges_are_complement_of_keep_segments() {
        let keep_segments = vec![(1_000_000, 2_000_000), (3_000_000, 4_000_000)];
        let deleted = deleted_ranges_from_keep_segments(&keep_segments, 5_000_000);
        assert_eq!(
            deleted,
            vec![
                (0, 1_000_000),
                (2_000_000, 3_000_000),
                (4_000_000, 5_000_000)
            ]
        );
    }

    #[test]
    fn collect_deleted_phrases_uses_deleted_overlap_threshold() {
        let source_segments = vec![
            TranscriptionSegment {
                start: 0.0,
                end: 1.0,
                text: "this is kept".to_string(),
            },
            TranscriptionSegment {
                start: 1.0,
                end: 2.0,
                text: "remove this phrase now".to_string(),
            },
            TranscriptionSegment {
                start: 2.0,
                end: 3.0,
                text: "also remove this line".to_string(),
            },
        ];
        let deleted_ranges = vec![(950_000, 2_800_000)];
        let deleted_phrases =
            collect_deleted_phrases_from_source_segments(&source_segments, &deleted_ranges);
        assert_eq!(
            deleted_phrases,
            vec![
                "also remove this line".to_string(),
                "remove this phrase now".to_string()
            ]
        );
    }

    #[test]
    fn leaked_deleted_phrases_detects_exact_token_sequences() {
        let deleted_phrases = vec![
            "remove this phrase now".to_string(),
            "red marker".to_string(),
            "do not leak".to_string(),
        ];
        let transcript = normalize_asr_text(
            "Intro text. We still hear REMOVE this phrase now and a red marker today.",
        );
        let leaks = leaked_deleted_phrases(&deleted_phrases, &transcript);
        assert_eq!(
            leaks,
            vec![
                "remove this phrase now".to_string(),
                "red marker".to_string()
            ]
        );
    }

    #[test]
    fn live_validation_failure_reasons_capture_multiple_metric_failures() {
        let asr_report = AsrLeakageOracleReport {
            enabled: true,
            model_id: Some("small".to_string()),
            deleted_ranges_us: vec![(0, 1_000_000)],
            deleted_phrases: vec!["remove this phrase now".to_string()],
            preview_leaked_deleted_phrases: vec!["remove this phrase now".to_string()],
            export_leaked_deleted_phrases: Vec::new(),
            preview_transcript_excerpt: Some("remove this phrase now".to_string()),
            export_transcript_excerpt: Some("kept transcript".to_string()),
            pass: false,
            error: Some("mock oracle failure".to_string()),
        };

        let reasons = collect_live_validation_failure_reasons(
            250_000,
            320_000,
            180_000,
            220_000,
            false,
            false,
            &[2.0, 24.0],
            20.0,
            &asr_report,
        );

        assert!(reasons
            .iter()
            .any(|reason| reason.contains("preview duration drift exceeded tolerance")));
        assert!(reasons
            .iter()
            .any(|reason| reason.contains("export duration drift exceeded tolerance")));
        assert!(reasons
            .iter()
            .any(|reason| reason.contains("boundary metric failed")));
        assert!(reasons
            .iter()
            .any(|reason| reason.contains("seam discontinuity exceeded max ratio")));
        assert!(reasons
            .iter()
            .any(|reason| reason.contains("ASR leakage oracle error")));
        assert!(reasons
            .iter()
            .any(|reason| reason.contains("preview leaked deleted phrases")));
    }

    #[test]
    fn live_validation_failure_reasons_empty_when_all_metrics_pass() {
        let asr_report = AsrLeakageOracleReport {
            enabled: true,
            model_id: Some("small".to_string()),
            deleted_ranges_us: vec![],
            deleted_phrases: vec![],
            preview_leaked_deleted_phrases: Vec::new(),
            export_leaked_deleted_phrases: Vec::new(),
            preview_transcript_excerpt: None,
            export_transcript_excerpt: None,
            pass: true,
            error: None,
        };

        let reasons = collect_live_validation_failure_reasons(
            100_000,
            150_000,
            180_000,
            220_000,
            true,
            true,
            &[0.4, 0.7],
            20.0,
            &asr_report,
        );

        assert!(reasons.is_empty());
    }

    #[test]
    #[ignore = "requires local media fixture; run via scripts/run-live-midstream-validation.ps1"]
    fn live_validation_backend_media_pipeline() {
        const PREVIEW_DURATION_TOLERANCE_US: i64 = 180_000;
        const EXPORT_DURATION_TOLERANCE_US: i64 = 220_000;
        const SEAM_MAX_RATIO: f32 = 20.0;

        let media_path =
            std::env::var("TOASTER_LIVE_MEDIA_PATH").unwrap_or_else(|_| default_live_media_path());
        let media = PathBuf::from(media_path.clone());
        assert!(
            media.exists(),
            "live validation media file not found: {}",
            media.display()
        );

        let source_duration_us =
            ffprobe_duration_us(&media).expect("failed to probe source media duration");
        assert!(
            source_duration_us > 5_000_000,
            "media is too short for live validation: {source_duration_us}us"
        );

        let segments = deterministic_segments(source_duration_us);
        assert!(
            segments.len() >= 2,
            "deterministic segment generation did not produce enough segments"
        );
        let expected_keep_duration_us: i64 = segments.iter().map(|(s, e)| e - s).sum();

        let output_root = std::env::var("TOASTER_LIVE_OUTPUT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("toaster-live-validation"));
        std::fs::create_dir_all(&output_root)
            .expect("failed to create live validation output directory");

        let preview_path = output_root.join("live-preview.m4a");
        let export_ext = media
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_else(|| "mp4".to_string());
        let export_path = output_root.join(format!("live-export.{export_ext}"));

        let preview_args = build_preview_render_args(&media, &preview_path, &segments);
        run_ffmpeg(&preview_args).expect("preview render ffmpeg failed");

        let has_video = matches!(
            export_ext.as_str(),
            "mp4" | "mkv" | "mov" | "avi" | "webm" | "flv"
        );
        let export_args = build_export_args(
            &media.to_string_lossy(),
            &export_path.to_string_lossy(),
            &segments,
            has_video,
            &ExportAudioOptions::default(),
            None,
            "",
            None,
        );
        run_ffmpeg(&export_args).expect("export render ffmpeg failed");

        let preview_duration_us =
            ffprobe_duration_us(&preview_path).expect("failed to probe preview duration");
        let export_duration_us =
            ffprobe_duration_us(&export_path).expect("failed to probe export duration");

        let preview_duration_error_us =
            abs_diff_i64(preview_duration_us, expected_keep_duration_us);
        let export_duration_error_us = abs_diff_i64(export_duration_us, expected_keep_duration_us);
        let duration_metric_pass = preview_duration_error_us <= PREVIEW_DURATION_TOLERANCE_US
            && export_duration_error_us <= EXPORT_DURATION_TOLERANCE_US;

        let preview_cmd = preview_args.join(" ");
        let export_cmd = export_args.join(" ");
        let boundary_metric_pass = segments.iter().all(|(start, end)| {
            let token = format!(
                "start={:.6}:end={:.6}",
                *start as f64 / 1_000_000.0,
                *end as f64 / 1_000_000.0
            );
            preview_cmd.contains(&token) && export_cmd.contains(&token)
        });

        let seam_boundaries = seam_boundaries_edit_time_us(&segments);
        let mut seam_ratios = Vec::new();
        for seam_us in seam_boundaries {
            let center_s = seam_us as f64 / 1_000_000.0;
            let (samples, boundary_index) =
                decode_pcm_window(&export_path, center_s, 0.024).expect("failed seam decode");
            seam_ratios.push(seam_discontinuity_ratio(&samples, boundary_index));
        }
        let seam_metric_pass = seam_ratios.iter().all(|ratio| *ratio <= SEAM_MAX_RATIO);

        let asr_leakage_oracle = run_asr_leakage_oracle(
            &media,
            &preview_path,
            &export_path,
            &segments,
            source_duration_us,
        );
        let asr_metric_pass = asr_leakage_oracle.pass;
        let failure_reasons = collect_live_validation_failure_reasons(
            preview_duration_error_us,
            export_duration_error_us,
            PREVIEW_DURATION_TOLERANCE_US,
            EXPORT_DURATION_TOLERANCE_US,
            boundary_metric_pass,
            seam_metric_pass,
            &seam_ratios,
            SEAM_MAX_RATIO,
            &asr_leakage_oracle,
        );

        let overall_pass =
            duration_metric_pass && boundary_metric_pass && seam_metric_pass && asr_metric_pass;

        let report = LiveValidationReport {
            media_path: media.to_string_lossy().to_string(),
            preview_output_path: preview_path.to_string_lossy().to_string(),
            export_output_path: export_path.to_string_lossy().to_string(),
            criteria: LiveValidationCriteria {
                preview_duration_tolerance_us: PREVIEW_DURATION_TOLERANCE_US,
                export_duration_tolerance_us: EXPORT_DURATION_TOLERANCE_US,
                seam_max_ratio: SEAM_MAX_RATIO,
                boundary_metric_note:
                    "every deterministic keep-segment start/end token must be present in both ffmpeg trim commands"
                        .to_string(),
                asr_metric_note:
                    "ASR oracle passes only when no deleted phrases appear in preview/export transcripts and no oracle error is reported"
                        .to_string(),
            },
            keep_segments: segments,
            expected_keep_duration_us,
            preview_duration_us,
            export_duration_us,
            preview_duration_error_us,
            export_duration_error_us,
            seam_discontinuity_ratios: seam_ratios,
            duration_metric_pass,
            boundary_metric_pass,
            seam_metric_pass,
            asr_metric_pass,
            asr_leakage_oracle,
            failure_reasons,
            overall_pass,
        };

        let report_path = output_root.join("live-validation-report.json");
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report).expect("failed to serialize report"),
        )
        .expect("failed to write live validation report");

        assert!(
            overall_pass,
            "live validation failed; report: {}",
            report_path.display()
        );
    }

    // ---- Caption style construction tests ----

    #[test]
    fn test_ass_primary_colour_white() {
        let style = build_caption_style("#FFFFFF", "#000000B3", 24, 90, 1080);
        // #FFFFFF → BGR is still FFFFFF, alpha 00 = opaque
        assert!(
            style.contains("PrimaryColour=&H00FFFFFF&"),
            "White text should be &H00FFFFFF&, got: {style}"
        );
    }

    #[test]
    fn test_ass_primary_colour_red() {
        let style = build_caption_style("#FF0000", "#000000B3", 24, 90, 1080);
        // #FF0000 → R=FF,G=00,B=00 → BGR=0000FF → &H000000FF&
        assert!(
            style.contains("PrimaryColour=&H000000FF&"),
            "Red text (#FF0000) should be &H000000FF& in BGR, got: {style}"
        );
    }

    #[test]
    fn test_ass_primary_colour_blue() {
        let style = build_caption_style("#0000FF", "#000000B3", 24, 90, 1080);
        // #0000FF → R=00,G=00,B=FF → BGR=FF0000 → &H00FF0000&
        assert!(
            style.contains("PrimaryColour=&H00FF0000&"),
            "Blue text (#0000FF) should be &H00FF0000& in BGR, got: {style}"
        );
    }

    #[test]
    fn test_ass_back_colour_with_alpha() {
        let style = build_caption_style("#FFFFFF", "#000000B3", 24, 90, 1080);
        // bg=#000000B3 → R=00,G=00,B=00, CSS alpha=B3=179
        // ASS alpha = 255-179 = 76 = 0x4C
        // OutlineColour=&H4C000000&
        assert!(
            style.contains("OutlineColour=&H4C000000&"),
            "BG #000000B3 should produce OutlineColour=&H4C000000&, got: {style}"
        );
    }

    #[test]
    fn test_ass_back_colour_fully_opaque() {
        let style = build_caption_style("#FFFFFF", "#000000FF", 24, 90, 1080);
        // CSS alpha FF=255 → ASS alpha = 255-255 = 0 = 0x00 (opaque)
        assert!(
            style.contains("OutlineColour=&H00000000&"),
            "Fully opaque BG should have ASS alpha 00, got: {style}"
        );
    }

    #[test]
    fn test_ass_margin_v_default_position() {
        let style = build_caption_style("#FFFFFF", "#000000B3", 24, 90, 1080);
        // MarginV = (100-90)/100 * 1080 = 108
        assert!(
            style.contains("MarginV=108"),
            "position=90 on 1080p should give MarginV=108, got: {style}"
        );
    }

    #[test]
    fn test_ass_margin_v_position_50() {
        let style = build_caption_style("#FFFFFF", "#000000B3", 24, 50, 1080);
        // MarginV = (100-50)/100 * 1080 = 540
        assert!(
            style.contains("MarginV=540"),
            "position=50 on 1080p should give MarginV=540, got: {style}"
        );
    }

    #[test]
    fn test_ass_margin_v_position_0() {
        let style = build_caption_style("#FFFFFF", "#000000B3", 24, 0, 1080);
        // MarginV = (100-0)/100 * 1080 = 1080
        assert!(
            style.contains("MarginV=1080"),
            "position=0 on 1080p should give MarginV=1080, got: {style}"
        );
    }

    #[test]
    fn test_caption_style_contains_border_style_3() {
        let style = build_caption_style("#FFFFFF", "#000000B3", 24, 90, 1080);
        assert!(
            style.contains("BorderStyle=3"),
            "Must use BorderStyle=3 for opaque box mode, got: {style}"
        );
    }

    #[test]
    fn test_caption_style_uses_outline_colour_for_bg() {
        // The root cause of the missing background bug: bg color must go on
        // OutlineColour (not BackColour) when BorderStyle=3 is used.
        let style = build_caption_style("#FFFFFF", "#FF0000B3", 24, 90, 1080);
        // bg=#FF0000B3 → R=FF,G=00,B=00 → BGR=0000FF, alpha=4C
        assert!(
            style.contains("OutlineColour=&H4C0000FF&"),
            "User bg color must go on OutlineColour for BorderStyle=3, got: {style}"
        );
        // BackColour should be the fixed shadow value, not the user's bg color
        assert!(
            style.contains("BackColour=&H80000000&"),
            "BackColour should be fixed shadow value, got: {style}"
        );
    }

    #[test]
    fn test_caption_style_font_size() {
        let style = build_caption_style("#FFFFFF", "#000000B3", 36, 90, 1080);
        assert!(
            style.contains("FontSize=36"),
            "FontSize should match input, got: {style}"
        );
    }

    #[test]
    fn test_caption_style_720p_margin() {
        let style = build_caption_style("#FFFFFF", "#000000B3", 24, 90, 720);
        // MarginV = (100-90)/100 * 720 = 72
        assert!(
            style.contains("MarginV=72"),
            "position=90 on 720p should give MarginV=72, got: {style}"
        );
    }

    // ---- FFmpeg build_export_args tests ----

    fn default_audio_opts() -> ExportAudioOptions {
        ExportAudioOptions {
            normalize_audio: false,
            volume_db: 0.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
        }
    }

    #[test]
    fn test_build_export_args_single_segment_video() {
        let args = build_export_args(
            "input.mp4",
            "output.mp4",
            &[(0, 5_000_000)],
            true,
            &default_audio_opts(),
            None,
            "FontSize=24",
            None,
        );
        assert!(args.contains(&"-y".to_string()));
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"input.mp4".to_string()));
        assert!(args.contains(&"output.mp4".to_string()));
    }

    #[test]
    fn test_build_export_args_single_segment_with_captions() {
        let args = build_export_args(
            "input.mp4",
            "output.mp4",
            &[(0, 5_000_000)],
            true,
            &default_audio_opts(),
            Some("C:\\path\\to\\captions.srt"),
            "FontSize=24,BorderStyle=3",
            None,
        );
        let vf_idx = args.iter().position(|a| a == "-vf");
        assert!(vf_idx.is_some(), "Single segment video with captions should have -vf flag");
        let filter = &args[vf_idx.unwrap() + 1];
        assert!(
            filter.contains("subtitles="),
            "Filter should contain subtitles directive, got: {filter}"
        );
        assert!(
            filter.contains("force_style='FontSize=24,BorderStyle=3'"),
            "Filter should include caption style, got: {filter}"
        );
    }

    #[test]
    fn test_build_export_args_multi_segment_video() {
        let segments = vec![(0, 2_000_000), (3_000_000, 5_000_000)];
        let args = build_export_args(
            "input.mp4",
            "output.mp4",
            &segments,
            true,
            &default_audio_opts(),
            None,
            "FontSize=24",
            None,
        );
        assert!(
            args.contains(&"-filter_complex".to_string()),
            "Multi-segment should use filter_complex"
        );
        let fc_idx = args.iter().position(|a| a == "-filter_complex").unwrap();
        let filter = &args[fc_idx + 1];
        assert!(filter.contains("concat=n=2"), "Should concat 2 segments, got: {filter}");
        assert!(filter.contains("[v0]"), "Should reference video segment 0");
        assert!(filter.contains("[v1]"), "Should reference video segment 1");
    }

    #[test]
    fn test_build_export_args_multi_segment_with_captions() {
        let segments = vec![(0, 2_000_000), (3_000_000, 5_000_000)];
        let args = build_export_args(
            "input.mp4",
            "output.mp4",
            &segments,
            true,
            &default_audio_opts(),
            Some("C:\\captions.srt"),
            "FontSize=24,BorderStyle=3",
            None,
        );
        let fc_idx = args.iter().position(|a| a == "-filter_complex").unwrap();
        let filter = &args[fc_idx + 1];
        assert!(
            filter.contains("subtitles="),
            "Multi-segment captions should chain subtitles filter"
        );
        assert!(
            filter.contains("[outvs]"),
            "Should output to [outvs] label after subtitles"
        );
        // The map should reference the subtitled output
        let map_indices: Vec<_> = args
            .iter()
            .enumerate()
            .filter(|(_, a)| a.as_str() == "-map")
            .map(|(i, _)| i)
            .collect();
        assert!(
            args[map_indices[0] + 1] == "[outvs]",
            "First -map should reference [outvs] for captioned video"
        );
    }

    #[test]
    fn test_srt_path_escaping() {
        // Backslashes become forward slashes, colons get escaped with backslash
        let escaped = escape_srt_path_for_ffmpeg("C:\\Users\\test\\file.srt");
        assert_eq!(escaped, "C\\:/Users/test/file.srt");
        let escaped2 = escape_srt_path_for_ffmpeg("D:\\test.srt");
        assert_eq!(escaped2, "D\\:/test.srt", "Colons should be escaped for FFmpeg filter syntax");
        // Unix-style path with no special chars passes through
        let escaped3 = escape_srt_path_for_ffmpeg("/tmp/captions.srt");
        assert_eq!(escaped3, "/tmp/captions.srt");
    }

    #[test]
    fn test_build_export_args_audio_only() {
        let args = build_export_args(
            "input.wav",
            "output.mp3",
            &[(0, 5_000_000)],
            false,
            &default_audio_opts(),
            None,
            "",
            None,
        );
        // Audio-only should not contain -vf or video filter
        assert!(
            !args.contains(&"-vf".to_string()),
            "Audio-only export should not have -vf"
        );
        assert!(
            !args.contains(&"-filter_complex".to_string()),
            "Single segment audio should not need filter_complex"
        );
    }

    #[test]
    fn test_single_segment_captions_include_original_size() {
        let args = build_export_args(
            "input.mp4",
            "output.mp4",
            &[(0, 5_000_000)],
            true,
            &default_audio_opts(),
            Some("C:\\path\\to\\captions.srt"),
            "FontSize=24,MarginV=108",
            Some((1920, 1080)),
        );
        let vf_idx = args.iter().position(|a| a == "-vf").unwrap();
        let filter = &args[vf_idx + 1];
        assert!(
            filter.contains("original_size=1920x1080"),
            "Single segment subtitle filter must include original_size to match preview coordinates, got: {filter}"
        );
    }

    #[test]
    fn test_multi_segment_captions_include_original_size() {
        let segments = vec![(0, 2_000_000), (3_000_000, 5_000_000)];
        let args = build_export_args(
            "input.mp4",
            "output.mp4",
            &segments,
            true,
            &default_audio_opts(),
            Some("C:\\captions.srt"),
            "FontSize=24,MarginV=72",
            Some((1280, 720)),
        );
        let fc_idx = args.iter().position(|a| a == "-filter_complex").unwrap();
        let filter = &args[fc_idx + 1];
        assert!(
            filter.contains("original_size=1280x720"),
            "Multi-segment subtitle filter must include original_size, got: {filter}"
        );
    }

    #[test]
    fn test_captions_without_video_size_omit_original_size() {
        let args = build_export_args(
            "input.mp4",
            "output.mp4",
            &[(0, 5_000_000)],
            true,
            &default_audio_opts(),
            Some("C:\\captions.srt"),
            "FontSize=24",
            None,
        );
        let vf_idx = args.iter().position(|a| a == "-vf").unwrap();
        let filter = &args[vf_idx + 1];
        assert!(
            !filter.contains("original_size"),
            "Without video_size, original_size should not appear, got: {filter}"
        );
    }
}
