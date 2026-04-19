"""
Stdlib-only audio feature extraction over WAV windows.

Used by the cleanup verifier to ask "did this candidate delete an audible
region or a silent gap?" without pulling in numpy/scipy. Works for 16-bit
PCM WAVs — which is what Toaster's parity and boundary fixtures already
emit.

Features provided over a [start_us, end_us] window:

  * ``rms_dbfs``     — overall RMS in dBFS (-inf..0)
  * ``peak_dbfs``    — max absolute sample in dBFS
  * ``silence_ratio``— fraction of 10 ms sub-windows below -50 dBFS
  * ``samples``      — number of samples actually read
  * ``duration_us``  — duration of the window (clamped to file length)

These are enough to score:

  * "a 'good' filler deletion should remove audible speech, not silence"
    → penalize candidates whose deleted regions have high silence_ratio.
  * "a seam should butt up against speech on both sides"
    → compare rms_dbfs of the 40 ms window before vs after a splice.

No external deps. Tested via ``tests/test_audio_features.py``.
"""

from __future__ import annotations

import math
import wave
from dataclasses import dataclass
from typing import Optional

_SILENCE_DBFS = -50.0
_SUBWINDOW_MS = 10


@dataclass
class WindowFeatures:
    rms_dbfs: float
    peak_dbfs: float
    silence_ratio: float
    samples: int
    duration_us: int


def _dbfs(linear: float) -> float:
    if linear <= 0:
        return -120.0
    return 20.0 * math.log10(linear)


def _read_window_samples(wav_path: str, start_us: int, end_us: int):
    """Return (samples, sample_rate, channels) for the requested window.

    Samples are returned as a list of ints in the first-channel stream (mono
    if mono, left channel if stereo). Out-of-range requests are clamped.
    """

    with wave.open(wav_path, "rb") as wf:
        sr = wf.getframerate()
        ch = wf.getnchannels()
        sw = wf.getsampwidth()
        n_frames = wf.getnframes()

        if sw != 2:
            raise ValueError(
                f"{wav_path}: only 16-bit PCM supported (got sampwidth={sw})"
            )

        start_frame = max(0, int(start_us * sr / 1_000_000))
        end_frame = min(n_frames, int(end_us * sr / 1_000_000))
        if end_frame <= start_frame:
            return [], sr, ch

        wf.setpos(start_frame)
        raw = wf.readframes(end_frame - start_frame)

    # 16-bit little-endian PCM. Walk bytes manually to avoid struct overhead
    # on very large reads (this is stdlib-only on purpose).
    samples = []
    step = 2 * ch
    for i in range(0, len(raw), step):
        lo = raw[i]
        hi = raw[i + 1]
        s = lo | (hi << 8)
        if s & 0x8000:
            s -= 0x10000
        samples.append(s)
    return samples, sr, ch


def extract_window_features(
    wav_path: str,
    start_us: int,
    end_us: int,
) -> WindowFeatures:
    samples, sr, _ch = _read_window_samples(wav_path, start_us, end_us)
    return _features_from_samples(samples, sr)


def _features_from_samples(samples, sr: int) -> WindowFeatures:
    n = len(samples)
    if n == 0:
        return WindowFeatures(
            rms_dbfs=-120.0,
            peak_dbfs=-120.0,
            silence_ratio=1.0,
            samples=0,
            duration_us=0,
        )

    peak = 0
    sq = 0.0
    for s in samples:
        a = abs(s)
        if a > peak:
            peak = a
        sq += float(s) * float(s)
    rms = math.sqrt(sq / n) / 32768.0
    peak_lin = peak / 32768.0
    rms_db = _dbfs(rms)
    peak_db = _dbfs(peak_lin)

    sub_n = max(1, int(sr * _SUBWINDOW_MS / 1000))
    silent_subs = 0
    total_subs = 0
    for offset in range(0, n, sub_n):
        chunk = samples[offset : offset + sub_n]
        if not chunk:
            break
        sq2 = 0.0
        for s in chunk:
            sq2 += float(s) * float(s)
        rms2 = math.sqrt(sq2 / len(chunk)) / 32768.0
        if _dbfs(rms2) < _SILENCE_DBFS:
            silent_subs += 1
        total_subs += 1
    silence_ratio = silent_subs / total_subs if total_subs else 1.0

    duration_us = int(n * 1_000_000 / sr)

    return WindowFeatures(
        rms_dbfs=rms_db,
        peak_dbfs=peak_db,
        silence_ratio=silence_ratio,
        samples=n,
        duration_us=duration_us,
    )


