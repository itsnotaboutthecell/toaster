//! Zero-crossing snap for splice boundaries.
//!
//! Cuts that land in the middle of voiced audio produce an audible click
//! — a step discontinuity at the splice. Snapping each boundary to the
//! nearest zero-crossing (within a small search window) eliminates the
//! step without changing total duration, without requiring a crossfade,
//! and without pulling deleted content back into the seam.
//!
//! The snap is **microsecond-precise in and out**: callers pass `i64`
//! microsecond timestamps and the decoded sample buffer. Output
//! timestamps are bounded by the input window so time-mapping does not
//! drift across a snap.

/// Default search radius in microseconds.
///
/// 5 ms is smaller than a single pitch period at male fundamentals
/// (f0 ≈ 85 Hz → ~12 ms) but larger than the glottal closure transient
/// at most voicing fundamentals, so we find a zero-crossing in all but
/// pathological cases without crossing into the adjacent phoneme.
pub const DEFAULT_SNAP_RADIUS_US: i64 = 5_000;

/// Default search radius in microseconds for the energy-valley search.
///
/// 20 ms is long enough to span the transition between a voiced phoneme
/// and silence (typical consonant-to-vowel or word-boundary release is
/// 5–15 ms) but short enough that the search cannot leak into an
/// adjacent syllable.
pub const DEFAULT_ENERGY_RADIUS_US: i64 = 20_000;

/// RMS frame length used by the energy-valley search.
///
/// 2 ms at typical sample rates (32 samples at 16 kHz, 88 at 44.1 kHz)
/// is short enough to localize the valley to one pitch period but long
/// enough to reject single-sample noise.
const ENERGY_FRAME_MS: i64 = 2;

/// Snap a single microsecond-granular timestamp to the nearest
/// zero-crossing in `samples` within ±`radius_us`.
///
/// Returns the original timestamp if no zero-crossing exists in the
/// window (silent segment, or the timestamp is past end-of-buffer).
pub fn snap_to_zero_crossing(
    target_us: i64,
    samples: &[f32],
    sample_rate_hz: u32,
    radius_us: i64,
) -> i64 {
    if samples.is_empty() || sample_rate_hz == 0 {
        return target_us;
    }
    let sr = sample_rate_hz as i64;
    let target_sample = us_to_sample(target_us, sr);
    let radius_samples = ((radius_us.abs() * sr) / 1_000_000).max(1);
    let lo = (target_sample - radius_samples).max(0);
    let hi = (target_sample + radius_samples).min(samples.len() as i64 - 1);
    if hi <= lo {
        return target_us;
    }

    let mut best: Option<(i64, i64)> = None;
    let lo_usize = lo as usize;
    let hi_usize = hi as usize;
    for i in lo_usize..hi_usize {
        let a = samples[i];
        let b = samples[i + 1];
        let crosses = (a <= 0.0 && b > 0.0) || (a >= 0.0 && b < 0.0) || a == 0.0;
        if crosses {
            let dist = (i as i64 - target_sample).abs();
            if best.is_none_or(|(d, _)| dist < d) {
                best = Some((dist, i as i64));
            }
        }
    }
    match best {
        Some((_, idx)) => sample_to_us(idx, sr),
        None => target_us,
    }
}

/// Snap every `(start_us, end_us)` pair in `segments` to the nearest
/// zero-crossings within ±`radius_us`, preserving ordering and
/// non-overlap invariants. Pairs where a snap would invert the segment
/// (end ≤ start) fall back to the original bounds.
/// Segment-batch zero-crossing-only snap.
///
/// Preserved for fallback testing and external callers; production
/// preview+export use [`snap_segments_energy_biased`] which subsumes
/// this behaviour in the degenerate (no energy gradient) case.
#[cfg(test)]
pub fn snap_segments(
    segments: &[(i64, i64)],
    samples: &[f32],
    sample_rate_hz: u32,
    radius_us: i64,
) -> Vec<(i64, i64)> {
    let mut out: Vec<(i64, i64)> = Vec::with_capacity(segments.len());
    let mut last_end: i64 = i64::MIN;
    for &(s, e) in segments {
        let snapped_s = snap_to_zero_crossing(s, samples, sample_rate_hz, radius_us);
        let snapped_e = snap_to_zero_crossing(e, samples, sample_rate_hz, radius_us);
        let safe_s = snapped_s.max(last_end);
        let (fs, fe) = if snapped_e > safe_s {
            (safe_s, snapped_e)
        } else {
            (s.max(last_end), e)
        };
        if fe > fs {
            out.push((fs, fe));
            last_end = fe;
        }
    }
    out
}

