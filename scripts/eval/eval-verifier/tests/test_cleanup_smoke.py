"""Smoke tests for the cleanup verifier runner."""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


HERE = pathlib.Path(__file__).resolve().parent
PKG_DIR = HERE.parent
RUNNER = PKG_DIR / "run_cleanup.py"


def _run(*extra, output_root=None):
    args = [sys.executable, str(RUNNER), "--backend", "mock", *extra]
    if output_root:
        args += ["--output-root", str(output_root)]
    return subprocess.run(args, capture_output=True, text=True, check=False)


class TestCleanupSmoke(unittest.TestCase):
    def test_perfect_candidate_wins_each_fixture(self):
        with tempfile.TemporaryDirectory() as tmp:
            res = _run(output_root=pathlib.Path(tmp))
            self.assertEqual(
                res.returncode,
                0,
                msg=f"runner failed: stdout={res.stdout}\nstderr={res.stderr}",
            )
            reports = sorted(pathlib.Path(tmp).glob("*/report.json"))
            self.assertTrue(reports, "no report.json emitted")
            report = json.loads(reports[-1].read_text(encoding="utf-8"))
            self.assertEqual(report["status"], "pass", msg=json.dumps(report, indent=2))
            for fx in report["fixtures"]:
                self.assertEqual(fx["winner"], fx["expected_winner"])
                # Winner must have zero quote violations.
                winner_row = next(
                    t for t in fx["trials"] if t["candidate"] == fx["winner"]
                )
                self.assertEqual(winner_row["quote_violations"], 0)

    def test_quote_violator_ranks_below_perfect(self):
        """The 'quote_violation' candidate in intro_with_quote must rank
        strictly below 'rule_v1_perfect'."""

        with tempfile.TemporaryDirectory() as tmp:
            res = _run("--fixture", "intro_with_quote", output_root=pathlib.Path(tmp))
            self.assertEqual(res.returncode, 0, msg=res.stderr)
            report = json.loads(
                sorted(pathlib.Path(tmp).glob("*/report.json"))[-1].read_text()
            )
            fx = report["fixtures"][0]
            by_name = {t["candidate"]: t for t in fx["trials"]}
            self.assertIn("quote_violation", by_name)
            self.assertIn("rule_v1_perfect", by_name)
            self.assertLess(
                by_name["quote_violation"]["wins"],
                by_name["rule_v1_perfect"]["wins"],
            )

    def test_audio_aware_prefers_audible_deletion(self):
        """phrase_alpha_audio exposes two fillers — one audible, one silent.
        Candidates that each delete exactly one filler are indistinguishable
        under recall/preservation/monotonicity; the audio-aware criterion
        must break the tie so 'deletes_real_filler' ranks above
        'deletes_phantom_filler'."""

        with tempfile.TemporaryDirectory() as tmp:
            res = _run(
                "--fixture", "phrase_alpha_audio", output_root=pathlib.Path(tmp)
            )
            self.assertEqual(res.returncode, 0, msg=res.stderr)
            report = json.loads(
                sorted(pathlib.Path(tmp).glob("*/report.json"))[-1].read_text()
            )
            fx = report["fixtures"][0]
            by_name = {t["candidate"]: t for t in fx["trials"]}
            self.assertEqual(fx["winner"], "rule_v1_perfect")
            self.assertLess(
                by_name["deletes_phantom_filler"]["wins"],
                by_name["deletes_real_filler"]["wins"],
                msg=(
                    "audio-aware tiebreaker failed: phantom="
                    f"{by_name['deletes_phantom_filler']['wins']} "
                    f"real={by_name['deletes_real_filler']['wins']}"
                ),
            )


if __name__ == "__main__":
    unittest.main()
