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
    // Seams land exactly on the deleted-word boundaries. Seam fade is
    // applied inside the kept segments by `build_audio_segment_filter`
    // and never pulls deleted audio back across the cut.
    assert_eq!(segments, vec![(0, 1_000_000), (2_000_000, 3_000_000)]);
}

#[test]
fn canonical_keep_segments_with_parakeet_outer_trim_removes_outer_padding() {
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

    // Parakeet opt-in: 300 ms outer trim. Exercises the with_options
    // API to cover the outer-trim path (the only remaining tunable on
    // this function after CUT_GUARD_US was removed).
    let segments =
        canonical_keep_segments_for_media_with_options(&state, false, PARAKEET_OUTER_TRIM_US);
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
fn canonical_keep_segments_never_extend_past_deleted_neighbour() {
    // Regression: an earlier guard-band knob used to extend kept segments
    // 20 ms into the deleted region on each side of every seam, which
    // reintroduced the onset/offset of the deleted word. With the knob
    // removed, no neighbouring segment may intersect the deleted
    // interval.
    let deleted_start = 1_000_000_i64;
    let deleted_end = 2_000_000_i64;
    let mut state = EditorState::new();
    state.set_words(vec![
        Word {
            text: "alpha".to_string(),
            start_us: 0,
            end_us: deleted_start,
            deleted: false,
            silenced: false,
            confidence: 0.9,
            speaker_id: 0,
        },
        Word {
            text: "the".to_string(),
            start_us: deleted_start,
            end_us: deleted_end,
            deleted: true,
            silenced: false,
            confidence: 0.9,
            speaker_id: 0,
        },
        Word {
            text: "gamma".to_string(),
            start_us: deleted_end,
            end_us: 3_000_000,
            deleted: false,
            silenced: false,
            confidence: 0.9,
            speaker_id: 0,
        },
    ]);

    let segments = canonical_keep_segments_for_media(&state, false);
    for seg in &segments {
        let intersects = seg.0 < deleted_end && seg.1 > deleted_start;
        assert!(
            !intersects,
            "segment {seg:?} intersects deleted interval ({deleted_start}, {deleted_end})"
        );
    }
    // And specifically: the outgoing seam ends at the deleted start and
    // the incoming seam begins at the deleted end.
    assert_eq!(segments, vec![(0, deleted_start), (deleted_end, 3_000_000)]);
}

#[test]
fn cursor_mapping_matches_canonical_pipeline() {
    // Locks in the fix to `map_edit_to_source_time`: the default
    // (non-experimental) cursor path must read from the same segments the
    // rendered preview/export audio was concatenated from. If these two
    // ever disagree the cursor and audio drift apart by `Σ guard × seams`.
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
    // Sample a handful of edit-time cursor positions that straddle the
    // internal seam (edit_time 1_000_000 is the seam itself).
    for edit_time_us in [
        0, 250_000, 999_999, 1_000_000, 1_000_001, 1_500_000, 1_999_999,
    ] {
        let via_canonical = map_edit_time_to_source_time_from_segments(edit_time_us, &segments);
        // Derive what the Tauri command returns by calling the same
        // shared helper the command body uses (we can't invoke the
        // #[tauri::command] directly in a unit test without an AppHandle).
        let via_command_body = map_edit_time_to_source_time_from_segments(
            edit_time_us,
            &canonical_keep_segments_for_media(&state, false),
        );
        assert_eq!(
            via_canonical, via_command_body,
            "cursor mapping diverged from canonical pipeline at edit_time_us={edit_time_us}"
        );
    }
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
    let filter = build_audio_concat_filter_with_fade(&[(0, 1_000_000), (2_000_000, 3_000_000)], 0);
    assert!(!filter.contains("afade="));
}

#[test]
fn single_segment_preview_uses_filter_complex_trim_pipeline() {
    let input = Path::new("input.mp4");
    let output = Path::new("preview.m4a");
    let args = build_preview_render_args(input, output, &[(1_000_000, 2_500_000)], &[]);

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
    let args = build_preview_render_args(input, output, &segments, &[]);

    let filter = args
        .windows(2)
        .find(|w| w[0] == "-filter_complex")
        .map(|w| w[1].as_str())
        .expect("missing preview filter");

    assert_eq!(
        filter,
        "[0:a]atrim=start=0.000000:end=1.000000,asetpts=PTS-STARTPTS,afade=t=out:st=0.980000:d=0.020000[a0]; [0:a]atrim=start=2.000000:end=3.500000,asetpts=PTS-STARTPTS,afade=t=in:st=0:d=0.020000[a1]; [a0][a1]concat=n=2:v=0:a=1[outa]"
    );
}

#[test]
fn preview_and_export_share_identical_seam_fade_policy() {
    // Dual-path rule (AGENTS.md) + todo p0-waveform-boundary-policy:
    // given the same input segments, the preview render and the
    // audio-only export path must produce the same filter_complex graph.
    let segments = [(0, 1_000_000), (2_000_000, 3_500_000)];
    let preview_args =
        build_preview_render_args(Path::new("in.mp4"), Path::new("out.m4a"), &segments, &[]);
    let export_args = build_export_args(
        "in.mp4",
        "out.m4a",
        &segments,
        false,
        &default_audio_opts(),
        None,
        None,
        &[],
    );

    let preview_filter = preview_args
        .windows(2)
        .find(|w| w[0] == "-filter_complex")
        .map(|w| w[1].clone())
        .expect("preview must emit -filter_complex");
    let export_filter = export_args
        .windows(2)
        .find(|w| w[0] == "-filter_complex")
        .map(|w| w[1].clone())
        .expect("audio-only export must emit -filter_complex");

    assert_eq!(preview_filter, export_filter);
    // Both paths must reference the unified 20 ms seam fade, not the
    // legacy asymmetric 0 ms / 10 ms split.
    assert!(preview_filter.contains("d=0.020000"));
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

    let reasons = collect_live_validation_failure_reasons(LiveValidationFailureInputs {
        preview_duration_error_us: 250_000,
        export_duration_error_us: 320_000,
        preview_duration_tolerance_us: 180_000,
        export_duration_tolerance_us: 220_000,
        boundary_metric_pass: false,
        seam_metric_pass: false,
        seam_ratios: &[2.0, 24.0],
        seam_max_ratio: 20.0,
        asr_leakage_oracle: &asr_report,
    });

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

    let reasons = collect_live_validation_failure_reasons(LiveValidationFailureInputs {
        preview_duration_error_us: 100_000,
        export_duration_error_us: 150_000,
        preview_duration_tolerance_us: 180_000,
        export_duration_tolerance_us: 220_000,
        boundary_metric_pass: true,
        seam_metric_pass: true,
        seam_ratios: &[0.4, 0.7],
        seam_max_ratio: 20.0,
        asr_leakage_oracle: &asr_report,
    });

    assert!(reasons.is_empty());
}

#[test]
// Kept as a manual-only backend harness invoked by the offline rollout gate
// `scripts/run-local-llm-eval-gate.ps1`. Requires a local media file
// (`TOASTER_LIVE_MEDIA_PATH`) and a local Whisper model file
// (`TOASTER_LIVE_ASR_MODEL_PATH`) — neither is checked into the repo, so
// this can never run as a headless CI gate. See AGENTS.md `eval-harness-
// runner` entry: the CI boundary/parity gates are covered by
// `scripts/eval-audio-boundary.ps1` + `scripts/eval-edit-quality.ps1`.
#[ignore = "requires local media + Whisper model; run via scripts/run-local-llm-eval-gate.ps1"]
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

    let preview_args = build_preview_render_args(&media, &preview_path, &segments, &[]);
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
        None,
        &[],
    );
    run_ffmpeg(&export_args).expect("export render ffmpeg failed");

    let preview_duration_us =
        ffprobe_duration_us(&preview_path).expect("failed to probe preview duration");
    let export_duration_us =
        ffprobe_duration_us(&export_path).expect("failed to probe export duration");

    let preview_duration_error_us = abs_diff_i64(preview_duration_us, expected_keep_duration_us);
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
    let failure_reasons = collect_live_validation_failure_reasons(LiveValidationFailureInputs {
        preview_duration_error_us,
        export_duration_error_us,
        preview_duration_tolerance_us: PREVIEW_DURATION_TOLERANCE_US,
        export_duration_tolerance_us: EXPORT_DURATION_TOLERANCE_US,
        boundary_metric_pass,
        seam_metric_pass,
        seam_ratios: &seam_ratios,
        seam_max_ratio: SEAM_MAX_RATIO,
        asr_leakage_oracle: &asr_leakage_oracle,
    });

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
