//! EBU R128 / LUFS loudness measurement and target-gain computation.
//!
//! Wraps the `ebur128` crate (MIT) to provide a deterministic,
//! single-pass alternative to FFmpeg's two-pass `loudnorm` string
//! filter. The output is a measured integrated LUFS value and a simple
//! dB gain offset to reach a target loudness. The gain can then be
//! applied via a trivial FFmpeg `volume=XdB` or a software multiply.
//!
//! Callers provide already-decoded PCM samples and a sample rate; no
//! I/O, no shelling out. This keeps the "single source of truth for
//! dual-path logic" invariant — preview and export can call the same
//! function on the same buffer and will get bit-identical gain.

use ebur128::{EbuR128, Error as EbuError, Mode};
use serde::{Deserialize, Serialize};
use specta::Type;

/// User-facing loudness target enum. Single source of truth for the
/// `loudnorm` filter parameters used by the export pipeline; the
/// frontend only stores/sends this enum and never builds a filter
/// string itself (AGENTS.md "Single source of truth for dual-path
/// logic"). See `build_loudnorm_filter` for the mapping.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum LoudnessTarget {
    /// No `loudnorm` filter is emitted; export passes audio through.
    Off,
    /// Podcast / broadcast preset: integrated -16 LUFS.
    #[serde(rename = "podcast_-16")]
    PodcastMinus16,
    /// Streaming preset (Spotify/YouTube-friendly): integrated -14 LUFS.
    #[serde(rename = "streaming_-14")]
    StreamingMinus14,
}

impl Default for LoudnessTarget {
    fn default() -> Self {
        Self::Off
    }
}

impl LoudnessTarget {
    /// The integrated-LUFS target (`I=` parameter), if any.
    pub fn target_lufs(self) -> Option<f64> {
        match self {
            Self::Off => None,
            Self::PodcastMinus16 => Some(-16.0),
            Self::StreamingMinus14 => Some(-14.0),
        }
    }
}

/// Build the `loudnorm` FFmpeg filter string for the given target, or
/// `None` when normalization is off.
///
/// This is the single Rust authority for the filter parameters; the
/// frontend MUST NOT hand-build a `loudnorm=...` string. AGENTS.md
/// "Single source of truth for dual-path logic" — preview/preflight
/// and export consume the same struct/string.
pub fn build_loudnorm_filter(target: LoudnessTarget) -> Option<String> {
    target
        .target_lufs()
        .map(|i| format!("loudnorm=I={}:TP=-1.5:LRA=11", format_target(i)))
}

fn format_target(i: f64) -> String {
    // Print integers without a trailing ".0" so the filter string matches
    // FFmpeg's expected canonical form ("I=-16", "I=-14").
    if (i.fract()).abs() < f64::EPSILON {
        format!("{}", i as i64)
    } else {
        format!("{i}")
    }
}

/// Broadcast / streaming-friendly default target loudness.
///
/// -16 LUFS matches the existing `loudnorm=I=-16` filter string in
/// `commands/waveform/mod.rs`, so swapping ebur128-based gain in does
/// not change user-perceived loudness.
pub const DEFAULT_TARGET_LUFS: f64 = -16.0;

/// A measured loudness report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessReport {
    /// Integrated loudness across the whole buffer, LUFS.
    pub integrated_lufs: f64,
    /// True-peak across the whole buffer, dBTP (max across channels).
    pub true_peak_dbtp: f64,
    /// Channel count observed.
    pub channels: u32,
    /// Sample rate observed (Hz).
    pub sample_rate_hz: u32,
}

/// Preflight DTO surfaced to the frontend. All fields are computed
/// here in Rust — the React side reads and formats them but performs
/// **no arithmetic** on LUFS / dBTP / LRA values (AGENTS.md
/// "Single source of truth for dual-path logic"; AC-006-a).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Type)]
pub struct LoudnessPreflight {
    /// Integrated loudness across the analyzed buffer, LUFS. May be
    /// `f64::NEG_INFINITY` for silent input — formatters should detect
    /// non-finite values.
    pub integrated_lufs: f64,
    /// True-peak across the analyzed buffer, dBTP (max across
    /// channels). May be `f64::NEG_INFINITY` for silent input.
    pub true_peak_dbtp: f64,
    /// EBU R128 loudness range (LRA), in LU.
    pub lra: f64,
    /// The integrated-LUFS target for the selected `LoudnessTarget`,
    /// or `None` when the target is `Off`.
    pub target_lufs: Option<f64>,
    /// `target_lufs - integrated_lufs` in LU when both are finite,
    /// otherwise `None`. Computed in Rust so the frontend cannot
    /// re-derive it (AC-006-a).
    pub delta_lu: Option<f64>,
}

