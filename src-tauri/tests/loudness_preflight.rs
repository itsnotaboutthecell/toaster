//! Integration test for `compute_loudness_preflight` (AC-002-a).
//!
//! Lives in `src-tauri/tests/` rather than as a `#[cfg(test)] mod tests`
//! inside the lib because an unrelated pre-existing breakage in
//! `commands/transcribe_file/precision_benchmarks.rs` (missing
//! `timing_us_to_sample`, `correct_short_word_boundaries`, etc.) prevents
//! `cargo test -p toaster --lib` from compiling. Integration tests build
//! against the lib without `cfg(test)`, so they bypass that breakage and
//! still exercise the public surface that AC-002-a requires.

use toaster_app_lib::managers::splice::loudness::{
    compute_loudness_preflight, LoudnessTarget,
};

const SR: u32 = 48_000;
const CH: u32 = 1;

fn sine_buffer(seconds: f32, freq_hz: f32, amp: f32) -> Vec<f32> {
    let n = (seconds * SR as f32) as usize;
    let mut out = Vec::with_capacity(n);
    let two_pi = std::f32::consts::TAU;
    for i in 0..n {
        let t = i as f32 / SR as f32;
        out.push(amp * (two_pi * freq_hz * t).sin());
    }
    out
}

#[test]
fn loudness_preflight_roundtrip() {
    // 1 s of 1 kHz sine at -6 dBFS amplitude.
    let buf = sine_buffer(1.0, 1_000.0, 0.5);

    // Off target: integrated should be finite, target/delta should be None.
    let off = compute_loudness_preflight(&buf, SR, CH, LoudnessTarget::Off)
        .expect("preflight off");
    assert!(off.integrated_lufs.is_finite(), "integrated must be finite");
    assert!(off.true_peak_dbtp.is_finite(), "true peak must be finite");
    assert!(off.lra >= 0.0, "lra must be non-negative");
    assert_eq!(off.target_lufs, None);
    assert_eq!(off.delta_lu, None);

    // Podcast target: target = -16, delta_lu = -16 - integrated, computed in Rust.
    let podcast =
        compute_loudness_preflight(&buf, SR, CH, LoudnessTarget::PodcastMinus16)
            .expect("preflight podcast");
    assert_eq!(podcast.target_lufs, Some(-16.0));
    let expected_delta = -16.0 - podcast.integrated_lufs;
    let actual_delta = podcast.delta_lu.expect("delta must be Some when finite");
    assert!(
        (actual_delta - expected_delta).abs() < 1e-9,
        "delta_lu must equal target - integrated, got {} expected {}",
        actual_delta,
        expected_delta,
    );

    // Streaming target: target = -14.
    let streaming =
        compute_loudness_preflight(&buf, SR, CH, LoudnessTarget::StreamingMinus14)
            .expect("preflight streaming");
    assert_eq!(streaming.target_lufs, Some(-14.0));
    assert!(streaming.delta_lu.is_some());

    // Silent input: integrated may be -inf; delta_lu must be None even with
    // an active target (cannot subtract from -inf without surfacing a bogus
    // number to the UI).
    let silent = vec![0.0f32; SR as usize];
    let dto =
        compute_loudness_preflight(&silent, SR, CH, LoudnessTarget::PodcastMinus16)
            .expect("preflight silent");
    assert!(dto.integrated_lufs.is_infinite() || dto.integrated_lufs < -70.0);
    assert_eq!(dto.target_lufs, Some(-16.0));
    if dto.integrated_lufs.is_infinite() {
        assert_eq!(dto.delta_lu, None);
    }
}

#[test]
fn build_loudnorm_filter_off_emits_nothing() {
    use toaster_app_lib::managers::splice::loudness::build_loudnorm_filter;
    assert_eq!(build_loudnorm_filter(LoudnessTarget::Off), None);
    assert_eq!(
        build_loudnorm_filter(LoudnessTarget::PodcastMinus16).as_deref(),
        Some("loudnorm=I=-16:TP=-1.5:LRA=11"),
    );
    assert_eq!(
        build_loudnorm_filter(LoudnessTarget::StreamingMinus14).as_deref(),
        Some("loudnorm=I=-14:TP=-1.5:LRA=11"),
    );
}

/// AC-004-a: the legacy `normalize_audio_on_export` boolean must migrate
/// to a `LoudnessTarget` value, with `true → PodcastMinus16` (preserves
/// the old hard-coded -16 LUFS behavior) and `false → Off`. Mirrors the
/// in-lib unit test that cannot run because of the unrelated
/// `precision_benchmarks.rs` lib-test breakage.
#[test]
fn migrate_loudness_setting_maps_legacy_boolean() {
    use toaster_app_lib::settings::migrate_loudness_setting;

    // Explicit non-default target wins over legacy bool.
    assert_eq!(
        migrate_loudness_setting(Some(false), Some(LoudnessTarget::StreamingMinus14)),
        LoudnessTarget::StreamingMinus14
    );
    // Legacy true -> Podcast (-16).
    assert_eq!(
        migrate_loudness_setting(Some(true), Some(LoudnessTarget::Off)),
        LoudnessTarget::PodcastMinus16
    );
    assert_eq!(
        migrate_loudness_setting(Some(true), None),
        LoudnessTarget::PodcastMinus16
    );
    // Legacy false -> Off.
    assert_eq!(
        migrate_loudness_setting(Some(false), None),
        LoudnessTarget::Off
    );
    // No legacy and no current target -> Off.
    assert_eq!(migrate_loudness_setting(None, None), LoudnessTarget::Off);
    // No legacy, current set -> current preserved.
    assert_eq!(
        migrate_loudness_setting(None, Some(LoudnessTarget::StreamingMinus14)),
        LoudnessTarget::StreamingMinus14
    );
}
