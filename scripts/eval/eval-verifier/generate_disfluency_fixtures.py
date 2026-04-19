#!/usr/bin/env python3
"""Synthesize disfluency cleanup fixtures.

Writes both a 16-bit PCM WAV of fake speech ("the the best best part")
and the matching JSON fixture that declares which repetitions are
disfluencies. Used by the disfluency verifier to prove it picks the
clearest survivor from each repetition group.

Stdlib only. Regenerate by running:

    python scripts/eval-verifier/generate_disfluency_fixtures.py
"""

from __future__ import annotations

import json
import math
import pathlib
import struct
import sys
import wave

HERE = pathlib.Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
FIXTURES_DIR = REPO_ROOT / "src-tauri" / "tests" / "fixtures" / "disfluency"

SAMPLE_RATE = 48000


def _sine_burst(freq: float, duration_us: int, amp: float) -> bytes:
    n = int(duration_us * SAMPLE_RATE / 1_000_000)
    # Short linear fades to avoid click artifacts at word boundaries;
    # the test WAV should *not* itself ship with clicks that poison the
    # seam features.
    fade = min(240, n // 8)
    out = bytearray()
    for i in range(n):
        env = 1.0
        if i < fade:
            env = i / fade
        elif i >= n - fade:
            env = (n - i) / fade
        s = int(amp * env * 32767 * math.sin(2 * math.pi * freq * i / SAMPLE_RATE))
        out += struct.pack("<h", s)
    return bytes(out)


def _silence(duration_us: int) -> bytes:
    n = int(duration_us * SAMPLE_RATE / 1_000_000)
    return b"\x00\x00" * n


def _write_wav(path: pathlib.Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(SAMPLE_RATE)
        wf.writeframes(payload)


def build_repeated_the_best() -> None:
    """Build `repeated_the_best.wav` + fixture JSON.

    Words:
      0. "the"  (freq=220 Hz, amp=0.08)   -- MUMBLED repetition
      1. "the"  (freq=220 Hz, amp=0.55)   -- CLEAR survivor
      2. "best" (freq=330 Hz, amp=0.60)   -- CLEAR survivor
      3. "best" (freq=330 Hz, amp=0.10)   -- MUMBLED repetition
      4. "part" (freq=440 Hz, amp=0.55)   -- single clear content word

    Disfluency groups:
      g1: {0, 1} — best survivor is 1 (louder, clearer)
      g2: {2, 3} — best survivor is 2 (louder, clearer)
    """

    schedule = [
        ("the", 220.0, 0.08, 260_000),
        ("gap",   0.0, 0.00,  80_000),
        ("the", 220.0, 0.55, 300_000),
        ("gap",   0.0, 0.00, 120_000),
        ("best", 330.0, 0.60, 340_000),
        ("gap",   0.0, 0.00, 120_000),
        ("best", 330.0, 0.10, 320_000),
        ("gap",   0.0, 0.00, 140_000),
        ("part", 440.0, 0.55, 460_000),
    ]

    payload = bytearray()
    word_entries = []
    cursor_us = 0
    for label, freq, amp, dur in schedule:
        start_us = cursor_us
        if label == "gap":
            payload += _silence(dur)
        else:
            payload += _sine_burst(freq, dur, amp)
            word_entries.append(
                {
                    "text": label,
                    "start_us": start_us,
                    "end_us": start_us + dur,
                    "amp": amp,
                }
            )
        cursor_us += dur

    # Build fixture JSON with group labels + a clarity_hint derived from
    # the synthesized amplitude. The clarity_hint is a documentation
    # aid — the audio-aware criteria read real features from the WAV.
    oracle = []
    group_map = {0: "g1", 1: "g1", 2: "g2", 3: "g2", 4: None}
    for i, w in enumerate(word_entries):
        oracle.append(
            {
                "text": w["text"],
                "start_us": w["start_us"],
                "end_us": w["end_us"],
                "group_id": group_map[i],
                "is_disfluency": group_map[i] is not None,
                "in_quote": False,
                "clarity_hint": round(w["amp"], 2),
            }
        )

    fixture = {
        "_comment": (
            "Disfluency ranker fixture. Two repetition groups: g1 = words [0,1] "
            "('the the'), g2 = words [2,3] ('best best'). Each group has one "
            "clearly articulated survivor and one mumbled one, encoded by the "
            "synthesized amplitude. The audio-aware survivor_clarity criterion "
            "must score candidates that keep {1, 2, 4} higher than candidates "
            "that keep {0, 3, 4}, even though both produce identical word "
            "counts and both collapse each group to exactly one survivor."
        ),
        "fixture": "repeated_the_best",
        "audio_path": "./repeated_the_best.wav",
        "oracle_words": oracle,
        "desired_kept_indices": [1, 2, 4],
        "groups": [
            {"id": "g1", "member_indices": [0, 1]},
            {"id": "g2", "member_indices": [2, 3]},
        ],
        "candidates": [
            {"name": "clear_survivors",   "kept_indices": [1, 2, 4]},
            {"name": "unclear_survivors", "kept_indices": [0, 3, 4]},
            {"name": "mixed_survivors",   "kept_indices": [0, 2, 4]},
            {"name": "no_collapse",       "kept_indices": [0, 1, 2, 3, 4]},
            {"name": "over_collapse",     "kept_indices": [1, 4]},
            {"name": "drops_content",     "kept_indices": [1, 2]},
        ],
        "expected_winner": "clear_survivors",
    }

    wav_path = FIXTURES_DIR / "repeated_the_best.wav"
    fixture_path = FIXTURES_DIR / "repeated_the_best.fixture.json"
    _write_wav(wav_path, bytes(payload))
    with open(fixture_path, "w", encoding="utf-8") as f:
        json.dump(fixture, f, indent=2)
        f.write("\n")
    print(f"wrote {wav_path.relative_to(REPO_ROOT)}")
    print(f"wrote {fixture_path.relative_to(REPO_ROOT)}")


def main() -> int:
    FIXTURES_DIR.mkdir(parents=True, exist_ok=True)
    build_repeated_the_best()
    return 0


if __name__ == "__main__":
    sys.exit(main())
