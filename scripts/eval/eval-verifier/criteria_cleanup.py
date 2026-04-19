"""
Criteria for the cleanup / filler-removal verifier.

A "trial" here is a candidate cleanup strategy applied to one fixture: it
produces ``kept_indices`` (which oracle words survive). We score each
candidate against the fixture's ``desired_kept_indices`` plus optional
audio-aware signals from the original WAV.

Criterion IDs are load-bearing — ``backends._mock_score`` dispatches on
them. Keep in sync.
"""

from __future__ import annotations

from typing import Dict, List, Optional, Sequence

from verifier_core import Criterion, Trial

try:  # audio features are optional (depend only on stdlib wave)
    from audio_features import extract_window_features
except Exception:  # pragma: no cover - import failure is treated as "no audio"
    extract_window_features = None  # type: ignore


GROUND_TRUTH_NOTE = (
    "**IMPORTANT:** The ground truth is the fixture's `desired_kept_indices`, "
    "plus Toaster's hard rules: (1) fillers inside quoted spans must NEVER be "
    "deleted, (2) the kept subsequence must remain monotonic in time, "
    "(3) deletions should target audible speech, not pre-existing silence. "
    "A candidate that 'over-deletes' to boost recall but removes content "
    "words, or drops a quoted filler, must score worse than a conservative "
    "candidate that leaves one filler."
)


CRITERIA: List[Criterion] = [
    Criterion(
        id="filler_removal_recall",
        name="Filler Removal Recall",
        description=(
            "Among the non-quoted filler words in the fixture, what fraction "
            "did the candidate remove? Higher is better. Do NOT reward "
            "removing quoted fillers — those are measured separately under "
            "content preservation."
        ),
    ),
    Criterion(
        id="content_preservation",
        name="Content Preservation",
        description=(
            "Of the non-filler (content) words in the fixture, what fraction "
            "did the candidate KEEP? A candidate that deletes content words "
            "must score worse than one that leaves a filler. Deleting a "
            "filler that sits inside a quoted span counts as a content "
            "deletion because the quote wrapper is product-visible text."
        ),
    ),
    Criterion(
        id="timing_monotonicity",
        name="Timing Monotonicity",
        description=(
            "The kept word sequence must be strictly monotonic and non-"
            "overlapping in time. A candidate that produces overlapping or "
            "out-of-order kept words is broken, regardless of its content "
            "accuracy."
        ),
    ),
    Criterion(
        id="deleted_region_audible",
        name="Deleted Region Audibility",
        description=(
            "When audio is available, the regions the candidate deletes "
            "should contain audible speech, not silence. Deleting silence "
            "is a no-op at best and a timing-drift bug at worst. Score "
            "higher for candidates whose deleted ranges have low silence "
            "ratio. If no audio is attached, this criterion is neutral."
        ),
    ),
]


# ---------------------------------------------------------------------------
# Trial construction
# ---------------------------------------------------------------------------


def _analyze(
    fixture: dict,
    candidate: dict,
) -> Dict[str, float]:
    oracle = fixture["oracle_words"]
    kept = list(candidate.get("kept_indices", []))
    kept_set = set(kept)
    total = len(oracle)

    fillers_total = sum(
        1 for w in oracle if w.get("is_filler") and not w.get("in_quote")
    )
    fillers_removed = sum(
        1
        for i, w in enumerate(oracle)
        if w.get("is_filler") and not w.get("in_quote") and i not in kept_set
    )
    content_total = sum(1 for w in oracle if not w.get("is_filler"))
    content_deleted = sum(
        1
        for i, w in enumerate(oracle)
        if not w.get("is_filler") and i not in kept_set
    )
    quote_violations = sum(
        1
        for i, w in enumerate(oracle)
        if w.get("in_quote") and i not in kept_set
    )

    # Monotonicity / overlaps on kept indices (using oracle timings).
    kept_monotonic = True
    kept_overlaps = 0
    last_end = -1
    for idx in kept:
        if idx < 0 or idx >= total:
            kept_monotonic = False
            break
        s = int(oracle[idx]["start_us"])
        e = int(oracle[idx]["end_us"])
        if s < last_end:
            kept_overlaps += 1
        if e <= s:
            kept_monotonic = False
        last_end = e
    # Also: kept indices must be sorted ascending.
    if kept != sorted(kept):
        kept_monotonic = False

    deleted_indices = [i for i in range(total) if i not in kept_set]
    return {
        "fillers_total": fillers_total,
        "fillers_removed": fillers_removed,
        "content_total": content_total,
        "content_deleted": content_deleted,
        "quote_violations": quote_violations,
        "kept_monotonic": kept_monotonic,
        "kept_overlaps": kept_overlaps,
        "deleted_any": len(deleted_indices),
        "deleted_indices": deleted_indices,
    }


