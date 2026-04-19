use log::{debug, warn};
use std::path::Path;
use std::time::Duration;
use tauri::AppHandle;

use crate::managers::editor::{EditorState, TimingContractSnapshot};
use crate::managers::splice::boundaries::{
    snap_segments_energy_biased, DEFAULT_ENERGY_RADIUS_US, DEFAULT_SNAP_RADIUS_US,
};
use crate::managers::splice::loudness::{build_loudnorm_filter, LoudnessTarget};

mod preview_cache;
mod export_format;
pub use export_format::{export_format_codec_map, CodecSpec, AudioExportFormat};
use preview_cache::{
    edit_version_token, preview_generation_token, preview_output_path, source_media_fingerprint,
    urlencoding,
};

/// Seam fade applied symmetrically on both the preview and export paths.
/// 20 ms matches one AAC MDCT window (~23 ms at 44.1 kHz), so the codec sees a
/// full frame of continuous material across every edit seam. Per AGENTS.md's
/// dual-path rule this is a single constant — preview and export MUST NOT
/// drift (previously EXPORT_SEAM_FADE_US=10ms vs PREVIEW_SEAM_FADE_US=0 was a
/// dual-path violation). See todo p0-waveform-boundary-policy.
const SEAM_FADE_US: i64 = 20_000;

const FIRST_BOUNDARY_FADE_US: i64 = 2_000;

/// Pre-speech padding removed from the outer edges of the first/last kept
/// segment when the transcription engine is known to include significant
/// leading/trailing silence in its word timestamps (notably Parakeet).
/// Whisper and unknown engines get 0 µs — the previous unconditional 300 ms
/// trim was amputating the first/last kept word on those engines. Callers
/// that know they're in Parakeet territory pass PARAKEET_OUTER_TRIM_US; all
/// others pass 0. See todo p0-waveform-boundary-policy.
//
// Forward-looking infrastructure from todo p0-waveform-boundary-policy. No
// caller passes this constant yet because the engine type isn't plumbed
// through EditorState; it will be consumed once the adapter trait lands.
// TODO(p1-adapter-trait): wire engine_type through EditorState and pass this
// constant from the Parakeet-aware site.
#[allow(dead_code)]
const PARAKEET_OUTER_TRIM_US: i64 = 300_000;
/// FFmpeg preview render timeout (10 minutes).
const PREVIEW_RENDER_TIMEOUT: Duration = Duration::from_secs(600);
/// FFmpeg export timeout (30 minutes).
const EXPORT_TIMEOUT: Duration = Duration::from_secs(1800);

