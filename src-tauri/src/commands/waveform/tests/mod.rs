use super::*;

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
    if values.len().is_multiple_of(2) {
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

    let deleted_phrases =
        collect_deleted_phrases_from_source_segments(&source_segments, &report.deleted_ranges_us);
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
            "ASR oracle did not recover a deleted phrase set from source transcription".to_string(),
        );
        return report;
    }

    report.pass = report.preview_leaked_deleted_phrases.is_empty()
        && report.export_leaked_deleted_phrases.is_empty();
    report
}

/// Inputs for [`collect_live_validation_failure_reasons`]. Bundled into a
/// struct so adding new metrics doesn't balloon the function signature
/// past clippy's `too_many_arguments` threshold (see todo
/// p0-waveform-boundary-policy — the duration/tolerance/seam knobs pushed
/// the signature to 9 args).
struct LiveValidationFailureInputs<'a> {
    preview_duration_error_us: i64,
    export_duration_error_us: i64,
    preview_duration_tolerance_us: i64,
    export_duration_tolerance_us: i64,
    boundary_metric_pass: bool,
    seam_metric_pass: bool,
    seam_ratios: &'a [f32],
    seam_max_ratio: f32,
    asr_leakage_oracle: &'a AsrLeakageOracleReport,
}

fn collect_live_validation_failure_reasons(inputs: LiveValidationFailureInputs<'_>) -> Vec<String> {
    let LiveValidationFailureInputs {
        preview_duration_error_us,
        export_duration_error_us,
        preview_duration_tolerance_us,
        export_duration_tolerance_us,
        boundary_metric_pass,
        seam_metric_pass,
        seam_ratios,
        seam_max_ratio,
        asr_leakage_oracle,
    } = inputs;
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

mod part1;
mod part2;