# ---------------------------------------------------------------------------
# Spectral clarity (mirror of managers::splice::clarity in Rust).
# ---------------------------------------------------------------------------

_FFT_SIZE = 512
_HOP_SIZE = 256
_HF_HZ = 2_000.0


def _hann_window(n: int):
    return [0.5 * (1.0 - math.cos(2.0 * math.pi * i / (n - 1))) for i in range(n)]


def _fft_iterative(x):
    """Iterative radix-2 Cooley–Tukey FFT. Length must be a power of 2.

    Stdlib-only, operates on a list of complex numbers in place.
    O(N log N), fine for FFT_SIZE=512.
    """

    n = len(x)
    # Bit-reversal permutation.
    j = 0
    for i in range(1, n):
        bit = n >> 1
        while j & bit:
            j ^= bit
            bit >>= 1
        j |= bit
        if i < j:
            x[i], x[j] = x[j], x[i]

    size = 2
    while size <= n:
        half = size // 2
        # Twiddle factor for this stage.
        w_step = complex(math.cos(-2.0 * math.pi / size), math.sin(-2.0 * math.pi / size))
        for start in range(0, n, size):
            w = complex(1.0, 0.0)
            for k in range(half):
                a = x[start + k]
                b = x[start + k + half] * w
                x[start + k] = a + b
                x[start + k + half] = a - b
                w *= w_step
        size <<= 1


def spectral_clarity(samples, sr: int) -> dict:
    """Return {tonal, hf_ratio, centroid_motion, score, frames}.

    Mirrors `managers::splice::clarity::analyze` byte-for-byte in formula
    (FFT_SIZE 512, HOP_SIZE 256, Hann window, HF cutoff 2 kHz, equal-
    weighted mean of three features). Returns neutral 0.5 when the input
    is shorter than one frame so very short words score sensibly.
    """

    neutral = {"tonal": 0.5, "hf_ratio": 0.5, "centroid_motion": 0.5, "score": 0.5, "frames": 0}
    if len(samples) < _FFT_SIZE or sr <= 0:
        return neutral

    window = _hann_window(_FFT_SIZE)
    hf_bin = int(_HF_HZ * _FFT_SIZE / sr)
    half = _FFT_SIZE // 2

    flatness_accum = 0.0
    hf_ratio_accum = 0.0
    centroids = []
    frames = 0

    frame_start = 0
    while frame_start + _FFT_SIZE <= len(samples):
        buf = [
            complex(float(samples[frame_start + i]) * window[i], 0.0)
            for i in range(_FFT_SIZE)
        ]
        _fft_iterative(buf)

        # Single-sided magnitude spectrum, skip bin 0 (DC).
        mags = [abs(buf[bin_]) for bin_ in range(1, half)]

        total_energy = sum(m * m for m in mags)
        if total_energy <= 1e-30:
            frame_start += _HOP_SIZE
            continue

        # Spectral flatness: geo_mean / arith_mean.
        log_sum = 0.0
        arith_sum = 0.0
        for m in mags:
            mm = m if m > 1e-12 else 1e-12
            log_sum += math.log(mm)
            arith_sum += mm
        n_mags = len(mags)
        geo = math.exp(log_sum / n_mags)
        arith = arith_sum / n_mags
        flatness = (geo / arith) if arith > 0 else 1.0

        # HF / total energy. Predicate `bin + 1 >= hf_bin` because we
        # stripped DC, so mags index `i` corresponds to FFT bin `i+1`.
        hf_energy = 0.0
        for i, m in enumerate(mags):
            if i + 1 >= hf_bin:
                hf_energy += m * m
        hf_ratio = max(0.0, min(1.0, hf_energy / total_energy))

        # Centroid in bins.
        weighted = sum(i * m for i, m in enumerate(mags))
        denom = sum(mags)
        centroid = weighted / denom if denom > 0 else 0.0

        flatness_accum += flatness
        hf_ratio_accum += hf_ratio
        centroids.append(centroid)
        frames += 1
        frame_start += _HOP_SIZE

    if frames == 0:
        return neutral

    mean_flatness = flatness_accum / frames
    tonal = max(0.0, min(1.0, 1.0 - mean_flatness))
    hf_ratio = hf_ratio_accum / frames

    mean_centroid = sum(centroids) / len(centroids)
    var = sum((c - mean_centroid) ** 2 for c in centroids) / len(centroids)
    stddev = math.sqrt(var)
    nyquist_bins = _FFT_SIZE / 2
    centroid_motion = max(0.0, min(1.0, stddev / (nyquist_bins * 0.25)))

    score = max(0.0, min(1.0, (tonal + hf_ratio + centroid_motion) / 3.0))

    return {
        "tonal": tonal,
        "hf_ratio": hf_ratio,
        "centroid_motion": centroid_motion,
        "score": score,
        "frames": frames,
    }


