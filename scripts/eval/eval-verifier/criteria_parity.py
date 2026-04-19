"""
Toaster-specific criteria for ASR multi-backend parity.

Criterion IDs are load-bearing: ``backends.MockBackend`` dispatches on them
to compute deterministic numeric scores, so keep them in sync with
``backends._mock_score``.
"""

from __future__ import annotations

from typing import Dict, List, Sequence

from verifier_core import Criterion, Trial


GROUND_TRUTH_NOTE = (
    "**IMPORTANT:** Focus on WORD TIMINGS and TRANSCRIBED TEXT as ground "
    "truth. The oracle is analytical (synthesized fixtures) or forced-aligned "
    "(real speech). Do NOT trust a backend's claim of "
    "`word_timestamps_authoritative` in isolation — cross-check it against "
    "observed p95 timing error. A backend that claims authoritative timings "
    "but blows past the 40 ms p95 gate is *lying* and must be scored worse "
    "than an honest non-authoritative backend with the same error."
)


CRITERIA: List[Criterion] = [
    Criterion(
        id="transcription_fidelity",
        name="Transcription Fidelity",
        description=(
            "Compare the transcribed word sequence to the oracle. Count "
            "substitutions, insertions, and deletions from a Levenshtein-"
            "anchored alignment. Fewer is better. Ignore per-word timing "
            "in this criterion — it is scored separately."
        ),
    ),
    Criterion(
        id="word_timing_fidelity",
        name="Word-Timing Fidelity",
        description=(
            "Evaluate per-word start/end timestamps. Prefer timings that "
            "are (a) monotonic and non-overlapping, (b) do NOT synthesize "
            "equal durations across multi-word segments (Toaster's PRD "
            "bans this), and (c) have low median and p95 absolute error "
            "vs the oracle. Gate thresholds: median <= 20 ms, p95 <= "
            "40 ms. Equal-duration synthesis must score worst even if "
            "median error is low by coincidence — the regression is in "
            "the mechanism, not the metric."
        ),
    ),
    Criterion(
        id="authoritative_honesty",
        name="Authoritative-Claim Honesty",
        description=(
            "The adapter contract exposes a `word_timestamps_authoritative` "
            "flag. A backend that claims authoritative timings AND stays "
            "within the p95 <= 40 ms gate is ideal. A backend that does "
            "not claim authoritative timings but performs within gate is "
            "acceptable. A backend that DOES claim authoritative timings "
            "but blows past the gate is lying and must score worst."
        ),
    ),
]


# ---------------------------------------------------------------------------
# Hypothesis → Trial
# ---------------------------------------------------------------------------


def _normalize_text(s: str) -> str:
    return "".join(c for c in s.lower() if c.isalnum())


def _levenshtein(ref: Sequence[str], hyp: Sequence[str]) -> Dict[str, int]:
    m, n = len(ref), len(hyp)
    d = [[0] * (n + 1) for _ in range(m + 1)]
    for i in range(m + 1):
        d[i][0] = i
    for j in range(n + 1):
        d[0][j] = j
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            cost = 0 if ref[i - 1] == hyp[j - 1] else 1
            d[i][j] = min(
                d[i - 1][j] + 1,
                d[i][j - 1] + 1,
                d[i - 1][j - 1] + cost,
            )
    i, j = m, n
    subs = ins = dels = matches = 0
    while i > 0 or j > 0:
        if (
            i > 0
            and j > 0
            and ref[i - 1] == hyp[j - 1]
            and d[i][j] == d[i - 1][j - 1]
        ):
            matches += 1
            i -= 1
            j -= 1
        elif i > 0 and j > 0 and d[i][j] == d[i - 1][j - 1] + 1:
            subs += 1
            i -= 1
            j -= 1
        elif j > 0 and d[i][j] == d[i][j - 1] + 1:
            ins += 1
            j -= 1
        else:
            dels += 1
            i -= 1
    return {"matches": matches, "substitutions": subs, "insertions": ins, "deletions": dels}


def _percentile(values: Sequence[float], p: float) -> float:
    if not values:
        return 0.0
    s = sorted(values)
    idx = min(len(s) - 1, max(0, int((p / 100.0) * len(s)) - 1))
    return float(s[idx])


