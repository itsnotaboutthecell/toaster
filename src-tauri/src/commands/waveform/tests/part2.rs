use super::*;

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
        None,
        &[],
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
        None,
        &[],
    );
    let vf_idx = args.iter().position(|a| a == "-vf");
    assert!(
        vf_idx.is_some(),
        "Single segment video with captions should have -vf flag"
    );
    let filter = &args[vf_idx.unwrap() + 1];
    assert!(
        filter.contains("subtitles="),
        "Filter should contain subtitles directive, got: {filter}"
    );
    assert!(
        !filter.contains("force_style"),
        "New pipeline embeds styling in the ASS document; force_style must be gone, got: {filter}"
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
        None,
        &[],
    );
    assert!(
        args.contains(&"-filter_complex".to_string()),
        "Multi-segment should use filter_complex"
    );
    let fc_idx = args.iter().position(|a| a == "-filter_complex").unwrap();
    let filter = &args[fc_idx + 1];
    assert!(
        filter.contains("concat=n=2"),
        "Should concat 2 segments, got: {filter}"
    );
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
        None,
        &[],
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
    assert_eq!(
        escaped2, "D\\:/test.srt",
        "Colons should be escaped for FFmpeg filter syntax"
    );
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
        None,
        &[],
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
fn test_single_segment_captions_use_fontsdir_when_provided() {
    let args = build_export_args(
        "input.mp4",
        "output.mp4",
        &[(0, 5_000_000)],
        true,
        &default_audio_opts(),
        Some("C:\\path\\to\\captions.ass"),
        Some("C:\\fonts"),
        &[],
    );
    let vf_idx = args.iter().position(|a| a == "-vf").unwrap();
    let filter = &args[vf_idx + 1];
    assert!(
        filter.contains("fontsdir="),
        "Single segment subtitle filter must include fontsdir when bundled fonts available, got: {filter}"
    );
}

#[test]
fn test_multi_segment_captions_use_fontsdir_when_provided() {
    let segments = vec![(0, 2_000_000), (3_000_000, 5_000_000)];
    let args = build_export_args(
        "input.mp4",
        "output.mp4",
        &segments,
        true,
        &default_audio_opts(),
        Some("C:\\captions.ass"),
        Some("C:\\fonts"),
        &[],
    );
    let fc_idx = args.iter().position(|a| a == "-filter_complex").unwrap();
    let filter = &args[fc_idx + 1];
    assert!(
        filter.contains("fontsdir="),
        "Multi-segment subtitle filter must include fontsdir when bundled fonts available, got: {filter}"
    );
}

#[test]
fn test_captions_without_fonts_dir_omit_fontsdir() {
    let args = build_export_args(
        "input.mp4",
        "output.mp4",
        &[(0, 5_000_000)],
        true,
        &default_audio_opts(),
        Some("C:\\captions.ass"),
        None,
        &[],
    );
    let vf_idx = args.iter().position(|a| a == "-vf").unwrap();
    let filter = &args[vf_idx + 1];
    assert!(
        !filter.contains("fontsdir"),
        "Without fonts_dir, fontsdir= should not appear, got: {filter}"
    );
}

// ---- Silenced ranges (p3-resolve-silenced-flag) ----

#[test]
fn silenced_edit_time_ranges_maps_single_segment() {
    // Silenced word fully inside a single keep-segment: edit-time offset
    // is (source_time - seg_start).
    let silenced = [(1_500_000, 2_000_000)];
    let keep = [(1_000_000, 3_000_000)];
    let ranges = silenced_edit_time_ranges(&silenced, &keep);
    assert_eq!(ranges, vec![(500_000, 1_000_000)]);
}

#[test]
fn silenced_edit_time_ranges_drops_ranges_outside_keep() {
    // Silenced word in deleted region (between segments) drops out.
    let silenced = [(2_500_000, 2_800_000)];
    let keep = [(0, 2_000_000), (3_000_000, 4_000_000)];
    let ranges = silenced_edit_time_ranges(&silenced, &keep);
    assert!(ranges.is_empty());
}