/// Compute the preflight DTO from already-decoded interleaved f32 PCM.
///
/// This is the single Rust authority for the preflight numbers; the
/// Tauri command in `commands/waveform/commands.rs::loudness_preflight`
/// is a thin wrapper that decodes audio and calls this helper.
/// AC-002-a is verified by `loudness_preflight_roundtrip` below.
pub fn compute_loudness_preflight(
    samples: &[f32],
    sample_rate_hz: u32,
    channels: u32,
    target: LoudnessTarget,
) -> Result<LoudnessPreflight, EbuError> {
    let mut meter = EbuR128::new(
        channels,
        sample_rate_hz,
        Mode::I | Mode::TRUE_PEAK | Mode::LRA,
    )?;
    if !samples.is_empty() {
        meter.add_frames_f32(samples)?;
    }
    let integrated = meter.loudness_global().unwrap_or(f64::NEG_INFINITY);
    let lra = meter.loudness_range().unwrap_or(0.0);
    let mut peak = f64::NEG_INFINITY;
    for ch in 0..channels {
        if let Ok(p) = meter.true_peak(ch) {
            let db = if p > 0.0 {
                20.0 * p.log10()
            } else {
                f64::NEG_INFINITY
            };
            if db > peak {
                peak = db;
            }
        }
    }
    let target_lufs = target.target_lufs();
    let delta_lu = target_lufs.and_then(|t| {
        if integrated.is_finite() {
            Some(t - integrated)
        } else {
            None
        }
    });
    Ok(LoudnessPreflight {
        integrated_lufs: integrated,
        true_peak_dbtp: peak,
        lra,
        target_lufs,
        delta_lu,
    })
}

/// Measure integrated loudness and true-peak for interleaved f32
/// samples.
///
/// `samples` is interleaved (L,R,L,R,...) when `channels > 1`.
pub fn measure_loudness(
    samples: &[f32],
    sample_rate_hz: u32,
    channels: u32,
) -> Result<LoudnessReport, EbuError> {
    let mut meter = EbuR128::new(channels, sample_rate_hz, Mode::I | Mode::TRUE_PEAK)?;
    if !samples.is_empty() {
        meter.add_frames_f32(samples)?;
    }
    let integrated = meter.loudness_global().unwrap_or(f64::NEG_INFINITY);
    let mut peak = f64::NEG_INFINITY;
    for ch in 0..channels {
        if let Ok(p) = meter.true_peak(ch) {
            let db = if p > 0.0 { 20.0 * p.log10() } else { f64::NEG_INFINITY };
            if db > peak {
                peak = db;
            }
        }
    }
    Ok(LoudnessReport {
        integrated_lufs: integrated,
        true_peak_dbtp: peak,
        channels,
        sample_rate_hz,
    })
}