def seam_discontinuity(
    wav_path: str,
    a_end_us: int,
    b_start_us: int,
    window_us: int = 40_000,
) -> Optional[float]:
    """Measure the acoustic discontinuity across a splice.

    Returns the absolute sample-value delta across the seam, normalized by
    16-bit full-scale (so 0.0 = perfectly continuous, 1.0 = max jump).
    Useful for evaluating the *quality* of a cut, not just its location.
    Returns ``None`` if either window is empty.
    """

    a_feat_samples, _sr, _ch = _read_window_samples(
        wav_path, max(0, a_end_us - window_us), a_end_us
    )
    b_feat_samples, _sr2, _ch2 = _read_window_samples(
        wav_path, b_start_us, b_start_us + window_us
    )
    if not a_feat_samples or not b_feat_samples:
        return None
    # Compare the last sample before the cut to the first sample after.
    delta = abs(a_feat_samples[-1] - b_feat_samples[0]) / 32768.0
    return min(1.0, delta)


# ---------------------------------------------------------------------------
# Disfluency-aware features
# ---------------------------------------------------------------------------


def articulation_score(wav_path: str, start_us: int, end_us: int) -> float:
    """Derive a [0, 1] articulation score for a word-sized window.

    The v2 score combines four signals (mirroring the Rust
    ``managers::disfluency::score_word`` formula one-for-one — if you
    change one side you MUST change the other in the same commit):

      * peak_dbfs — usable peak level (-40 dBFS = 0, -6 dBFS = 1).
      * rms_dbfs — usable RMS level (-50 dBFS = 0, -15 dBFS = 1).
      * silence_ratio — fraction of window above the silence floor.
      * spectral.score — equal-weighted mean of tonal (Wiener entropy
        inverse), HF-ratio (>= 2 kHz energy / total), and centroid
        motion (stddev of spectral centroid across frames).

    Weights: 0.40 * peak + 0.25 * rms + 0.15 * (1-silence)
             + 0.20 * spectral.

    Spectral analysis uses FFT size 512 with 50% hop and a Hann
    window. Inputs shorter than one frame degrade to a neutral 0.5
    spectral score, matching the Rust ``SpectralClarity::neutral()``
    fallback.
    """

    samples, sr, _ch = _read_window_samples(wav_path, start_us, end_us)
    feat = _features_from_samples(samples, sr)
    if feat.samples == 0:
        return 0.0

    # Peak term: -40 dBFS or quieter -> 0; -6 dBFS or louder -> 1.
    peak_term = (feat.peak_dbfs + 40.0) / 34.0
    peak_term = max(0.0, min(1.0, peak_term))

    # RMS term: -50 dBFS -> 0; -15 dBFS -> 1.
    rms_term = (feat.rms_dbfs + 50.0) / 35.0
    rms_term = max(0.0, min(1.0, rms_term))

    # Silence penalty: a window that's mostly silent can't be articulate.
    silence_penalty = 1.0 - feat.silence_ratio

    spectral = spectral_clarity(samples, sr)

    score = (
        0.40 * peak_term
        + 0.25 * rms_term
        + 0.15 * silence_penalty
        + 0.20 * spectral["score"]
    )
    return max(0.0, min(1.0, score))


def seam_silence_ratio(
    wav_path: str,
    cut_us: int,
    window_us: int = 40_000,
) -> float:
    """Silence ratio of a symmetric window centered on a splice point.

    A clean cut lands in silence (ratio near 1.0); a click-prone cut
    lands in voiced speech (ratio near 0.0). Returns 0.0 if the window
    is empty (e.g. cut past EOF).
    """

    half = max(1, window_us // 2)
    feat = extract_window_features(
        wav_path, max(0, cut_us - half), cut_us + half
    )
    if feat.samples == 0:
        return 0.0
    return feat.silence_ratio