#[test]
fn silenced_edit_time_ranges_accumulates_across_keep_segments() {
    // Two silenced words, one per keep-segment. Edit-time for the second
    // keep-segment starts at the total duration of previous segment(s).
    let silenced = [(500_000, 700_000), (3_100_000, 3_400_000)];
    let keep = [(0, 2_000_000), (3_000_000, 4_000_000)];
    let ranges = silenced_edit_time_ranges(&silenced, &keep);
    // First word: 500k..700k in source, seg_start=0, edit=500k..700k.
    // Second word: 3_100k..3_400k, seg_start=3_000k, elapsed=2_000k,
    // edit=2_100k..2_400k.
    assert_eq!(ranges, vec![(500_000, 700_000), (2_100_000, 2_400_000)]);
}

#[test]
fn silenced_edit_time_ranges_merges_adjacent() {
    let silenced = [(0, 500_000), (500_000, 1_000_000)];
    let keep = [(0, 2_000_000)];
    let ranges = silenced_edit_time_ranges(&silenced, &keep);
    assert_eq!(ranges, vec![(0, 1_000_000)]);
}

#[test]
fn build_export_args_emits_volume_gate_for_silenced_word() {
    // Single-segment export with one silenced word in the middle:
    // should emit -af volume=enable='between(t,...)':volume=0.
    let args = build_export_args(
        "input.wav",
        "output.mp3",
        &[(1_000_000, 3_000_000)],
        false,
        &default_audio_opts(),
        None,
        None,
        &[(1_500_000, 2_000_000)],
    );
    let af_idx = args
        .iter()
        .position(|a| a == "-af")
        .expect("expected -af flag for silenced word");
    let filter = &args[af_idx + 1];
    assert!(
        filter.contains("volume=enable='between(t,0.500000,1.000000)':volume=0"),
        "expected volume gate between 0.5s and 1.0s edit-time, got: {filter}"
    );
}

#[test]
fn build_export_args_multi_segment_audio_silences_word() {
    // Multi-segment audio-only export: gate must be chained post-concat
    // using [outa_raw] -> [outa] routing so the edit-time coordinates
    // match the concatenated stream.
    let args = build_export_args(
        "input.wav",
        "output.mp3",
        &[(0, 1_000_000), (2_000_000, 3_000_000)],
        false,
        &default_audio_opts(),
        None,
        None,
        &[(2_400_000, 2_700_000)], // inside second keep-segment
    );
    let fc_idx = args
        .iter()
        .position(|a| a == "-filter_complex")
        .expect("expected filter_complex for multi-segment export");
    let filter = &args[fc_idx + 1];
    assert!(
        filter.contains("[outa_raw]volume=enable='between(t,1.400000,1.700000)':volume=0[outa]"),
        "expected silence gate on post-concat stream, got: {filter}"
    );
}

#[test]
fn preview_and_export_silence_gate_parity() {
    // Dual-path rule: silenced words must produce identical audio filter
    // graphs for preview and audio-only export (same seam fade policy
    // already tested in preview_and_export_share_identical_seam_fade_policy).
    let segments = [(0, 1_000_000), (2_000_000, 3_500_000)];
    let silenced = [(500_000, 800_000), (2_200_000, 2_400_000)];
    let preview_args = build_preview_render_args(
        Path::new("in.mp4"),
        Path::new("out.m4a"),
        &segments,
        &silenced,
    );
    let export_args = build_export_args(
        "in.mp4",
        "out.m4a",
        &segments,
        false,
        &default_audio_opts(),
        None,
        None,
        &silenced,
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
    assert!(preview_filter.contains("volume=enable="));
}

#[test]
fn build_export_args_no_silence_keeps_existing_filter_graph() {
    // Regression: when silenced_source_ranges is empty, the emitted
    // filter graph must be byte-identical to the pre-p3 output.
    let segments = [(0, 1_000_000), (2_000_000, 3_500_000)];
    let args = build_export_args(
        "in.mp4",
        "out.m4a",
        &segments,
        false,
        &default_audio_opts(),
        None,
        None,
        &[],
    );
    let fc_idx = args.iter().position(|a| a == "-filter_complex").unwrap();
    let filter = &args[fc_idx + 1];
    assert!(!filter.contains("volume="));
    assert!(!filter.contains("[outa_raw]"));
}
