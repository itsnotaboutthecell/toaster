#!/usr/bin/env python3
"""
Multi-backend ASR parity via LLM-as-a-Verifier Best-of-N selection.

For each parity fixture under ``src-tauri/tests/fixtures/parity/`` this
runner:

  1. Loads the oracle word list (``<fixture>.oracle.json``).
  2. Loads each backend's cached hypothesis
     (``backend_outputs/<backend>/<fixture>.result.json``).
  3. Builds a per-backend :class:`Trial` whose ``trace`` embeds numeric
     quality hints (match counts, median/p95 error, equal-duration
     fraction, authoritative flag).
  4. Scores every backend pair × criterion × rep via the configured
     backend (mock / openai-compat / gemini).
  5. Runs a round-robin tournament to pick the best backend per fixture.
  6. Emits a JSON + Markdown report; exit code != 0 if any fixture fails
     the pass/fail policy (either no clear winner, or the winner's
     timing-fidelity score falls below a floor).
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import pathlib
import sys
from typing import Dict, List


HERE = pathlib.Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

from backends import make_backend  # noqa: E402
from criteria_parity import (  # noqa: E402
    CRITERIA,
    GROUND_TRUTH_NOTE,
    build_trial,
)
from verifier_core import Trial, score_all_pairs, select_best  # noqa: E402


DEFAULT_FIXTURES_DIR = REPO_ROOT / "src-tauri" / "tests" / "fixtures" / "parity"
DEFAULT_OUTPUT_ROOT = REPO_ROOT / "eval" / "output" / "verifier-parity"


def _discover_fixtures(fixtures_dir: pathlib.Path) -> List[str]:
    return sorted(p.stem.replace(".oracle", "") for p in fixtures_dir.glob("*.oracle.json"))


def _discover_backends(fixtures_dir: pathlib.Path) -> List[str]:
    backends_root = fixtures_dir / "backend_outputs"
    if not backends_root.is_dir():
        return []
    return sorted(p.name for p in backends_root.iterdir() if p.is_dir())


def _load_json(path: pathlib.Path):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _floor_score(hints_trace: str, key: str, default: float) -> float:
    import re

    m = re.search(rf"{key}\s*=\s*([0-9.]+)", hints_trace)
    return float(m.group(1)) if m else default


def _ensure_dir(p: pathlib.Path) -> None:
    p.mkdir(parents=True, exist_ok=True)


def _format_markdown(report: dict) -> str:
    lines = [
        f"# eval-verifier parity — {report['status']}",
        "",
        f"- backend: `{report['backend']}`",
        f"- n_verifications: {report['n_verifications']}",
        f"- criteria: {', '.join(c['id'] for c in report['criteria'])}",
        f"- timestamp: {report['timestamp']}",
        "",
    ]
    for fx in report["fixtures"]:
        lines += [
            f"## {fx['fixture']} — {fx['status']}",
            "",
            f"Winner: **{fx['winner']}** (wins={fx['wins']})",
            "",
            "| Backend | Wins | Authoritative | Median µs | p95 µs | Eq-dur frac |",
            "| --- | --- | --- | --- | --- | --- |",
        ]
        for tr in fx["trials"]:
            lines.append(
                f"| {tr['backend']} | {tr['wins']:.1f} | "
                f"{tr['authoritative']} | {tr['median_err_us']:.0f} | "
                f"{tr['p95_err_us']:.0f} | {tr['equal_duration_fraction']:.3f} |"
            )
        lines.append("")
    return "\n".join(lines)


def _extract_numeric(trace: str, key: str) -> float:
    import re

    m = re.search(rf"{key}\s*=\s*([-0-9.]+)", trace)
    return float(m.group(1)) if m else 0.0


def _extract_bool(trace: str, key: str) -> bool:
    import re

    m = re.search(rf"{key}\s*=\s*(true|false)", trace)
    return bool(m) and m.group(1) == "true"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixtures-dir", type=pathlib.Path, default=DEFAULT_FIXTURES_DIR)
    parser.add_argument("--output-root", type=pathlib.Path, default=DEFAULT_OUTPUT_ROOT)
    parser.add_argument("--fixture", default="", help="Filter to one fixture stem")
    parser.add_argument(
        "--backend",
        default="mock",
        choices=["mock", "openai-compat", "gemini"],
        help="LLM backend for scoring. Default: mock (no network).",
    )
    parser.add_argument("--base-url", default="http://127.0.0.1:8080/v1")
    parser.add_argument("--model", default="local")
    parser.add_argument("--api-key", default="")
    parser.add_argument("--n-verifications", type=int, default=4)
    parser.add_argument("--criteria", type=int, default=len(CRITERIA))
    parser.add_argument("--max-workers", type=int, default=8)
    parser.add_argument("--cache-file", default="")
    parser.add_argument(
        "--p95-floor-us",
        type=float,
        default=40000.0,
        help="Fail if the winning trial's p95 exceeds this. Matches G2 gate.",
    )
    parser.add_argument(
        "--no-exit-code",
        action="store_true",
        help="Always exit 0 (report only; useful when the eval is advisory).",
    )
    args = parser.parse_args()

    fixtures_dir = args.fixtures_dir.resolve()
    output_root = args.output_root.resolve()

    if not fixtures_dir.is_dir():
        print(f"fixtures dir not found: {fixtures_dir}", file=sys.stderr)
        return 2

    fixture_stems = _discover_fixtures(fixtures_dir)
    if args.fixture:
        fixture_stems = [s for s in fixture_stems if s == args.fixture]
    if not fixture_stems:
        print("no parity fixtures discovered", file=sys.stderr)
        return 2

    backend_names = _discover_backends(fixtures_dir)
    if len(backend_names) < 2:
        print(
            f"need at least 2 backends under {fixtures_dir}/backend_outputs/ "
            f"(found: {backend_names})",
            file=sys.stderr,
        )
        return 2

    criteria = CRITERIA[: args.criteria]
    backend_kwargs = dict(ground_truth_note=GROUND_TRUTH_NOTE)
    if args.backend == "openai-compat":
        backend_kwargs.update(
            base_url=args.base_url, model=args.model, api_key=args.api_key
        )
    scorer = make_backend(args.backend, **backend_kwargs)

    def score_fn(problem, trace_a, trace_b, crit):
        return scorer.score_pair(problem, trace_a, trace_b, crit)

    tasks: Dict[str, List[Trial]] = {}
    fixture_meta: Dict[str, dict] = {}
    for stem in fixture_stems:
        oracle_path = fixtures_dir / f"{stem}.oracle.json"
        oracle = _load_json(oracle_path)
        trials: List[Trial] = []
        per_backend_meta = []
        for be in backend_names:
            hyp_path = fixtures_dir / "backend_outputs" / be / f"{stem}.result.json"
            if not hyp_path.exists():
                print(f"  skip {stem}/{be}: no {hyp_path}")
                continue
            hyp = _load_json(hyp_path)
            trial = build_trial(stem, be, oracle, hyp)
            trials.append(trial)
            per_backend_meta.append(
                {
                    "backend": be,
                    "authoritative": _extract_bool(trial.trace, "authoritative"),
                    "median_err_us": _extract_numeric(trial.trace, "median_err_us"),
                    "p95_err_us": _extract_numeric(trial.trace, "p95_err_us"),
                    "equal_duration_fraction": _extract_numeric(
                        trial.trace, "equal_duration_fraction"
                    ),
                }
            )
        if len(trials) < 2:
            print(f"  skip {stem}: <2 backends loaded")
            continue
        tasks[stem] = trials
        fixture_meta[stem] = {"per_backend": per_backend_meta, "oracle_words": len(oracle)}

    if not tasks:
        print("no fixtures with >= 2 loaded backends", file=sys.stderr)
        return 2

    timestamp = _dt.datetime.utcnow().strftime("%Y%m%dT%H%M%SZ")
    run_dir = output_root / timestamp
    _ensure_dir(run_dir)
    cache_file = args.cache_file or str(run_dir / "scores.cache.json")

    print(
        f"[eval-verifier-parity] backend={scorer.name} "
        f"fixtures={list(tasks)} criteria={[c.id for c in criteria]} "
        f"n_verifications={args.n_verifications}"
    )

    def _on_progress(done, total, errs):
        if done == total or done % max(1, total // 10) == 0:
            print(f"  scoring: {done}/{total} ({errs} errors)")

    scores = score_all_pairs(
        tasks=tasks,
        swing_tasks=list(tasks),
        criteria=criteria,
        score_fn=score_fn,
        n_reps=args.n_verifications,
        max_workers=args.max_workers,
        cache_file=cache_file,
        on_progress=_on_progress,
    )

    selections = select_best(
        tasks=tasks,
        swing_tasks=list(tasks),
        scores=scores,
        criteria_ids=[c.id for c in criteria],
        n_reps=args.n_verifications,
    )

    overall_status = "pass"
    fixtures_out = []
    for stem, sel in selections.items():
        trials = tasks[stem]
        winner_idx = sel["idx"]
        winner = trials[winner_idx]
        winner_p95 = _extract_numeric(winner.trace, "p95_err_us")
        winner_eqdur = _extract_numeric(winner.trace, "equal_duration_fraction")

        fx_status = "pass"
        reasons = []
        # Fail if the winner blows past the p95 gate
        if winner_p95 > args.p95_floor_us:
            fx_status = "fail"
            reasons.append(
                f"winner p95={winner_p95:.0f}us exceeds floor {args.p95_floor_us:.0f}us"
            )
        # Fail if the winner looks synthesized
        if winner_eqdur >= 0.95:
            fx_status = "fail"
            reasons.append(
                f"winner equal_duration_fraction={winner_eqdur:.2f} "
                "(synthesized timings)"
            )
        if fx_status != "pass":
            overall_status = "fail"

        fixtures_out.append(
            {
                "fixture": stem,
                "status": fx_status,
                "reasons": reasons,
                "winner": winner.trial_name,
                "winner_idx": winner_idx,
                "wins": list(sel["wins"]),
                "trials": [
                    {
                        **fixture_meta[stem]["per_backend"][k],
                        "wins": sel["wins"][k],
                    }
                    for k in range(len(trials))
                ],
            }
        )

    report = {
        "status": overall_status,
        "timestamp": timestamp,
        "backend": scorer.name,
        "n_verifications": args.n_verifications,
        "criteria": [
            {"id": c.id, "name": c.name, "description": c.description} for c in criteria
        ],
        "fixtures": fixtures_out,
        "cache_file": cache_file,
    }

    json_path = run_dir / "report.json"
    md_path = run_dir / "report.md"
    with open(json_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)
    with open(md_path, "w", encoding="utf-8") as f:
        f.write(_format_markdown(report))

    print("")
    print(f"=== eval-verifier parity ({overall_status.upper()}) ===")
    for fx in fixtures_out:
        print(f"  [{fx['status'].upper()}] {fx['fixture']} -> winner={fx['winner']}")
        for reason in fx["reasons"]:
            print(f"      ! {reason}")
    print(f"\nReport: {json_path}")

    if args.no_exit_code:
        return 0
    return 0 if overall_status == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