def _ordinal_align(oracle, hyp_words):
    pairs = []
    j = 0
    for i, ow in enumerate(oracle):
        on = _normalize_text(ow.get("word") or ow.get("text", ""))
        while j < len(hyp_words) and _normalize_text(hyp_words[j].get("text", "")) != on:
            j += 1
        if j < len(hyp_words):
            pairs.append((i, j))
            j += 1
    return pairs


def _timing_stats(oracle, hyp_words, alignment_indices):
    errs = []
    for ref_idx, hyp_idx in alignment_indices:
        rs = float(oracle[ref_idx]["start_us"])
        re_ = float(oracle[ref_idx]["end_us"])
        hs = float(hyp_words[hyp_idx]["start_us"])
        he = float(hyp_words[hyp_idx]["end_us"])
        errs.append(abs(hs - rs))
        errs.append(abs(he - re_))
    return {
        "median_err_us": _percentile(errs, 50),
        "p95_err_us": _percentile(errs, 95),
        "err_count": len(errs),
    }


def _equal_duration_fraction(hyp_words: Sequence[dict]) -> float:
    if len(hyp_words) < 3:
        return 0.0
    durs = [
        float(w["end_us"]) - float(w["start_us"])
        for w in hyp_words
        if float(w.get("end_us", 0)) > float(w.get("start_us", 0))
    ]
    if len(durs) < 3:
        return 0.0
    mean = sum(durs) / len(durs)
    if mean <= 0:
        return 0.0
    var = sum((d - mean) ** 2 for d in durs) / len(durs)
    cv = (var**0.5) / mean
    if cv < 0.02:
        return 1.0
    if cv > 0.25:
        return 0.0
    return max(0.0, min(1.0, (0.25 - cv) / (0.25 - 0.02)))


def build_trial(
    task_name: str,
    backend_name: str,
    oracle_words: Sequence[dict],
    hyp: dict,
) -> Trial:
    """Build a ``Trial`` whose ``trace`` embeds numeric hints (readable by
    both the mock backend and a real LLM)."""

    hyp_words = hyp.get("words", [])
    alignment = _ordinal_align(oracle_words, hyp_words)
    ref = [_normalize_text(ow.get("word") or ow.get("text", "")) for ow in oracle_words]
    hyp_norm = [_normalize_text(w.get("text", "")) for w in hyp_words]
    lev = _levenshtein(ref, hyp_norm)
    timing = _timing_stats(oracle_words, hyp_words, alignment)
    eqdur = _equal_duration_fraction(hyp_words)
    authoritative = bool(hyp.get("word_timestamps_authoritative", False))

    trace_lines = [
        f"backend={backend_name}",
        f"authoritative={'true' if authoritative else 'false'}",
        f"language={hyp.get('language', 'unknown')}",
        f"word_count={len(hyp_words)}",
        f"matched_words={lev['matches']}",
        f"substitutions={lev['substitutions']}",
        f"insertions={lev['insertions']}",
        f"deletions={lev['deletions']}",
        f"median_err_us={timing['median_err_us']:.0f}",
        f"p95_err_us={timing['p95_err_us']:.0f}",
        "p95_threshold_us=40000",
        f"equal_duration_fraction={eqdur:.3f}",
        "",
        "Transcribed words with timings:",
    ]
    for w in hyp_words:
        trace_lines.append(
            f"  {w.get('text', '')!r}  [{w.get('start_us', 0)}us -> {w.get('end_us', 0)}us]"
        )

    problem_lines = [
        f"task={task_name}",
        f"oracle_word_count={len(oracle_words)}",
        "Oracle words:",
    ]
    for ow in oracle_words:
        text = ow.get("word") or ow.get("text", "")
        problem_lines.append(
            f"  {text!r}  [{ow.get('start_us', 0)}us -> {ow.get('end_us', 0)}us]"
        )

    return Trial(
        trial_name=backend_name,
        reward=0.0,
        problem="\n".join(problem_lines),
        trace="\n".join(trace_lines),
    )
