//! AC-002-b: for any audio-only AudioExportFormat the constructed FFmpeg
//! argv must contain `-vn` and must NOT contain `-c:v`.
//!
//! Drives the same `build_export_args` the production export command
//! invokes (via the test-only public wrapper) so we are exercising the
//! single source of truth — not a parallel reimplementation. AGENTS.md
//! "Single source of truth for dual-path logic".

use toaster_app_lib::commands::waveform::{
    build_audio_only_export_args_for_tests, AudioExportFormat,
};

fn args_for(format: AudioExportFormat, segments: &[(i64, i64)]) -> Vec<String> {
    build_audio_only_export_args_for_tests(
        "input.mp4",
        // Use the format's canonical extension so the call matches what
        // the real frontend file picker would produce.
        &format!("output{}", format.extension()),
        segments,
        format,
    )
}

#[test]
fn export_format_args_no_video_stream() {
    let single = vec![(0_i64, 5_000_000_i64)];
    let multi = vec![(0_i64, 2_000_000_i64), (3_000_000, 5_000_000)];

    let cases: &[(AudioExportFormat, &str)] = &[
        (AudioExportFormat::Mp3, "libmp3lame"),
        (AudioExportFormat::Wav, "pcm_s16le"),
        (AudioExportFormat::M4a, "aac"),
        (AudioExportFormat::Opus, "libopus"),
    ];

    for (format, codec) in cases {
        for (label, segments) in
            [("single", &single), ("multi", &multi)]
        {
            let args = args_for(*format, segments);
            assert!(
                args.iter().any(|a| a == "-vn"),
                "{label} {format:?}: argv must contain -vn",
            );
            assert!(
                !args.iter().any(|a| a == "-c:v"),
                "{label} {format:?}: argv must NOT contain -c:v",
            );
            assert!(
                args.windows(2).any(|w| w[0] == "-c:a" && w[1] == *codec),
                "{label} {format:?}: argv must contain -c:a {codec}",
            );
        }
    }
}

#[test]
fn export_format_mp4_keeps_video_codec_when_source_has_video() {
    // Sanity check that the same wrapper, when given Mp4, retains the
    // existing video pipeline behavior (libx264). This guards against
    // accidental regression of the default Mp4 path while we add the
    // audio-only branches.
    let single = vec![(0_i64, 5_000_000_i64)];
    let args = args_for(AudioExportFormat::Mp4, &single);
    assert!(
        args.windows(2).any(|w| w[0] == "-c:v" && w[1] == "libx264"),
        "Mp4 (single segment) must keep libx264; argv={args:?}",
    );
    assert!(
        !args.iter().any(|a| a == "-vn"),
        "Mp4 must NOT force -vn; argv={args:?}",
    );
}