def _audio_stats(
    fixture: dict,
    deleted_indices: Sequence[int],
) -> Optional[Dict[str, float]]:
    audio_path = fixture.get("audio_path")
    if not audio_path or extract_window_features is None or not deleted_indices:
        return None
    oracle = fixture["oracle_words"]
    ratios: List[float] = []
    for i in deleted_indices:
        w = oracle[i]
        try:
            feat = extract_window_features(
                audio_path, int(w["start_us"]), int(w["end_us"])
            )
        except Exception:
            return None
        ratios.append(feat.silence_ratio)
    if not ratios:
        return None
    return {
        "mean_deleted_silence_ratio": sum(ratios) / len(ratios),
    }


def build_trial(
    fixture: dict,
    candidate: dict,
) -> Trial:
    stats = _analyze(fixture, candidate)
    audio = _audio_stats(fixture, stats["deleted_indices"])

    trace_lines = [
        f"candidate={candidate['name']}",
        f"fixture={fixture.get('fixture', 'unknown')}",
        f"kept_count={len(candidate.get('kept_indices', []))}",
        f"fillers_total={int(stats['fillers_total'])}",
        f"fillers_removed={int(stats['fillers_removed'])}",
        f"content_total={int(stats['content_total'])}",
        f"content_deleted={int(stats['content_deleted'])}",
        f"quote_violations={int(stats['quote_violations'])}",
        f"kept_monotonic={'true' if stats['kept_monotonic'] else 'false'}",
        f"kept_overlaps={int(stats['kept_overlaps'])}",
        f"deleted_any={int(stats['deleted_any'])}",
    ]
    if audio is not None:
        trace_lines.append("audio_present=true")
        trace_lines.append(
            f"mean_deleted_silence_ratio={audio['mean_deleted_silence_ratio']:.3f}"
        )
    else:
        trace_lines.append("audio_present=false")

    trace_lines.append("")
    trace_lines.append("Kept words (in order):")
    for idx in candidate.get("kept_indices", []):
        if 0 <= idx < len(fixture["oracle_words"]):
            w = fixture["oracle_words"][idx]
            trace_lines.append(
                f"  [{idx}] {w['text']!r}  [{w['start_us']}us -> {w['end_us']}us]"
            )

    problem_lines = [
        f"task=cleanup/{fixture.get('fixture', 'unknown')}",
        f"oracle_word_count={len(fixture['oracle_words'])}",
        f"desired_kept_indices={fixture.get('desired_kept_indices', [])}",
        "Oracle words:",
    ]
    for i, w in enumerate(fixture["oracle_words"]):
        tags = []
        if w.get("is_filler"):
            tags.append("filler")
        if w.get("in_quote"):
            tags.append("quoted")
        tag = f" ({','.join(tags)})" if tags else ""
        problem_lines.append(
            f"  [{i}] {w['text']!r}{tag}  [{w['start_us']}us -> {w['end_us']}us]"
        )

    return Trial(
        trial_name=candidate["name"],
        reward=0.0,
        problem="\n".join(problem_lines),
        trace="\n".join(trace_lines),
    )