/// Compute the dB gain needed to move a measured integrated loudness to
/// a target, clipped to a sane range so a silent buffer doesn't produce
/// an infinite gain.
///
/// `max_boost_db` caps the upward gain — applying more than +18 dB to a
/// quiet track almost always amplifies noise more than signal.
pub fn target_gain_db(measured_lufs: f64, target_lufs: f64, max_boost_db: f64) -> f64 {
    if !measured_lufs.is_finite() {
        return 0.0;
    }
    let raw = target_lufs - measured_lufs;
    raw.clamp(-60.0, max_boost_db)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn sine(freq_hz: f32, sr: u32, seconds: f32, amp: f32) -> Vec<f32> {
        let n = (sr as f32 * seconds) as usize;
        (0..n)
            .map(|i| amp * (TAU * freq_hz * (i as f32) / sr as f32).sin())
            .collect()
    }

    #[test]
    fn silent_buffer_reports_neg_inf_loudness() {
        let sr = 48_000u32;
        let buf = vec![0.0f32; (sr as usize) * 3];
        let r = measure_loudness(&buf, sr, 1).expect("measure");
        assert!(r.integrated_lufs.is_infinite() && r.integrated_lufs < 0.0);
        assert_eq!(target_gain_db(r.integrated_lufs, -16.0, 18.0), 0.0);
    }

    #[test]
    fn steady_sine_has_stable_lufs() {
        let sr = 48_000u32;
        let buf = sine(1_000.0, sr, 5.0, 0.1);
        let a = measure_loudness(&buf, sr, 1).expect("measure a");
        let b = measure_loudness(&buf, sr, 1).expect("measure b");
        assert!(a.integrated_lufs.is_finite());
        assert!(a.integrated_lufs < 0.0);
        assert_eq!(
            a.integrated_lufs, b.integrated_lufs,
            "measurement must be deterministic"
        );
        assert!(a.true_peak_dbtp > -40.0, "peak {} too low", a.true_peak_dbtp);
    }

    #[test]
    fn target_gain_reaches_target_when_applied_in_linear_domain() {
        let sr = 48_000u32;
        let buf = sine(1_000.0, sr, 5.0, 0.1);
        let measured = measure_loudness(&buf, sr, 1).unwrap().integrated_lufs;
        let target = -18.0;
        let gain_db = target_gain_db(measured, target, 18.0);
        let gain_lin = 10f32.powf((gain_db as f32) / 20.0);
        let applied: Vec<f32> = buf.iter().map(|s| s * gain_lin).collect();
        let after = measure_loudness(&applied, sr, 1).unwrap().integrated_lufs;
        assert!(
            (after - target).abs() < 0.25,
            "after={after} target={target} gain_db={gain_db}"
        );
    }

    #[test]
    fn build_loudnorm_filter_emits_target_strings() {
        // AC-003-a: enum -> filter string is the single source of truth.
        assert_eq!(
            build_loudnorm_filter(LoudnessTarget::PodcastMinus16),
            Some("loudnorm=I=-16:TP=-1.5:LRA=11".to_string())
        );
        assert_eq!(
            build_loudnorm_filter(LoudnessTarget::StreamingMinus14),
            Some("loudnorm=I=-14:TP=-1.5:LRA=11".to_string())
        );
        assert_eq!(build_loudnorm_filter(LoudnessTarget::Off), None);
    }

    #[test]
    fn loudness_target_serde_uses_kebab_with_lufs_suffix() {
        let off = serde_json::to_string(&LoudnessTarget::Off).unwrap();
        let podcast = serde_json::to_string(&LoudnessTarget::PodcastMinus16).unwrap();
        let streaming = serde_json::to_string(&LoudnessTarget::StreamingMinus14).unwrap();
        assert_eq!(off, "\"off\"");
        assert_eq!(podcast, "\"podcast_-16\"");
        assert_eq!(streaming, "\"streaming_-14\"");

        let round: LoudnessTarget = serde_json::from_str("\"podcast_-16\"").unwrap();
        assert_eq!(round, LoudnessTarget::PodcastMinus16);
    }

    #[test]
    fn target_gain_is_clamped() {
        assert_eq!(target_gain_db(f64::NEG_INFINITY, -16.0, 18.0), 0.0);
        let g = target_gain_db(-3.0, -16.0, 18.0);
        assert!((g - (-13.0)).abs() < 0.01);
        assert_eq!(target_gain_db(-60.0, -16.0, 18.0), 18.0);
    }

    #[test]
    fn loudness_preflight_roundtrip() {
        // AC-002-a: pure-function round-trip. Synthesize a deterministic
        // 5 s 1 kHz sine at -20 dBFS, verify the DTO populates every
        // field, and that delta_lu is computed by the backend (not the
        // frontend) when a target is selected.
        let sr = 48_000u32;
        let amp = 10f32.powf(-20.0 / 20.0);
        let buf = sine(1_000.0, sr, 5.0, amp);

        let off = compute_loudness_preflight(&buf, sr, 1, LoudnessTarget::Off)
            .expect("preflight off");
        assert!(off.integrated_lufs.is_finite());
        assert!(off.integrated_lufs < 0.0);
        assert!(off.true_peak_dbtp.is_finite());
        assert!(off.lra.is_finite());
        assert_eq!(off.target_lufs, None);
        assert_eq!(off.delta_lu, None);

        let podcast = compute_loudness_preflight(&buf, sr, 1, LoudnessTarget::PodcastMinus16)
            .expect("preflight podcast");
        assert_eq!(podcast.target_lufs, Some(-16.0));
        let delta = podcast.delta_lu.expect("delta_lu must be Some");
        assert!(
            (delta - (-16.0 - podcast.integrated_lufs)).abs() < 1e-9,
            "delta_lu must be target - integrated"
        );

        let streaming =
            compute_loudness_preflight(&buf, sr, 1, LoudnessTarget::StreamingMinus14)
                .expect("preflight streaming");
        assert_eq!(streaming.target_lufs, Some(-14.0));

        // Silent buffer: delta_lu must be None even when target is set
        // (we cannot meaningfully subtract from -inf).
        let silent = vec![0.0f32; (sr as usize) * 1];
        let silent_dto = compute_loudness_preflight(&silent, sr, 1, LoudnessTarget::PodcastMinus16)
            .expect("preflight silent");
        assert!(silent_dto.integrated_lufs.is_infinite());
        assert_eq!(silent_dto.delta_lu, None);
        assert_eq!(silent_dto.target_lufs, Some(-16.0));
    }
}