/// Snap a timestamp to the nearest **energy valley**, then to the
/// nearest zero-crossing within ±`zc_radius_us` of that valley.
///
/// The valley search scans ±`energy_radius_us` around `target_us` and
/// finds the frame with minimum short-window RMS — i.e. the quietest
/// moment nearby. Zero-crossing snap then aligns the phase so the
/// splice introduces no step discontinuity.
///
/// This targets **bleed-through** (tail of a deleted phoneme leaking
/// into the next segment) that pure zero-crossing snap cannot fix:
/// ZC snap prevents clicks but has no notion of which candidate phase
/// is quieter.
pub fn snap_to_energy_valley(
    target_us: i64,
    samples: &[f32],
    sample_rate_hz: u32,
    energy_radius_us: i64,
    zc_radius_us: i64,
) -> i64 {
    if samples.is_empty() || sample_rate_hz == 0 {
        return target_us;
    }
    let sr = sample_rate_hz as i64;
    let target_sample = us_to_sample(target_us, sr);
    let energy_radius_samples = ((energy_radius_us.abs() * sr) / 1_000_000).max(1);
    let frame_len = ((ENERGY_FRAME_MS * sr) / 1_000).max(4) as usize;
    let half_frame = (frame_len / 2) as i64;

    let buf_last = samples.len() as i64 - 1;
    let search_lo = (target_sample - energy_radius_samples).max(half_frame);
    let search_hi = (target_sample + energy_radius_samples).min(buf_last - half_frame);
    if search_hi <= search_lo {
        return snap_to_zero_crossing(target_us, samples, sample_rate_hz, zc_radius_us);
    }

    let mut best_center = target_sample;
    let mut best_energy = f64::INFINITY;
    let mut best_dist = i64::MAX;
    let mut center = search_lo;
    while center <= search_hi {
        let start = (center - half_frame) as usize;
        let end = (center + half_frame) as usize;
        let mut sum_sq = 0f64;
        for &s in &samples[start..end] {
            sum_sq += (s as f64) * (s as f64);
        }
        let dist = (center - target_sample).abs();
        if sum_sq < best_energy - 1e-12
            || ((sum_sq - best_energy).abs() <= 1e-12 && dist < best_dist)
        {
            best_energy = sum_sq;
            best_center = center;
            best_dist = dist;
        }
        center += 1;
    }

    let valley_us = sample_to_us(best_center, sr);
    snap_to_zero_crossing(valley_us, samples, sample_rate_hz, zc_radius_us)
}

/// Segment-batch variant of [`snap_to_energy_valley`].
///
/// Behaves like [`snap_segments`] — preserves ordering, drops inverted
/// segments, clamps against the previous segment's end — but uses the
/// energy-biased snap so the valley (quietest phase near each boundary)
/// wins over the mere nearest zero-crossing.
pub fn snap_segments_energy_biased(
    segments: &[(i64, i64)],
    samples: &[f32],
    sample_rate_hz: u32,
    energy_radius_us: i64,
    zc_radius_us: i64,
) -> Vec<(i64, i64)> {
    let mut out: Vec<(i64, i64)> = Vec::with_capacity(segments.len());
    let mut last_end: i64 = i64::MIN;
    for &(s, e) in segments {
        let snapped_s =
            snap_to_energy_valley(s, samples, sample_rate_hz, energy_radius_us, zc_radius_us);
        let snapped_e =
            snap_to_energy_valley(e, samples, sample_rate_hz, energy_radius_us, zc_radius_us);
        let safe_s = snapped_s.max(last_end);
        let (fs, fe) = if snapped_e > safe_s {
            (safe_s, snapped_e)
        } else {
            (s.max(last_end), e)
        };
        if fe > fs {
            out.push((fs, fe));
            last_end = fe;
        }
    }
    out
}

