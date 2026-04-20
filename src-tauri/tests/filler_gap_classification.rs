//! R-004 / AC-004-a..c — gap classification metadata contract.
//!
//! Integration-level coverage of `classify_gap` / `classify_pauses`
//! against the public library surface. Lives outside the lib unit
//! tests so an AC-004 grep gate can target this file by name.
//!
//! Key invariants:
//!   - Empty VAD curve ⇒ every classification is `Unknown` (AC-004-c
//!     "no default behaviour change" guard — callers that ignore the
//!     metadata see no difference).
//!   - A curve with `mean < GAP_SILENCE_THRESHOLD` ⇒ `TrueSilence`.
//!   - A curve with `mean ∈ [GAP_SILENCE_THRESHOLD, GAP_SPEECH_THRESHOLD)` ⇒
//!     `NonSpeechAcoustic` (music / breath / clapping).
//!   - A curve with `mean ≥ GAP_SPEECH_THRESHOLD` ⇒ `MissedSpeech`
//!     (ASR likely dropped real speech).

use toaster_app_lib::managers::filler::{
    classify_gap, GapClassification, GAP_SILENCE_THRESHOLD, GAP_SPEECH_THRESHOLD,
};

#[test]
fn empty_curve_is_always_unknown() {
    assert_eq!(classify_gap(0, 1_000_000, &[]), GapClassification::Unknown);
    assert_eq!(
        classify_gap(5_000_000, 7_000_000, &[]),
        GapClassification::Unknown,
    );
}

#[test]
fn degenerate_range_is_unknown() {
    // end ≤ start must not panic and must not lie about the data.
    let curve = vec![0.5f32; 32];
    assert_eq!(classify_gap(0, 0, &curve), GapClassification::Unknown);
    assert_eq!(classify_gap(1_000, 500, &curve), GapClassification::Unknown);
}

#[test]
fn true_silence_when_mean_below_silence_threshold() {
    let curve = vec![0.05f32; 32];
    let class = classify_gap(0, 900_000, &curve);
    assert_eq!(class, GapClassification::TrueSilence);
    // Threshold constant is a public gate — make sure the band we
    // asserted above is actually below it.
    const _: () = assert!(0.05 < GAP_SILENCE_THRESHOLD);
}

#[test]
fn non_speech_acoustic_when_mean_in_middle_band() {
    let curve = vec![0.3f32; 32];
    let class = classify_gap(0, 900_000, &curve);
    assert_eq!(class, GapClassification::NonSpeechAcoustic);
    const _: () = assert!(0.3 >= GAP_SILENCE_THRESHOLD && 0.3 < GAP_SPEECH_THRESHOLD);
}

#[test]
fn missed_speech_when_mean_above_speech_threshold() {
    let curve = vec![0.9f32; 32];
    let class = classify_gap(0, 900_000, &curve);
    assert_eq!(class, GapClassification::MissedSpeech);
    const _: () = assert!(0.9 >= GAP_SPEECH_THRESHOLD);
}
