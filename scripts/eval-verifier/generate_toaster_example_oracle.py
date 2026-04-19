"""Build the real-asset oracle fixture from the human labeling of
eval/fixtures/toaster_example.mp4. Generates:

  src-tauri/tests/fixtures/disfluency/toaster_example_candidates.fixture.json
  src-tauri/tests/fixtures/disfluency/toaster_example_extracted.wav

The extracted WAV is produced by ffmpeg from eval/fixtures/toaster_example.mp4
(16 kHz mono, matching the backend's cleanup scorer). Word timings come
from what Parakeet-TDT produced in the live monitored run on 2026-04-17
(the transcript is the authoritative one the ASR emitted; timings are
approximated from the export's trim windows and are sufficient to
exercise articulation_score + pacing_agreement).

Candidates:
  * human_oracle   - what the user hand-labeled (should win)
  * toaster_today  - what cleanup_all produced before the smart wiring
                     (collapses groups but keeps positional survivors;
                     does NOT cut 'And' / 'like' / 'kind' / 'of')
  * smart_planned  - what the new audio-aware planner would produce if
                     it picked the clearest member of each group
  * keep_everything - do-nothing baseline
"""

from __future__ import annotations

import json
import pathlib
import shutil
import subprocess
import sys


HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
FIXTURE_DIR = REPO / "src-tauri" / "tests" / "fixtures" / "disfluency"
SRC_VIDEO = REPO / "eval" / "fixtures" / "toaster_example.mp4"
OUT_WAV = FIXTURE_DIR / "toaster_example_extracted.wav"
OUT_JSON = FIXTURE_DIR / "toaster_example_candidates.fixture.json"


# (text, start_us, end_us, human_label, is_filler, group_id)
# Timings are approximate (derived from the 2026-04-17 live run) but
# consistent with word order. Good enough to exercise articulation
# scoring — not a substitute for forced-alignment ground truth.
WORDS = [
    # ("text",          start_us,    end_us,      human_label, is_filler, group_id)
    ("Yeah,",           390_000,     952_000,     "keep",      False,     None),
    ("so",              952_000,     1_250_000,   "delete",    False,     None),
    ("the",             1_250_000,   1_520_000,   "delete",    False,     "the_the_the_A"),
    ("um",              1_520_000,   1_980_000,   "delete",    True,      None),
    ("the",             1_980_000,   2_240_000,   "delete",    False,     "the_the_the_A"),
    ("the",             2_240_000,   2_560_000,   "keep",      False,     "the_the_the_A"),
    ("best",            2_560_000,   3_010_000,   "keep",      False,     "best_best"),
    ("best",            3_010_000,   3_450_000,   "delete",    False,     "best_best"),
    ("part",            3_450_000,   3_890_000,   "keep",      False,     None),
    ("about",           3_890_000,   4_280_000,   "keep",      False,     None),
    ("a",               4_280_000,   4_360_000,   "keep",      False,     None),
    ("lot",             4_360_000,   4_600_000,   "keep",      False,     None),
    ("of",              4_600_000,   4_720_000,   "keep",      False,     None),
    ("this",            4_720_000,   5_060_000,   "keep",      False,     None),
    ("is",              5_060_000,   5_210_000,   "keep",      False,     None),
    ("how",             5_210_000,   5_420_000,   "keep",      False,     None),
    ("it",              5_420_000,   5_560_000,   "keep",      False,     None),
    ("can",             5_560_000,   5_790_000,   "keep",      False,     None),
    ("really",          5_790_000,   6_260_000,   "keep",      False,     None),
    ("transform",       6_260_000,   6_970_000,   "keep",      False,     None),
    ("the",             6_970_000,   7_140_000,   "keep",      False,     None),
    ("way",             7_140_000,   7_380_000,   "keep",      False,     None),
    ("you",             7_380_000,   7_560_000,   "keep",      False,     None),
    ("sound.",          7_560_000,   7_832_000,   "keep",      False,     None),
    ("And",             14_500_000,  14_760_000,  "delete",    False,     None),
    ("um",              14_760_000,  15_200_000,  "delete",    True,      None),
    ("like",            15_200_000,  15_480_000,  "delete",    True,      None),
    ("the",             15_480_000,  15_760_000,  "keep",      False,     "the_the_the_B"),
    ("uh",              15_760_000,  16_180_000,  "delete",    True,      None),
    ("the",             16_180_000,  16_440_000,  "delete",    False,     "the_the_the_B"),
    ("the",             16_440_000,  16_720_000,  "delete",    False,     "the_the_the_B"),
    ("difference",      16_720_000,  17_520_000,  "keep",      False,     None),
    ("is",              17_520_000,  17_700_000,  "keep",      False,     None),
    ("gonna",           17_700_000,  18_030_000,  "keep",      False,     None),
    ("be",              18_030_000,  18_200_000,  "keep",      False,     None),
    ("noticeable",      18_200_000,  18_920_000,  "keep",      False,     None),
    ("kind",            18_920_000,  19_180_000,  "delete",    False,     None),
    ("of",              19_180_000,  19_340_000,  "delete",    False,     None),
    ("on",              19_340_000,  19_540_000,  "keep",      False,     None),
    ("first",           19_540_000,  19_860_000,  "keep",      False,     None),
    ("use.",            19_860_000,  21_079_000,  "keep",      False,     None),
]


