"""Unit tests for audio_features: stdlib WAV window feature extraction."""

from __future__ import annotations

import math
import pathlib
import struct
import sys
import tempfile
import unittest
import wave


HERE = pathlib.Path(__file__).resolve().parent
PKG_DIR = HERE.parent
sys.path.insert(0, str(PKG_DIR))

from audio_features import (  # noqa: E402
    articulation_score,
    extract_window_features,
    seam_discontinuity,
    spectral_clarity,
)


def _write_sine(path, duration_s=1.0, sample_rate=16000, freq=440.0, amp=0.5):
    n = int(duration_s * sample_rate)
    with wave.open(str(path), "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sample_rate)
        frames = bytearray()
        for i in range(n):
            sample = int(amp * 32767 * math.sin(2 * math.pi * freq * i / sample_rate))
            frames += struct.pack("<h", sample)
        wf.writeframes(bytes(frames))


def _write_silence(path, duration_s=1.0, sample_rate=16000):
    n = int(duration_s * sample_rate)
    with wave.open(str(path), "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sample_rate)
        wf.writeframes(b"\x00\x00" * n)


class TestAudioFeatures(unittest.TestCase):
    def test_sine_wave_is_audible(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "sine.wav"
            _write_sine(path)
            feat = extract_window_features(str(path), 0, 1_000_000)
            # -0.5 full-scale sine -> ~-9 dBFS peak, ~-12 dBFS RMS.
            self.assertGreater(feat.peak_dbfs, -12.0)
            self.assertGreater(feat.rms_dbfs, -18.0)
            self.assertLess(feat.silence_ratio, 0.05)
            self.assertGreater(feat.samples, 15000)

    def test_silence_window_reports_silence(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "silent.wav"
            _write_silence(path)
            feat = extract_window_features(str(path), 0, 1_000_000)
            self.assertLess(feat.rms_dbfs, -60.0)
            self.assertGreaterEqual(feat.silence_ratio, 0.99)

    def test_empty_window_is_safe(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "sine.wav"
            _write_sine(path, duration_s=0.2)
            # Request a window past EOF.
            feat = extract_window_features(str(path), 500_000, 600_000)
            self.assertEqual(feat.samples, 0)
            self.assertEqual(feat.silence_ratio, 1.0)

    def test_seam_discontinuity_continuous(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "sine.wav"
            _write_sine(path, duration_s=1.0)
            # Seam at the same point: delta is one-sample sine step.
            d = seam_discontinuity(str(path), 500_000, 500_000, window_us=40_000)
            self.assertIsNotNone(d)
            # One-sample step of a 440 Hz / 0.5 amp sine at 16 kHz is <= ~0.1.
            self.assertLess(d, 0.15)

    def test_spectral_clarity_too_short_returns_neutral(self):
        clar = spectral_clarity([0.1] * 100, 16_000)
        self.assertEqual(clar["score"], 0.5)
        self.assertEqual(clar["frames"], 0)

    def test_spectral_clarity_tonal_signal_scores_higher_than_noise(self):
        sr = 16_000
        n = 4096
        tonal = [int(0.5 * 32767 * math.sin(2 * math.pi * 440 * i / sr)) for i in range(n)]
        # Deterministic LCG noise.
        state = 0xC0FFEE
        noise = []
        for _ in range(n):
            state = (state * 6364136223846793005 + 1442695040888963407) & 0xFFFFFFFFFFFFFFFF
            v = ((state >> 33) & 0x7FFFFFFF) / 0x7FFFFFFF * 2 - 1
            noise.append(int(v * 32767 * 0.5))
        c_tonal = spectral_clarity(tonal, sr)
        c_noise = spectral_clarity(noise, sr)
        self.assertGreater(c_tonal["tonal"], c_noise["tonal"])

    def test_articulation_score_uses_clarity_term(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "sine.wav"
            _write_sine(path, duration_s=0.5, freq=440.0, amp=0.5)
            score = articulation_score(str(path), 0, 500_000)
            # Loud, audible 440 Hz sine: peak/rms terms saturate, clarity
            # term partially contributes (tonal high, hf_ratio low).
            # New v2 weighting must keep this score firmly above 0.5.
            self.assertGreater(score, 0.55)
            self.assertLessEqual(score, 1.0)


if __name__ == "__main__":
    unittest.main()
