#!/usr/bin/env python3
"""
Disfluency-cleanup ranker via LLM-as-a-Verifier.

For each fixture under ``src-tauri/tests/fixtures/disfluency/``, this
runner loads a real WAV plus its oracle words (labeled with repetition
group ids), builds a :class:`Trial` per candidate, and runs the
pairwise tournament. The four criteria are:

  * group_collapse_completeness  — each group reduced to exactly one
  * survivor_clarity             — audio-aware: kept the clearer take
  * cut_placement_cleanliness    — audio-aware: seams land in silence
  * timing_monotonicity          — kept indices remain ordered

Exits non-zero if any fixture's tournament winner is not the fixture's
``expected_winner``.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import pathlib
import sys
from typing import Dict, List


HERE = pathlib.Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

from backends import make_backend  # noqa: E402
from criteria_disfluency import CRITERIA, GROUND_TRUTH_NOTE, build_trial  # noqa: E402
from verifier_core import Trial, score_all_pairs, select_best  # noqa: E402


DEFAULT_FIXTURES_DIR = REPO_ROOT / "src-tauri" / "tests" / "fixtures" / "disfluency"
DEFAULT_OUTPUT_ROOT = REPO_ROOT / "eval" / "output" / "verifier-disfluency"


def _discover_fixtures(fixtures_dir: pathlib.Path) -> List[pathlib.Path]:
    return sorted(fixtures_dir.glob("*.fixture.json"))


def _extract_numeric(trace: str, key: str) -> float:
    import re

    m = re.search(rf"{key}\s*=\s*([-0-9.]+)", trace)
    return float(m.group(1)) if m else 0.0


def _format_markdown(report: dict) -> str:
    lines = [
        f"# eval-verifier disfluency — {report['status']}",
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
            f"Winner: **{fx['winner']}**  (expected: `{fx['expected_winner']}`)",
            "",
            "| Candidate | Wins | Groups OK | Zero | Many | Clarity | Seam silence | Pacing |",
            "| --- | --- | --- | --- | --- | --- | --- | --- |",
        ]
        for tr in fx["trials"]:
            lines.append(
                f"| {tr['candidate']} | {tr['wins']:.1f} | "
                f"{tr['groups_ok']}/{tr['groups_total']} | "
                f"{tr['groups_with_zero']} | {tr['groups_with_many']} | "
                f"{tr['clarity_ratio']:.2f} | {tr['mean_seam_silence']:.2f} | "
                f"{tr['pacing_agreement']:.2f} |"
            )
        lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixtures-dir", type=pathlib.Path, default=DEFAULT_FIXTURES_DIR)
    parser.add_argument("--output-root", type=pathlib.Path, default=DEFAULT_OUTPUT_ROOT)
    parser.add_argument("--fixture", default="")
    parser.add_argument(
        "--backend",
        default="mock",
        choices=["mock", "openai-compat", "gemini"],
    )
    parser.add_argument("--base-url", default="http://127.0.0.1:8080/v1")
    parser.add_argument("--model", default="local")
    parser.add_argument("--api-key", default="")
    parser.add_argument("--n-verifications", type=int, default=4)
    parser.add_argument("--criteria", type=int, default=len(CRITERIA))
    parser.add_argument("--max-workers", type=int, default=8)
    parser.add_argument("--cache-file", default="")
    parser.add_argument("--no-exit-code", action="store_true")
    args = parser.parse_args()

    fixtures_dir = args.fixtures_dir.resolve()
    output_root = args.output_root.resolve()

    if not fixtures_dir.is_dir():
        print(f"fixtures dir not found: {fixtures_dir}", file=sys.stderr)
        return 2

    paths = _discover_fixtures(fixtures_dir)
    if args.fixture:
        paths = [p for p in paths if p.stem.replace(".fixture", "") == args.fixture]
    if not paths:
        print("no disfluency fixtures discovered", file=sys.stderr)
        return 2

    criteria = CRITERIA[: args.criteria]
    backend_kwargs = dict(ground_truth_note=GROUND_TRUTH_NOTE)
    if args.backend == "openai-compat":
        backend_kwargs.update(base_url=args.base_url, model=args.model, api_key=args.api_key)
    scorer = make_backend(args.backend, **backend_kwargs)

    def score_fn(problem, trace_a, trace_b, crit):
        return scorer.score_pair(problem, trace_a, trace_b, crit)

    tasks: Dict[str, List[Trial]] = {}
    fixture_meta: Dict[str, dict] = {}
    for path in paths:
        with open(path, "r", encoding="utf-8") as f:
            fixture = json.load(f)
        stem = fixture.get("fixture") or path.stem.replace(".fixture", "")
        audio_path = fixture.get("audio_path")
        if audio_path:
            resolved = (path.parent / audio_path).resolve()
            fixture["audio_path"] = str(resolved)

        candidates = fixture.get("candidates", [])
        if len(candidates) < 2:
            print(f"  skip {stem}: <2 candidates")
            continue
        trials = [build_trial(fixture, c) for c in candidates]
        tasks[stem] = trials
        fixture_meta[stem] = {
            "expected_winner": fixture.get(
                "expected_winner", candidates[0]["name"]
            ),
            "aspirational": bool(fixture.get("aspirational", False)),
        }

    if not tasks:
        print("no fixtures with >= 2 candidates", file=sys.stderr)
        return 2

    timestamp = _dt.datetime.utcnow().strftime("%Y%m%dT%H%M%SZ")
    run_dir = output_root / timestamp
    run_dir.mkdir(parents=True, exist_ok=True)
    cache_file = args.cache_file or str(run_dir / "scores.cache.json")

    print(
        f"[eval-verifier-disfluency] backend={scorer.name} "
        f"fixtures={list(tasks)} criteria={[c.id for c in criteria]} "
        f"n_verifications={args.n_verifications}"
    )

    def _on_progress(done, total, errs):
        if done == total or (total and done % max(1, total // 10) == 0):
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
        expected = fixture_meta[stem]["expected_winner"]
        aspirational = fixture_meta[stem]["aspirational"]
        winner_matches = winner.trial_name == expected
        if winner_matches:
            fx_status = "pass"
        elif aspirational:
            fx_status = "gap"
        else:
            fx_status = "fail"
        reasons = []
        if fx_status == "gap":
            reasons.append(
                f"winner={winner.trial_name} but aspirational_expected={expected} "
                f"(documented gap; does not fail CI)"
            )
        elif fx_status == "fail":
            reasons.append(f"winner={winner.trial_name} but expected={expected}")
            overall_status = "fail"

        rows = []
        for k, tr in enumerate(trials):
            rows.append(
                {
                    "candidate": tr.trial_name,
                    "wins": sel["wins"][k],
                    "groups_total": int(_extract_numeric(tr.trace, "groups_total")),
                    "groups_ok": int(_extract_numeric(tr.trace, "groups_ok")),
                    "groups_with_zero": int(
                        _extract_numeric(tr.trace, "groups_with_zero")
                    ),
                    "groups_with_many": int(
                        _extract_numeric(tr.trace, "groups_with_many")
                    ),
                    "clarity_ratio": _extract_numeric(tr.trace, "clarity_ratio"),
                    "mean_seam_silence": _extract_numeric(
                        tr.trace, "mean_seam_silence"
                    ),
                    "pacing_agreement": _extract_numeric(
                        tr.trace, "pacing_agreement"
                    ),
                    "pacing_scoreable": int(
                        _extract_numeric(tr.trace, "pacing_scoreable")
                    ),
                }
            )
        fixtures_out.append(
            {
                "fixture": stem,
                "status": fx_status,
                "reasons": reasons,
                "winner": winner.trial_name,
                "expected_winner": expected,
                "wins": list(sel["wins"]),
                "trials": rows,
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
    print(f"=== eval-verifier disfluency ({overall_status.upper()}) ===")
    for fx in fixtures_out:
        print(
            f"  [{fx['status'].upper()}] {fx['fixture']} -> "
            f"winner={fx['winner']} (expected={fx['expected_winner']})"
        )
        for reason in fx["reasons"]:
            print(f"      ! {reason}")
    print(f"\nReport: {json_path}")

    if args.no_exit_code:
        return 0
    return 0 if overall_status == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