#[inline]
fn us_to_sample(us: i64, sr: i64) -> i64 {
    (us * sr) / 1_000_000
}

#[inline]
fn sample_to_us(sample: i64, sr: i64) -> i64 {
    (sample * 1_000_000) / sr
}

// ---------------------------------------------------------------------
// R-003 — VAD-biased boundary refinement
// ---------------------------------------------------------------------
//
// See `features/reintroduce-silero-vad/PRD.md` §R-003 and BLUEPRINT §AD-5.
// The VAD-biased snap picks the locally quietest-from-speech-perspective
// candidate within ±`vad_radius_us`, then applies the existing
// zero-crossing phase alignment. `vad_curve` is expected at 30 ms cadence
// and in [0, 1]; any out-of-range or sub-radius curve silently degrades to
// [`snap_to_energy_valley`] so callers never error on missing data
// (graceful fallback per AD-8).
//
// Preview and export consume this through the same function — the
// dual-path SSoT invariant holds because there is only one
// implementation.

/// VAD probability frame cadence (ms). Matches the Silero 30 ms frame
/// cadence used by [`crate::audio_toolkit::vad::prefilter`] so a single
/// curve serves both callers.
pub const VAD_FRAME_MS: i64 = 30;

/// Snap a timestamp to the candidate with the lowest `P(speech)` within
/// ±`vad_radius_us`, then zero-crossing-align to eliminate step
/// discontinuity. Falls back to [`snap_to_energy_valley`] when the
/// curve is empty or does not cover the search window — ensures the
/// disabled path is byte-identical to the energy-only path
/// (AC-003-d guard).
#[allow(dead_code)] // wired by splice manager once settings flag is read.
pub fn snap_to_vad_valley(
    target_us: i64,
    samples: &[f32],
    sample_rate_hz: u32,
    vad_curve: &[f32],
    vad_radius_us: i64,
    zc_radius_us: i64,
) -> i64 {
    if vad_curve.is_empty() || samples.is_empty() || sample_rate_hz == 0 {
        return snap_to_energy_valley(
            target_us,
            samples,
            sample_rate_hz,
            vad_radius_us,
            zc_radius_us,
        );
    }

    let frame_us = VAD_FRAME_MS * 1_000;
    let target_idx = (target_us / frame_us).max(0) as usize;
    let radius_frames = (vad_radius_us.abs() / frame_us).max(1) as usize;
    let curve_end = vad_curve.len();
    let lo = target_idx.saturating_sub(radius_frames);
    let hi = (target_idx + radius_frames).min(curve_end.saturating_sub(1));
    if hi <= lo {
        return snap_to_energy_valley(
            target_us,
            samples,
            sample_rate_hz,
            vad_radius_us,
            zc_radius_us,
        );
    }

    let mut best_idx = target_idx.min(curve_end.saturating_sub(1));
    let mut best_prob = vad_curve.get(best_idx).copied().unwrap_or(1.0);
    let mut best_dist = 0i64;
    for (i, &p) in vad_curve.iter().enumerate().take(hi + 1).skip(lo) {
        let dist = (i as i64 - target_idx as i64).abs();
        if p < best_prob - 1e-6 || ((p - best_prob).abs() <= 1e-6 && dist < best_dist) {
            best_prob = p;
            best_idx = i;
            best_dist = dist;
        }
    }

    let valley_us = best_idx as i64 * frame_us;
    snap_to_zero_crossing(valley_us, samples, sample_rate_hz, zc_radius_us)
}