/// Audio post-processing options applied to the entire export output.
#[derive(Debug, Clone, Default)]
struct ExportAudioOptions {
    /// Selected loudness normalization target. The `loudnorm` filter
    /// string is built by `build_loudnorm_filter`; this struct only
    /// carries the enum so the FFmpeg arg construction remains the
    /// single Rust authority for the filter parameters (AGENTS.md
    /// "Single source of truth for dual-path logic").
    loudness_target: LoudnessTarget,
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

pub(crate) fn probe_video_dimensions(path: &str) -> Option<(u32, u32)> {
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

    if let Some(loudnorm) = build_loudnorm_filter(opts.loudness_target) {
        filters.push(loudnorm);
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
    build_audio_concat_filter_with_fade(segments, SEAM_FADE_US)
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

/// Project silenced source-time ranges onto the edited timeline.
///
/// Silenced ranges live in source-time (returned by
/// `EditorState::get_silenced_ranges`); this function clips each range to
/// the portion(s) that survive inside the keep-segments and rewrites them
/// into the post-concat edit-time space that the silence filter uses.
///
/// Overlapping or adjacent output ranges are merged so the emitted
/// `volume=enable='between(...)'` chain never double-applies to the same
/// sample window.
fn silenced_edit_time_ranges(
    silenced_source_ranges: &[(i64, i64)],
    keep_segments: &[(i64, i64)],
) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    let mut elapsed: i64 = 0;
    for (ks, ke) in keep_segments {
        let seg_dur = (ke - ks).max(0);
        for (ss, se) in silenced_source_ranges {
            let lo = (*ss).max(*ks);
            let hi = (*se).min(*ke);
            if hi > lo {
                out.push((elapsed + (lo - ks), elapsed + (hi - ks)));
            }
        }
        elapsed += seg_dur;
    }
    out.sort_by_key(|r| r.0);
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for (s, e) in out {
        if let Some(last) = merged.last_mut() {
            if s <= last.1 {
                last.1 = last.1.max(e);
                continue;
            }
        }
        merged.push((s, e));
    }
    merged
}

/// Build a `volume=enable='between(t,S,E)':volume=0` chain that mutes each
/// supplied edit-time range on the final post-concat audio stream.
///
/// Silenced ranges are purely multiplicative (volume=0), so they compose
/// cleanly with the seam-fade policy applied inside the per-segment atrim
/// branches — silenced audio inside a keep-segment rides over the segment
/// interior, not the seam, so there is no double-fade risk.
fn silence_filter_chain(edit_ranges: &[(i64, i64)]) -> Option<String> {
    if edit_ranges.is_empty() {
        return None;
    }
    let filters: Vec<String> = edit_ranges
        .iter()
        .map(|(s, e)| {
            let ss = *s as f64 / 1_000_000.0;
            let ee = *e as f64 / 1_000_000.0;
            format!("volume=enable='between(t,{ss:.6},{ee:.6})':volume=0")
        })
        .collect();
    Some(filters.join(","))
}

/// Append a silence gate chain to an existing `[outa]`-sinking filter_complex
/// graph, renaming the current sink and re-terminating at `[outa]`.
fn append_silence_gate(filter: &mut String, edit_ranges: &[(i64, i64)]) {
    if let Some(gate) = silence_filter_chain(edit_ranges) {
        *filter = filter.replace("[outa]", "[outa_raw]");
        filter.push_str(&format!("; [outa_raw]{gate}[outa]"));
    }
}

/// Canonical keep-segments for preview/export paths.
///
/// Uses the timing contract snapshot as the source of truth and normalizes
/// bounds/order so preview and export consume identical segment semantics.
fn settings_experimental_simplify_mode_enabled(app: &AppHandle) -> bool {
    let settings = crate::settings::get_settings(app);
    crate::settings::is_experiment_enabled(
        &settings,
        crate::settings::ExperimentKey::SimplifyMode,
    )
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
    // Default: no outer trim (Whisper / unknown engines — see todo
    // p0-waveform-boundary-policy). Callers that know they're running a
    // transcription engine whose word timestamps include significant
    // pre-speech padding (Parakeet) should use
    // `canonical_keep_segments_for_media_with_options` and pass
    // `PARAKEET_OUTER_TRIM_US`. Seam fades ride inside kept segments (see
    // `build_audio_segment_filter`); no seam-edge extension is applied.
    canonical_keep_segments_for_media_with_options(state, experimental_simplify_mode, 0)
}

fn canonical_keep_segments_for_media_with_options(
    state: &EditorState,
    experimental_simplify_mode: bool,
    outer_trim_us: i64,
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

    // Outer-edge trim: only applied when the caller knows the transcription
    // engine pads the first/last word with silence (Parakeet). See todo
    // p0-waveform-boundary-policy — the previous unconditional 300 ms trim
    // was amputating the first/last kept word on Whisper and any other
    // engine.
    if outer_trim_us > 0 && !normalized.is_empty() {
        let first = &mut normalized[0];
        let seg_dur = first.1 - first.0;
        let trim = (seg_dur / 2).min(outer_trim_us);
        first.0 += trim;

        let last = normalized.last_mut().unwrap();
        let seg_dur = last.1 - last.0;
        let trim = (seg_dur / 2).min(outer_trim_us);
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

/// Snap every `(start_us, end_us)` pair to the nearest **energy valley**
/// (plus zero-crossing) in the decoded source audio.
///
/// Zero-crossing snap alone eliminates the *click* at a seam but still lands
/// the boundary at whichever ZC is arithmetically closest — which, right at
/// the trailing edge of a deleted phoneme, is often a few ms *inside* that
/// phoneme. The result is faint bleed-through of the deleted sound ("uh"
/// after "And uh" → "And").
///
/// This energy-biased variant widens the search to ±`DEFAULT_ENERGY_RADIUS_US`
/// (20 ms), picks the quietest short frame, then snaps that to the nearest
/// zero-crossing within ±`DEFAULT_SNAP_RADIUS_US`. In voiced-only audio with
/// no energy gradient the behaviour degenerates back to plain ZC snap.
///
/// Decodes the media exactly once (via `ffmpeg -f f32le`), so preview and
/// export pay the same decode cost they already pay during the current
/// render. Returns the input segments unchanged if decode fails — **never**
/// regresses the current behavior.
fn snap_segments_against_media(segments: &[(i64, i64)], media_path: &Path) -> Vec<(i64, i64)> {
    if segments.len() < 2 {
        return segments.to_vec();
    }
    match crate::commands::disfluency::decode_media_audio(media_path) {
        Ok(samples) => {
            let snapped = snap_segments_energy_biased(
                segments,
                &samples,
                16_000,
                DEFAULT_ENERGY_RADIUS_US,
                DEFAULT_SNAP_RADIUS_US,
            );
            if snapped.is_empty() {
                segments.to_vec()
            } else {
                snapped
            }
        }
        Err(e) => {
            warn!(
                "Zero-crossing snap skipped for {}: decode failed ({}). Falling back to original segments.",
                media_path.display(),
                e
            );
            segments.to_vec()
        }
    }
}

fn build_preview_render_args(
    input_path: &Path,
    output_path: &Path,
    segments: &[(i64, i64)],
    silenced_source_ranges: &[(i64, i64)],
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string_lossy().to_string(),
        "-vn".to_string(),
    ];

    // Preview and export share the same seam fade policy (dual-path rule).
    let mut filter = build_audio_concat_filter_with_fade(segments, SEAM_FADE_US);
    let silenced_edit_ranges = silenced_edit_time_ranges(silenced_source_ranges, segments);
    append_silence_gate(&mut filter, &silenced_edit_ranges);
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
    audio_only_spec: Option<&CodecSpec>,
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

    if let Some(spec) = audio_only_spec {
        args.extend(["-c:a".to_string(), spec.codec.to_string()]);
        if let Some(b) = spec.bitrate_flag() {
            args.extend(["-b:a".to_string(), b]);
        }
    } else {
        args.extend([
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "192k".to_string(),
        ]);
    }
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
    subtitle_path: Option<&str>,
    fonts_dir: Option<&str>,
    silenced_source_ranges: &[(i64, i64)],
    format: AudioExportFormat,
) -> Vec<String> {
    // Audio-only formats force the video stream out (-vn) and pick a
    // codec/bitrate from the central codec map. The video pipeline
    // (Mp4) keeps the existing libx264 + aac behavior. Per AGENTS.md
    // dual-path rule + R-005, `build_audio_post_filters` (loudness +
    // seam fades) is reused unchanged for both paths.
    let audio_only_spec = export_format_codec_map(format);
    let effective_has_video = has_video && audio_only_spec.is_none();

    let mut args: Vec<String> = vec!["-y".to_string(), "-i".to_string(), input_path.to_string()];

    let total_duration_s: f64 = segments
        .iter()
        .map(|(s, e)| (e - s).max(0) as f64 / 1_000_000.0)
        .sum();

    let silenced_edit_ranges = silenced_edit_time_ranges(silenced_source_ranges, segments);
    let silence_gate = silence_filter_chain(&silenced_edit_ranges);

    if segments.len() == 1 {
        // Single segment — simple trim with re-encode for sample-accurate cuts
        let (start, end) = segments[0];
        extend_single_segment_export_args(
            &mut args,
            start,
            end,
            effective_has_video,
            audio_only_spec.as_ref(),
        );
        let post_filter = build_audio_post_filters(audio_opts, total_duration_s);
        let combined_af = match (silence_gate.as_ref(), post_filter) {
            (Some(gate), Some(pf)) => Some(format!("{gate},{pf}")),
            (Some(gate), None) => Some(gate.clone()),
            (None, Some(pf)) => Some(pf),
            (None, None) => None,
        };
        if let Some(af) = combined_af {
            args.extend(["-af".to_string(), af]);
        }
        // For audio-only single-segment exports, drop the source video
        // stream explicitly so the muxer does not choke on a leftover
        // unmapped video stream.
        if audio_only_spec.is_some() {
            args.push("-vn".to_string());
        }
        // Burn-in subtitles via -vf for single-segment video exports
        if effective_has_video {
            if let Some(sub) = subtitle_path {
                let escaped = escape_srt_path_for_ffmpeg(sub);
                let fonts_param = match fonts_dir {
                    Some(dir) => format!(":fontsdir='{}'", escape_srt_path_for_ffmpeg(dir)),
                    None => String::new(),
                };
                args.extend([
                    "-vf".to_string(),
                    format!("subtitles='{escaped}'{fonts_param}"),
                ]);
            }
        }
    } else {
        // Multiple segments — filter_complex with trim/atrim + concat
        let post_filters = build_audio_post_filters(audio_opts, total_duration_s);
        let combined_post = match (silence_gate.as_ref(), post_filters) {
            (Some(gate), Some(pf)) => Some(format!("{gate},{pf}")),
            (Some(gate), None) => Some(gate.clone()),
            (None, Some(pf)) => Some(pf),
            (None, None) => None,
        };

        if effective_has_video {
            let mut filter_parts = Vec::new();
            let n = segments.len();
            for (i, (start, end)) in segments.iter().enumerate() {
                let start_s = *start as f64 / 1_000_000.0;
                let end_s = *end as f64 / 1_000_000.0;
                filter_parts.push(format!(
                    "[0:v]trim=start={start_s:.6}:end={end_s:.6},setpts=PTS-STARTPTS[v{i}]"
                ));
                filter_parts.push(build_audio_segment_filter(i, n, *start, *end, SEAM_FADE_US));
            }
            let v_inputs: String = (0..n).map(|i| format!("[v{i}]")).collect();
            let a_inputs: String = (0..n).map(|i| format!("[a{i}]")).collect();
            if let Some(ref pf) = combined_post {
                filter_parts.push(format!(
                    "{v_inputs}concat=n={n}:v=1:a=0[outv]; {a_inputs}concat=n={n}:v=0:a=1[outa_raw]; [outa_raw]{pf}[outa]"
                ));
            } else {
                filter_parts.push(format!(
                    "{v_inputs}concat=n={n}:v=1:a=0[outv]; {a_inputs}concat=n={n}:v=0:a=1[outa]"
                ));
            }

            // Burn-in subtitles: chain after [outv] in filter_complex
            let video_map_label = if let Some(sub) = subtitle_path {
                let escaped = escape_srt_path_for_ffmpeg(sub);
                let fonts_param = match fonts_dir {
                    Some(dir) => format!(":fontsdir='{}'", escape_srt_path_for_ffmpeg(dir)),
                    None => String::new(),
                };
                filter_parts.push(format!("[outv]subtitles='{escaped}'{fonts_param}[outvs]"));
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
            if let Some(ref pf) = combined_post {
                filter = filter.replace("[outa]", "[outa_raw]");
                filter.push_str(&format!("; [outa_raw]{pf}[outa]"));
            }
            args.extend([
                "-filter_complex".to_string(),
                filter,
                "-map".to_string(),
                "[outa]".to_string(),
            ]);
            // Audio-only multi-segment: explicit -vn so any source
            // video that survives the filter graph is dropped before
            // muxing.
            if audio_only_spec.is_some() {
                args.push("-vn".to_string());
            }
        }

        // Multi-segment audio-only output codec selection. The
        // single-segment path is handled inside
        // `extend_single_segment_export_args` so it can sit alongside
        // the existing -ss/-to trim args.
        if let Some(spec) = audio_only_spec.as_ref() {
            args.extend(["-c:a".to_string(), spec.codec.to_string()]);
            if let Some(b) = spec.bitrate_flag() {
                args.extend(["-b:a".to_string(), b]);
            }
        }
    }

    args.push(output_path.to_string());
    args
}

mod commands;
pub use commands::*;

/// Test-only public wrapper around the private `build_export_args`.
///
/// Integration tests in `src-tauri/tests/` need to drive the audio-only
/// export pipeline end-to-end (AC-003-a round-trip), but the underlying
/// builder is intentionally crate-private. This shim exposes a minimal
/// surface using neutral defaults so tests don't have to reach into
/// private types like `ExportAudioOptions`.
///
/// Always uses `LoudnessTarget::Off` and zero volume/fades — those
/// concerns are tested independently. AGENTS.md "Single source of
/// truth": this shim does NOT re-implement codec selection; it
/// dispatches through `build_export_args`, the same function the
/// real export command uses.
pub fn build_audio_only_export_args_for_tests(
    input_path: &str,
    output_path: &str,
    segments: &[(i64, i64)],
    format: AudioExportFormat,
) -> Vec<String> {
    let opts = ExportAudioOptions {
        loudness_target: crate::managers::splice::loudness::LoudnessTarget::Off,
        volume_db: 0.0,
        fade_in_ms: 0,
        fade_out_ms: 0,
    };
    build_export_args(
        input_path,
        output_path,
        segments,
        true, // pretend the source has video; audio-only must drop it
        &opts,
        None,
        None,
        &[],
        format,
    )
}

#[cfg(test)]
mod tests;