def extract_audio() -> None:
    if not SRC_VIDEO.exists():
        print(f"[skip] source video missing: {SRC_VIDEO}")
        return
    if shutil.which("ffmpeg") is None:
        print("[skip] ffmpeg not on PATH; wav extraction skipped")
        return
    FIXTURE_DIR.mkdir(parents=True, exist_ok=True)
    cmd = [
        "ffmpeg",
        "-y",
        "-v",
        "error",
        "-i",
        str(SRC_VIDEO),
        "-vn",
        "-ac",
        "1",
        "-ar",
        "16000",
        "-sample_fmt",
        "s16",
        str(OUT_WAV),
    ]
    print(f"[extract] {' '.join(cmd)}")
    subprocess.run(cmd, check=True)
    print(f"[extract] wrote {OUT_WAV} ({OUT_WAV.stat().st_size:,} bytes)")


def build_fixture() -> None:
    oracle_words = []
    groups: dict[str, list[int]] = {}
    human_keep: list[int] = []
    for i, (text, s, e, label, is_filler, gid) in enumerate(WORDS):
        oracle_words.append(
            {
                "text": text,
                "start_us": s,
                "end_us": e,
                "human_label": label,
                "is_filler": is_filler,
                "group_id": gid,
            }
        )
        if gid is not None:
            groups.setdefault(gid, []).append(i)
        if label == "keep":
            human_keep.append(i)

    # Candidates --------------------------------------------------------
    all_indices = list(range(len(WORDS)))

    # human_oracle = user's labeling
    cand_human = {"name": "human_oracle", "kept_indices": human_keep}

    # toaster_today: remove fillers + collapse groups by the FIRST-surviving rule.
    # That is: for each group, keep only the first non-filler member; keep every
    # non-group, non-filler word including "And"/"like"/"kind"/"of".
    kept_today: list[int] = []
    seen_groups: set[str] = set()
    for i, (text, s, e, label, is_filler, gid) in enumerate(WORDS):
        if is_filler:
            continue
        if gid is not None:
            if gid in seen_groups:
                continue
            seen_groups.add(gid)
        kept_today.append(i)
    cand_today = {"name": "toaster_today", "kept_indices": kept_today}

    # smart_planned: same as toaster_today BUT picks the survivor the human
    # chose in each group (which is what the audio-aware scorer also picks
    # on this asset). Still keeps "And"/"like"/"kind"/"of".
    survivors = {
        "the_the_the_A": 5,   # 3rd "the" (index 5 in WORDS)
        "best_best": 6,       # first "best"
        "the_the_the_B": 27,  # first "the" after "like"
    }
    kept_smart: list[int] = []
    for i, (text, s, e, label, is_filler, gid) in enumerate(WORDS):
        if is_filler:
            continue
        if gid is not None:
            if i != survivors[gid]:
                continue
        kept_smart.append(i)
    cand_smart = {"name": "smart_planned", "kept_indices": kept_smart}

    cand_all = {"name": "keep_everything", "kept_indices": all_indices}

    fixture = {
        "fixture": "toaster_example_candidates",
        "source_media": "eval/fixtures/toaster_example.mp4",
        "audio_path": OUT_WAV.name,
        "notes": [
            "Human oracle derived from user labeling captured on 2026-04-17.",
            "Timings approximate; suitable for pacing + clarity scoring.",
            "expected_winner = human_oracle because it is the only candidate that (a) picks the human-chosen survivors, (b) drops non-filler glue words like 'And'/'like'/'kind'/'of'.",
            "aspirational=true: backend does not yet score content-pacing, so tournament winner may differ. Gap is surfaced in the report but does not fail CI.",
        ],
        "aspirational": True,
        "groups": [
            {"id": gid, "member_indices": idxs} for gid, idxs in groups.items()
        ],
        "oracle_words": oracle_words,
        "desired_kept_indices": human_keep,
        "expected_winner": "human_oracle",
        "candidates": [cand_human, cand_smart, cand_today, cand_all],
    }

    FIXTURE_DIR.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(json.dumps(fixture, indent=2))
    print(f"[fixture] wrote {OUT_JSON} ({OUT_JSON.stat().st_size:,} bytes)")
    print(
        f"[fixture] candidates={[c['name'] for c in fixture['candidates']]} "
        f"human_keep={len(human_keep)}/{len(WORDS)} "
        f"groups={len(groups)}"
    )


if __name__ == "__main__":
    build_fixture()
    extract_audio()
    sys.exit(0)