/// Segment-batch variant of [`snap_to_vad_valley`].
///
/// Matches the invariants of [`snap_segments_energy_biased`] — ordering
/// preserved, inverted segments dropped, `last_end` clamped — but
/// biased by a precomputed VAD curve when one is supplied. If `vad_curve`
/// is empty the function is byte-identical to
/// [`snap_segments_energy_biased`] (AC-003-d).
#[allow(dead_code)] // wired by splice preview/export once settings flag is read.
pub fn snap_segments_vad_biased(
    segments: &[(i64, i64)],
    samples: &[f32],
    sample_rate_hz: u32,
    vad_curve: &[f32],
    vad_radius_us: i64,
    zc_radius_us: i64,
) -> Vec<(i64, i64)> {
    let mut out: Vec<(i64, i64)> = Vec::with_capacity(segments.len());
    let mut last_end: i64 = i64::MIN;
    for &(s, e) in segments {
        let snapped_s = snap_to_vad_valley(
            s,
            samples,
            sample_rate_hz,
            vad_curve,
            vad_radius_us,
            zc_radius_us,
        );
        let snapped_e = snap_to_vad_valley(
            e,
            samples,
            sample_rate_hz,
            vad_curve,
            vad_radius_us,
            zc_radius_us,
        );
        let safe_s = snapped_s.max(last_end);
        let (fs, fe) = if snapped_e > safe_s {
            (safe_s, snapped_e)
        } else {
            (s.max(last_end), e)
        };
        if fe > fs {
            out.push((fs, fe));
            last_end = fe;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn sine(freq_hz: f32, sr: u32, samples: usize, amp: f32) -> Vec<f32> {
        (0..samples)
            .map(|i| amp * (TAU * freq_hz * (i as f32) / sr as f32).sin())
            .collect()
    }

    #[test]
    fn snap_finds_zero_crossing_within_radius() {
        let sr = 16_000u32;
        let buf = sine(100.0, sr, 16_000, 0.8);
        let snapped = snap_to_zero_crossing(2_500, &buf, sr, DEFAULT_SNAP_RADIUS_US);
        // Convert back to sample index; integer truncation can land us one
        // sample before the true snap point, so accept a ±1 window.
        let sample = ((snapped as i64 * sr as i64) / 1_000_000) as usize;
        let window_lo = sample.saturating_sub(1);
        let window_hi = (sample + 2).min(buf.len() - 1);
        let mut found = false;
        for i in window_lo..window_hi {
            let a = buf[i];
            let b = buf[i + 1];
            if (a <= 0.0 && b > 0.0) || (a >= 0.0 && b < 0.0) || a == 0.0 {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "no zero-crossing near snapped us {} (sample {})",
            snapped, sample
        );
    }

    #[test]
    fn snap_returns_input_when_no_crossing_in_radius() {
        let buf = vec![0.5f32; 1_000];
        let t = 2_000i64;
        assert_eq!(snap_to_zero_crossing(t, &buf, 16_000, 5_000), t);
    }

    #[test]
    fn snap_segments_preserves_non_overlap() {
        let sr = 16_000u32;
        let buf = sine(50.0, sr, 16_000, 0.7);
        let input = vec![(100_000, 200_000), (210_000, 400_000)];
        let out = snap_segments(&input, &buf, sr, DEFAULT_SNAP_RADIUS_US);
        assert_eq!(out.len(), 2);
        let (s0, e0) = out[0];
        let (s1, e1) = out[1];
        assert!(s0 < e0);
        assert!(s1 < e1);
        assert!(s1 >= e0, "segments overlap after snap: {e0} > {s1}");
    }

    #[test]
    fn snap_handles_empty_and_tiny_buffers() {
        assert_eq!(snap_to_zero_crossing(1_000, &[], 16_000, 5_000), 1_000);
        assert_eq!(
            snap_to_zero_crossing(1_000, &[0.1, -0.1], 16_000, 5_000),
            0,
        );
    }

    #[test]
    fn snap_does_not_move_a_target_already_on_a_crossing() {
        let sr = 16_000u32;
        let buf = sine(100.0, sr, 16_000, 0.8);
        let snapped = snap_to_zero_crossing(0, &buf, sr, DEFAULT_SNAP_RADIUS_US);
        assert!(snapped.abs() <= 62, "snap drifted off crossing: {snapped}");
    }

    #[test]
    fn energy_valley_prefers_quiet_region_over_loud_region() {
        // 16 kHz, 1 s buffer. Loud sine from 0..8000, silence from 8000..16000.
        let sr = 16_000u32;
        let mut buf = sine(100.0, sr, 8_000, 0.8);
        buf.extend(std::iter::repeat_n(0.0f32, 8_000));
        // Target right at the boundary sample index 8000 → 500_000 us.
        // With a 20 ms energy radius (±320 samples), the valley should
        // land inside the silent region (index > 8000).
        let snapped = snap_to_energy_valley(500_000, &buf, sr, 20_000, 5_000);
        let snapped_sample = ((snapped * sr as i64) / 1_000_000) as usize;
        assert!(
            snapped_sample >= 7_999,
            "energy valley should sit in or near silence, got sample {snapped_sample}"
        );
        assert!(
            snapped_sample <= 8_320,
            "energy valley drifted past silence search window: sample {snapped_sample}"
        );
    }

    #[test]
    fn energy_valley_falls_back_to_zc_when_fully_voiced() {
        // Pure sine everywhere — no energy gradient. Should still land on
        // a zero-crossing (i.e. same behaviour as plain ZC snap).
        let sr = 16_000u32;
        let buf = sine(100.0, sr, 16_000, 0.8);
        let snapped = snap_to_energy_valley(2_500, &buf, sr, 20_000, 5_000);
        let sample = ((snapped * sr as i64) / 1_000_000) as usize;
        let lo = sample.saturating_sub(1);
        let hi = (sample + 2).min(buf.len() - 1);
        let mut found = false;
        for i in lo..hi {
            let a = buf[i];
            let b = buf[i + 1];
            if (a <= 0.0 && b > 0.0) || (a >= 0.0 && b < 0.0) || a == 0.0 {
                found = true;
                break;
            }
        }
        assert!(found, "energy snap landed off a zero-crossing in voiced audio");
    }

    #[test]
    fn energy_biased_segments_preserve_non_overlap() {
        let sr = 16_000u32;
        let mut buf = sine(80.0, sr, 8_000, 0.7);
        buf.extend(std::iter::repeat_n(0.0f32, 4_000));
        buf.extend(sine(80.0, sr, 4_000, 0.7));
        let input = vec![(100_000, 450_000), (550_000, 900_000)];
        let out = snap_segments_energy_biased(&input, &buf, sr, 20_000, 5_000);
        assert_eq!(out.len(), 2);
        let (s0, e0) = out[0];
        let (s1, e1) = out[1];
        assert!(s0 < e0 && s1 < e1);
        assert!(s1 >= e0, "energy-biased segments overlap: {e0} > {s1}");
    }

    // ---------------------------- R-003 --------------------------------

    #[test]
    fn vad_biased_empty_curve_matches_energy_path_exactly() {
        // AC-003-d: with VAD disabled (here simulated by passing an empty
        // curve) the output is byte-identical to the energy-only path.
        let sr = 16_000u32;
        let buf = sine(80.0, sr, 16_000, 0.7);
        let segments = vec![(100_000, 400_000), (450_000, 900_000)];
        let baseline =
            snap_segments_energy_biased(&segments, &buf, sr, 20_000, 5_000);
        let vad_empty = snap_segments_vad_biased(
            &segments, &buf, sr, &[], 20_000, 5_000,
        );
        assert_eq!(baseline, vad_empty);
    }

    #[test]
    fn vad_biased_prefers_low_probability_frame() {
        // 16 frames × 30ms = 480ms of sine wave so zero-crossings exist
        // everywhere. VAD curve is 1.0 except at frame index 7 (~210ms)
        // where a pronounced dip (0.1) should attract the snap.
        let sr = 16_000u32;
        let buf = sine(120.0, sr, 8_000, 0.6);
        let mut curve = vec![1.0f32; 16];
        curve[7] = 0.1;
        // Target near frame 9 (270ms) with ±120ms radius should reach
        // frame 7 (210ms) and prefer it.
        let snapped = snap_to_vad_valley(
            270_000, &buf, sr, &curve, 120_000, 5_000,
        );
        // Expect snap close to frame 7 (210ms) ± one frame.
        assert!(
            (snapped - 210_000).abs() <= 30_000,
            "vad-biased snap did not attract to low-prob frame: snapped={snapped}"
        );
    }
}

