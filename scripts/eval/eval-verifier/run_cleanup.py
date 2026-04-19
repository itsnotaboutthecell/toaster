#!/usr/bin/env python3
"""
Cleanup / filler-removal ranker via LLM-as-a-Verifier.

For each cleanup fixture under ``src-tauri/tests/fixtures/cleanup/``, this
runner:

  1. Loads the fixture JSON (oracle words + desired kept indices +
     candidate cleanup outputs).
  2. Builds a :class:`Trial` per candidate via ``criteria_cleanup.build_trial``
     (which computes filler recall, content preservation, monotonicity,
     and — if ``audio_path`` is present — the mean silence ratio of the
     deleted regions).
  3. Runs the pairwise scorer × round-robin tournament.
  4. Emits JSON + Markdown reports. Exits non-zero if any fixture's
     winner is not the fixture's declared ``expected_winner`` (defaults
     to the candidate whose ``kept_indices`` exactly match
     ``desired_kept_indices``).
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
from criteria_cleanup import CRITERIA, GROUND_TRUTH_NOTE, build_trial  # noqa: E402
from verifier_core import Trial, score_all_pairs, select_best  # noqa: E402


DEFAULT_FIXTURES_DIR = REPO_ROOT / "src-tauri" / "tests" / "fixtures" / "cleanup"
DEFAULT_OUTPUT_ROOT = REPO_ROOT / "eval" / "output" / "verifier-cleanup"


def _discover_fixtures(fixtures_dir: pathlib.Path) -> List[pathlib.Path]:
    return sorted(fixtures_dir.glob("*.fixture.json"))


def _expected_winner(fixture: dict) -> str:
    if "expected_winner" in fixture:
        return fixture["expected_winner"]
    desired = fixture.get("desired_kept_indices", [])
    for c in fixture.get("candidates", []):
        if list(c.get("kept_indices", [])) == list(desired):
            return c["name"]
    return fixture["candidates"][0]["name"]


def _format_markdown(report: dict) -> str:
    lines = [
        f"# eval-verifier cleanup — {report['status']}",
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
            "| Candidate | Wins | Fillers removed | Content deleted | Quote violations |",
            "| --- | --- | --- | --- | --- |",
        ]
        for tr in fx["trials"]:
            lines.append(
                f"| {tr['candidate']} | {tr['wins']:.1f} | "
                f"{tr['fillers_removed']}/{tr['fillers_total']} | "
                f"{tr['content_deleted']}/{tr['content_total']} | "
                f"{tr['quote_violations']} |"
            )
        lines.append("")
    return "\n".join(lines)


def _extract_numeric(trace: str, key: str) -> float:
    import re

    m = re.search(rf"{key}\s*=\s*([-0-9.]+)", trace)
    return float(m.group(1)) if m else 0.0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixtures-dir", type=pathlib.Path, default=DEFAULT_FIXTURES_DIR)
    parser.add_argument("--output-root", type=pathlib.Path, default=DEFAULT_OUTPUT_ROOT)
    parser.add_argument("--fixture", default="", help="Filter to one fixture stem")
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
        print("no cleanup fixtures discovered", file=sys.stderr)
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
        # Resolve audio_path relative to the fixture file if provided.
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
            "expected_winner": _expected_winner(fixture),
            "candidates": candidates,
        }

    if not tasks:
        print("no fixtures with >= 2 candidates", file=sys.stderr)
        return 2

    timestamp = _dt.datetime.utcnow().strftime("%Y%m%dT%H%M%SZ")
    run_dir = output_root / timestamp
    run_dir.mkdir(parents=True, exist_ok=True)
    cache_file = args.cache_file or str(run_dir / "scores.cache.json")

    print(
        f"[eval-verifier-cleanup] backend={scorer.name} "
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
        fx_status = "pass" if winner.trial_name == expected else "fail"
        reasons = []
        if fx_status != "pass":
            reasons.append(
                f"winner={winner.trial_name} but expected={expected}"
            )
            overall_status = "fail"

        trials_rows = []
        for k, tr in enumerate(trials):
            trials_rows.append(
                {
                    "candidate": tr.trial_name,
                    "wins": sel["wins"][k],
                    "fillers_total": int(_extract_numeric(tr.trace, "fillers_total")),
                    "fillers_removed": int(_extract_numeric(tr.trace, "fillers_removed")),
                    "content_total": int(_extract_numeric(tr.trace, "content_total")),
                    "content_deleted": int(_extract_numeric(tr.trace, "content_deleted")),
                    "quote_violations": int(_extract_numeric(tr.trace, "quote_violations")),
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
                "trials": trials_rows,
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
    print(f"=== eval-verifier cleanup ({overall_status.upper()}) ===")
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
