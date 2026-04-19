"""
Verifier core: fine-grained scoring + round-robin tournament.

Adapted from https://github.com/llm-as-a-verifier/llm-as-a-verifier
(`scripts/verifier_core.py`) for Toaster's offline eval harness. Differences
from upstream:

  * Model backends are pluggable via ``backends`` so Toaster can run the
    harness with a deterministic mock, a local OpenAI-compatible server with
    top-k logprobs (preferred, preserves local-first product ethos), or
    Gemini 2.5 Flash (upstream parity).
  * Scoring scale identical to upstream: 20 letter tokens A..T with A=20.0
    (best) and T=1.0 (worst). Expected value over the top-k distribution at
    the ``<score_A>`` / ``<score_B>`` tag is normalized to [0, 1].
  * Selection uses the same round-robin tournament over all C(N, 2) pairs,
    averaged across criteria and repetitions (wins, ties = 0.5 each).
"""

from __future__ import annotations

import json
import math
import os
import re
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from itertools import combinations
from typing import Any, Callable, Dict, Iterable, List, Optional, Sequence, Tuple


GRANULARITY = 20

_LETTERS_UPPER = {chr(65 + i): float(GRANULARITY - i) for i in range(GRANULARITY)}
_LETTERS_LOWER = {chr(97 + i): float(GRANULARITY - i) for i in range(GRANULARITY)}

SCALE = {
    "description": (
        "Rate how likely trajectory X correctly solved the task on a "
        "20-point scale using letters A through T:\n"
        "  A = clearly and completely correct (best)\n"
        "  B-D = correct with only minor issues\n"
        "  E-G = above average, mostly correct with some issues\n"
        "  H-J = uncertain, leans toward correct\n"
        "  K-M = uncertain, leans toward incorrect\n"
        "  N-P = below average, significant issues remain\n"
        "  Q-S = incorrect with some partial progress\n"
        "  T = clearly and completely incorrect (worst)"
    ),
    "score_format": "LETTER_A_TO_T",
    "valid_tokens": {**_LETTERS_UPPER, **_LETTERS_LOWER},
}


@dataclass(frozen=True)
class Criterion:
    id: str
    name: str
    description: str


@dataclass
class Trial:
    """One candidate output for one task (e.g. one backend's transcription)."""

    trial_name: str
    reward: float
    problem: str
    trace: str


def score_from_logprobs(
    top_logprobs: Sequence[Tuple[str, float]],
) -> Optional[float]:
    """Expected letter-score from (token, logprob) pairs, normalized to [0, 1]."""

    valid = SCALE["valid_tokens"]
    probs: Dict[float, float] = {}
    for tok_str, logp in top_logprobs:
        tok = tok_str.strip()
        if tok in valid:
            val = valid[tok]
            p = math.exp(logp)
            probs[val] = max(probs.get(val, 0.0), p)
    if not probs:
        return None
    total = sum(probs.values())
    expected = sum(v * p for v, p in probs.items()) / total
    unique = sorted(set(valid.values()))
    lo, hi = unique[0], unique[-1]
    if hi <= lo:
        return 0.5
    return (expected - lo) / (hi - lo)


def score_from_text(text: str, tag: str) -> Optional[float]:
    """Fallback: parse ``<tag>LETTER</tag>`` from raw text."""

    valid = SCALE["valid_tokens"]
    tag_name = tag.strip("<>")
    pattern = rf"<{re.escape(tag_name)}>\s*([A-Ta-t])\s*</{re.escape(tag_name)}>"
    m = re.search(pattern, text or "")
    if not m:
        return None
    raw = valid.get(m.group(1)) or valid.get(m.group(1).upper())
    if raw is None:
        return None
    unique = sorted(set(valid.values()))
    lo, hi = unique[0], unique[-1]
    if hi <= lo:
        return 0.5
    return (raw - lo) / (hi - lo)


ScoreFn = Callable[[str, str, str, Criterion], Tuple[float, float]]


def score_all_pairs(
    tasks: Dict[str, List[Trial]],
    swing_tasks: Iterable[str],
    criteria: Sequence[Criterion],
    score_fn: ScoreFn,
    n_reps: int,
    max_workers: int = 16,
    cache_file: Optional[str] = None,
    on_progress: Optional[Callable[[int, int, int], None]] = None,
) -> Dict[str, Dict[str, float]]:
    """Score every (pair, criterion, rep) on ``swing_tasks`` with disk caching."""

    cached: Dict[str, Dict[str, float]] = {}
    if cache_file and os.path.exists(cache_file):
        with open(cache_file, "r", encoding="utf-8") as f:
            cached = json.load(f)

    jobs: List[Tuple[str, str, str, str, Criterion]] = []
    for task_name in swing_tasks:
        trials = tasks[task_name]
        for i, j in combinations(range(len(trials)), 2):
            for crit in criteria:
                for rep in range(n_reps):
                    key = f"{crit.id}|{task_name}|{i},{j}|{rep}"
                    if key not in cached:
                        jobs.append(
                            (key, trials[i].problem, trials[i].trace, trials[j].trace, crit)
                        )

    if not jobs:
        return cached

    errors = 0
    done = 0
    total = len(jobs)
    save_every = max(1, total // 20)

    def _run(job):
        key, prob, ta, tb, crit = job
        sa, sb = score_fn(prob, ta, tb, crit)
        return key, sa, sb

    with ThreadPoolExecutor(max_workers=max_workers) as ex:
        futures = {ex.submit(_run, j): j for j in jobs}
        for fut in as_completed(futures):
            job = futures[fut]
            try:
                key, sa, sb = fut.result()
                cached[key] = {"score_i": float(sa), "score_j": float(sb)}
            except Exception as exc:  # noqa: BLE001
                key = job[0]
                cached[key] = {"score_i": 0.5, "score_j": 0.5, "error": str(exc)}
                errors += 1
            done += 1
            if on_progress:
                on_progress(done, total, errors)
            if cache_file and done % save_every == 0:
                with open(cache_file, "w", encoding="utf-8") as f:
                    json.dump(cached, f)

    if cache_file:
        with open(cache_file, "w", encoding="utf-8") as f:
            json.dump(cached, f)

    return cached


def select_best(
    tasks: Dict[str, List[Trial]],
    swing_tasks: Iterable[str],
    scores: Dict[str, Dict[str, float]],
    criteria_ids: Sequence[str],
    n_reps: int,
) -> Dict[str, Dict[str, Any]]:
    """Round-robin tournament: pick the highest-wins trial per task."""

    selections: Dict[str, Dict[str, Any]] = {}
    for task_name in swing_tasks:
        trials = tasks[task_name]
        n = len(trials)
        wins = [0.0] * n
        for i, j in combinations(range(n), 2):
            si_sum = sj_sum = 0.0
            count = 0
            for cid in criteria_ids:
                for rep in range(n_reps):
                    key = f"{cid}|{task_name}|{i},{j}|{rep}"
                    entry = scores.get(key)
                    if not entry:
                        continue
                    si_sum += entry.get("score_i", 0.5)
                    sj_sum += entry.get("score_j", 0.5)
                    count += 1
            si = si_sum / count if count else 0.5
            sj = sj_sum / count if count else 0.5
            if si > sj:
                wins[i] += 1
            elif sj > si:
                wins[j] += 1
            else:
                wins[i] += 0.5
                wins[j] += 0.5
        best_idx = max(range(n), key=lambda t: wins[t])
        selections[task_name] = {
            "idx": best_idx,
            "trial": trials[best_idx].trial_name,
            "reward": trials[best_idx].reward,
            "wins": wins,
        }
    return selections
