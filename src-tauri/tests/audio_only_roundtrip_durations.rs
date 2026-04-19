//! AC-003-a: round-trip duration parity for the four audio-only
//! export formats.
//!
//! Pipeline per format:
//! 1. Build the audio-only FFmpeg argv via the public test wrapper that
//!    delegates to the same `build_export_args` the real export command
//!    uses (single source of truth).
//! 2. Spawn `ffmpeg` to produce the output file.
//! 3. Probe the output duration with `ffprobe`.
//! 4. Assert the probed duration is within +/- 30 ms of the post-edit
//!    keep-segment duration. This is the encoder-pads-silence guard
//!    called out in BLUEPRINT R-003.
//!
//! Marked `#[ignore]` per `coverage.json` so it does not run in the
//! default `cargo test` sweep — invoke explicitly with
//! `cargo test --test audio_only_roundtrip_durations -- --ignored`.
//!
//! Lives in `tests/` (not `src/.../tests/mod.rs`) for the same reason
//! `loudness_preflight.rs` does: an unrelated pre-existing breakage in
//! `commands/transcribe_file/precision_benchmarks.rs` blocks
//! `cargo test --lib` from compiling, but integration tests build
//! against the lib without `cfg(test)`.

use std::path::{Path, PathBuf};
use std::process::Command;

use toaster_app_lib::commands::waveform::{
    build_audio_only_export_args_for_tests, AudioExportFormat,
};

const TOLERANCE_MS: f64 = 30.0;
/// Single keep-segment exercised by the round-trip. 10 s is short
/// enough to keep the test under a few seconds of wall time per format
/// while still being long enough that any encoder-introduced silent
/// padding (the typical AAC priming-sample bug) shows up well above
/// the +/- 30 ms tolerance.
const KEEP_START_US: i64 = 1_000_000;
const KEEP_END_US: i64 = 11_000_000;

fn fixture_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("../eval/fixtures/toaster_example.mp4")
}

fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ffprobe_duration_seconds(path: &Path) -> f64 {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("ffprobe failed to launch");
    assert!(
        out.status.success(),
        "ffprobe failed for {:?}: {}",
        path,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .expect("ffprobe duration parse")
}

fn run_one(format: AudioExportFormat) {
    let fixture = fixture_path();
    assert!(
        fixture.exists(),
        "fixture missing: {} — eval/fixtures/toaster_example.mp4 must be present",
        fixture.display()
    );

    let temp_dir = std::env::temp_dir().join("toaster-audio-only-roundtrip");
    let _ = std::fs::create_dir_all(&temp_dir);
    let out_name = format!(
        "rt-{:?}{}",
        format,
        format.extension(),
    )
    .to_lowercase();
    let out_path = temp_dir.join(out_name);
    let _ = std::fs::remove_file(&out_path);

    let segments = vec![(KEEP_START_US, KEEP_END_US)];
    let args = build_audio_only_export_args_for_tests(
        fixture.to_str().unwrap(),
        out_path.to_str().unwrap(),
        &segments,
        format,
    );

    // Sanity: the args must drop video and pick the right codec.
    assert!(args.iter().any(|a| a == "-vn"), "{:?}: must contain -vn", format);
    assert!(
        !args.iter().any(|a| a == "-c:v"),
        "{:?}: must NOT contain -c:v",
        format
    );

    let result = Command::new("ffmpeg")
        .args(&args)
        .output()
        .expect("ffmpeg failed to launch");
    assert!(
        result.status.success(),
        "{:?} ffmpeg failed: stderr=\n{}\nargs={:?}",
        format,
        String::from_utf8_lossy(&result.stderr),
        args
    );

    let actual_s = ffprobe_duration_seconds(&out_path);
    let expected_s = (KEEP_END_US - KEEP_START_US) as f64 / 1_000_000.0;
    let delta_ms = (actual_s - expected_s).abs() * 1000.0;
    assert!(
        delta_ms <= TOLERANCE_MS,
        "{:?} round-trip duration drift {:.2} ms > {:.0} ms tolerance (expected {:.6}s, got {:.6}s)",
        format,
        delta_ms,
        TOLERANCE_MS,
        expected_s,
        actual_s
    );
}

#[test]
#[ignore = "needs ffmpeg + fixture; run with --ignored"]
fn audio_only_roundtrip_durations() {
    if !ffmpeg_available() {
        eprintln!(
            "ffmpeg not on PATH; skipping audio_only_roundtrip_durations. Install ffmpeg to run."
        );
        return;
    }
    for fmt in [
        AudioExportFormat::Mp3,
        AudioExportFormat::Wav,
        AudioExportFormat::M4a,
        AudioExportFormat::Opus,
    ] {
        run_one(fmt);
    }
}
