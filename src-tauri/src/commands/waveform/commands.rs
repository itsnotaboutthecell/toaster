//! Tauri command handlers for waveform/preview/export (extracted from mod.rs).
//!
//! These commands consume the private helpers defined in mod.rs. Child-module
//! visibility lets this file access them directly via `use super::*;`.

use std::time::Instant;

use log::{debug, info, warn};
use tauri::{AppHandle, State};

use super::preview_cache::{
    cleanup_preview_cache, invalidate_preview_cache_entries, preview_cache_dir,
};
use super::*;
use crate::commands::editor::EditorStore;
use crate::managers::media::MediaStore;

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

pub(super) fn normalize_peaks(mut peaks: Vec<f32>) -> Vec<f32> {
    let global_max = peaks.iter().copied().fold(0.01_f32, f32::max);
    for p in &mut peaks {
        *p /= global_max;
    }
    peaks
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
    let state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
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
/// Note: this diagnostic script reflects keep-segments only. Silenced words
/// (from `EditorState::get_silenced_ranges`) are applied inside the live
/// preview/export paths via a post-concat `volume=enable='between(...)'`
/// gate (see `silence_filter_chain`) and are intentionally not reproduced
/// here — the script is a debug aid, not a render-parity artifact.
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
    let state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
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
            // Audio leg uses the same seam-fade policy as preview/export so
            // the generated recipe matches the live render (AGENTS.md
            // dual-path rule; todo p0-waveform-boundary-policy).
            let audio_filter = build_audio_segment_filter(i, n, *start, *end, SEAM_FADE_US);
            filter_parts.push(format!(
                "[0:v]trim=start={start_s:.6}:end={end_s:.6},setpts=PTS-STARTPTS[v{i}]; {audio_filter}"
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
///
/// Always drives the mapping from `canonical_keep_segments_for_media` — the
/// same function the preview render (`render_temp_preview_audio`) and export
/// use — so the cursor and the audio it's scrubbing over stay sample-aligned
/// regardless of `experimental_simplify_mode`. The previous default path
/// routed through `EditorState::map_edit_time_to_source_time`, which uses raw
/// legacy keep-segments and drifted against the rendered audio whenever the
/// two pipelines disagreed on seam placement. See splice-logic synthesis
/// report.
#[tauri::command]
#[specta::specta]
pub fn map_edit_to_source_time(
    app: AppHandle,
    store: State<EditorStore>,
    edit_time_us: i64,
) -> Result<i64, String> {
    let experimental_simplify_mode = settings_experimental_simplify_mode_enabled(&app);
    let state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
    let segments = canonical_keep_segments_for_media(&state, experimental_simplify_mode);
    Ok(map_edit_time_to_source_time_from_segments(
        edit_time_us,
        &segments,
    ))
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
    let (segments, silenced_ranges) = {
        let state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
        (
            canonical_keep_segments_for_media(&state, experimental_simplify_mode),
            state.get_silenced_ranges(),
        )
    };

    let edit_version = edit_version_token(&segments, &silenced_ranges);

    let media_info = {
        let state = crate::lock_recovery::try_lock(media_store.0.lock()).map_err(|e| e.to_string())?;
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
        let snapped_segments = snap_segments_against_media(&segments, &media.path);
        let args = build_preview_render_args(
            &media.path,
            &output_path,
            &snapped_segments,
            &silenced_ranges,
        );

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
    let (segments, words, silenced_ranges) = {
        let state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
        let segs = canonical_keep_segments_for_media(&state, experimental_simplify_mode);
        let w = state.get_words().to_vec();
        let silenced = state.get_silenced_ranges();
        (segs, w, silenced)
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

    // Generate temp ASS for burn-in captions when requested on video exports.
    // The ASS file is produced from the authoritative `CaptionBlock` stream so
    // the export matches the preview exactly — rounded corners, Inter/Roboto
    // font, pixel-width wrapping, and consistent padding. See
    // `managers::captions::ass` for the document schema.
    let video_dims = probe_video_dimensions(input.to_str().unwrap_or(""));
    let frame_size = video_dims.unwrap_or_else(|| {
        log::warn!("Could not probe video dimensions for caption layout, assuming 1920x1080");
        (1920, 1080)
    });

    let settings = crate::settings::get_settings(&app);
    let ass_temp_path = if burn_captions.unwrap_or(false)
        && has_video
        && !settings.export_format.is_audio_only()
    {
        let blocks = crate::commands::export::build_caption_blocks_for_export(
            &words, &segments, &settings, frame_size,
        );
        let doc = crate::managers::captions::blocks_to_ass(&blocks);
        let ass_file = std::path::Path::new(&output_path).with_extension("burn_captions.ass");
        std::fs::write(&ass_file, &doc).map_err(|e| format!("Failed to write temp ASS: {}", e))?;
        Some(ass_file)
    } else {
        None
    };

    let audio_opts = ExportAudioOptions {
        loudness_target: crate::settings::migrate_loudness_setting(
            Some(settings.normalize_audio_on_export),
            Some(settings.loudness_target),
        ),
        volume_db: settings.export_volume_db.clamp(-60.0, 24.0),
        fade_in_ms: settings.export_fade_in_ms.min(30_000),
        fade_out_ms: settings.export_fade_out_ms.min(30_000),
    };

    let export_format = settings.export_format;
    // Audio-only formats drop the video stream regardless of source.
    let effective_has_video = has_video && !export_format.is_audio_only();

    // If the chosen format does not match the output filename's
    // extension, swap the extension. The frontend file picker uses the
    // format's `.extension()` as the suggestion (R-001 / edge case in
    // PRD); this is a defensive backstop for callers that pass the raw
    // input filename through.
    let output_path_buf = {
        let want_ext = export_format.extension().trim_start_matches('.');
        let p = std::path::Path::new(&output_path);
        let cur = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        if cur.eq_ignore_ascii_case(want_ext) {
            std::path::PathBuf::from(&output_path)
        } else {
            p.with_extension(want_ext)
        }
    };
    let output_path_str = output_path_buf.to_string_lossy().to_string();

    let fonts_dir = crate::commands::export::bundled_fonts_dir(&app);
    let fonts_dir_str = fonts_dir.as_ref().map(|p| p.to_string_lossy().to_string());

    let ass_path_str = ass_temp_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());
    let snapped_segments = snap_segments_against_media(&segments, input);
    let args = build_export_args(
        &input_path,
        &output_path_str,
        &snapped_segments,
        effective_has_video,
        &audio_opts,
        ass_path_str.as_deref(),
        fonts_dir_str.as_deref(),
        &silenced_ranges,
        export_format,
    );

    log::info!("Running FFmpeg export: ffmpeg {}", args.join(" "));

    let export_result = tokio::time::timeout(
        EXPORT_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("ffmpeg").args(&args).output()
        }),
    )
    .await;

    // Clean up temp ASS regardless of export outcome
    if let Some(ref ass_file) = ass_temp_path {
        let _ = std::fs::remove_file(ass_file);
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
    log::info!("FFmpeg export complete: {}", output_path_str);
    Ok(format!("Export complete: {}\n{}", output_path_str, stdout))
}

use crate::managers::splice::loudness::{compute_loudness_preflight, LoudnessPreflight};

/// Loudness preflight: render the post-edit audio (same keep-segments
/// + seam-fade chain that preview/export use) into PCM and run an EBU
/// R128 measurement against the selected target.
///
/// AGENTS.md "Single source of truth for dual-path logic": the
/// underlying measurement and target lookup live in
/// `managers::splice::loudness`; this command only orchestrates the
/// FFmpeg decode and forwards the buffer.
///
/// AC-002-a / AC-002-b / AC-002-c.
#[tauri::command]
#[specta::specta]
pub async fn loudness_preflight(
    app: AppHandle,
    store: State<'_, EditorStore>,
    media_store: State<'_, MediaStore>,
    target: crate::managers::splice::loudness::LoudnessTarget,
) -> Result<LoudnessPreflight, String> {
    let experimental_simplify_mode = settings_experimental_simplify_mode_enabled(&app);
    let (segments, silenced_ranges) = {
        let state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
        (
            canonical_keep_segments_for_media(&state, experimental_simplify_mode),
            state.get_silenced_ranges(),
        )
    };

    if segments.is_empty() {
        return Err("No segments to measure (all words deleted)".to_string());
    }

    let media_info = {
        let state =
            crate::lock_recovery::try_lock(media_store.0.lock()).map_err(|e| e.to_string())?;
        state.current().cloned()
    };
    let media = media_info.ok_or_else(|| "No media loaded for preflight".to_string())?;
    if !media.path.exists() {
        return Err(format!(
            "Media file missing: {}",
            media.path.to_string_lossy()
        ));
    }

    // Same seam-fade policy as preview/export so the preflight numbers
    // describe the audio the user will actually hear/export.
    let snapped_segments = snap_segments_against_media(&segments, &media.path);
    let mut filter = build_audio_concat_filter_with_fade(&snapped_segments, SEAM_FADE_US);
    let silenced_edit_ranges = silenced_edit_time_ranges(&silenced_ranges, &snapped_segments);
    append_silence_gate(&mut filter, &silenced_edit_ranges);

    const PREFLIGHT_SAMPLE_RATE_HZ: u32 = 48_000;
    const PREFLIGHT_CHANNELS: u32 = 1;

    let media_path = media.path.clone();
    let args: Vec<String> = vec![
        "-v".to_string(),
        "error".to_string(),
        "-i".to_string(),
        media_path.to_string_lossy().to_string(),
        "-vn".to_string(),
        "-filter_complex".to_string(),
        filter,
        "-map".to_string(),
        "[outa]".to_string(),
        "-ac".to_string(),
        PREFLIGHT_CHANNELS.to_string(),
        "-ar".to_string(),
        PREFLIGHT_SAMPLE_RATE_HZ.to_string(),
        "-f".to_string(),
        "f32le".to_string(),
        "pipe:1".to_string(),
    ];

    let started = Instant::now();
    let decode_result = tokio::time::timeout(
        PREVIEW_RENDER_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("ffmpeg").args(&args).output()
        }),
    )
    .await;

    let output = match decode_result {
        Ok(join_result) => join_result
            .map_err(|e| format!("Preflight task panicked: {}", e))?
            .map_err(|e| format!("FFmpeg not found. Install FFmpeg. Error: {}", e))?,
        Err(_) => {
            return Err(format!(
                "Preflight decode timed out after {} minutes",
                PREVIEW_RENDER_TIMEOUT.as_secs() / 60
            ));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("FFmpeg preflight decode failed: {}", stderr));
    }

    let mut samples: Vec<f32> = Vec::with_capacity(output.stdout.len() / 4);
    for chunk in output.stdout.chunks_exact(4) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }

    let preflight = compute_loudness_preflight(
        &samples,
        PREFLIGHT_SAMPLE_RATE_HZ,
        PREFLIGHT_CHANNELS,
        target,
    )
    .map_err(|e| format!("EBU R128 measurement failed: {}", e))?;

    log::info!(
        "Preflight complete in {} ms: integrated={:.2} LUFS, peak={:.2} dBTP, lra={:.2} LU, target={:?}",
        started.elapsed().as_millis(),
        preflight.integrated_lufs,
        preflight.true_peak_dbtp,
        preflight.lra,
        preflight.target_lufs,
    );

    Ok(preflight)
}
