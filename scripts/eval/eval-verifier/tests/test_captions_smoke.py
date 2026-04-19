"""Smoke tests for the caption-grouping verifier runner."""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


HERE = pathlib.Path(__file__).resolve().parent
PKG_DIR = HERE.parent
RUNNER = PKG_DIR / "run_captions.py"


def _run(*extra, output_root=None):
    args = [sys.executable, str(RUNNER), "--backend", "mock", *extra]
    if output_root:
        args += ["--output-root", str(output_root)]
    return subprocess.run(args, capture_output=True, text=True, check=False)


class TestCaptionsSmoke(unittest.TestCase):
    def test_balanced_natural_wins(self):
        with tempfile.TemporaryDirectory() as tmp:
            res = _run(output_root=pathlib.Path(tmp))
            self.assertEqual(
                res.returncode,
                0,
                msg=f"runner failed: stdout={res.stdout}\nstderr={res.stderr}",
            )
            report = json.loads(
                sorted(pathlib.Path(tmp).glob("*/report.json"))[-1].read_text()
            )
            self.assertEqual(report["status"], "pass", msg=json.dumps(report, indent=2))
            for fx in report["fixtures"]:
                self.assertEqual(fx["winner"], fx["expected_winner"])

    def test_quote_splitter_ranks_below_natural(self):
        with tempfile.TemporaryDirectory() as tmp:
            res = _run(output_root=pathlib.Path(tmp))
            self.assertEqual(res.returncode, 0, msg=res.stderr)
            report = json.loads(
                sorted(pathlib.Path(tmp).glob("*/report.json"))[-1].read_text()
            )
            fx = report["fixtures"][0]
            by_name = {t["candidate"]: t for t in fx["trials"]}
            self.assertLess(
                by_name["splits_quote"]["wins"],
                by_name["balanced_natural"]["wins"],
            )
            # The word-dropping candidate must not win either.
            self.assertLess(
                by_name["drops_word"]["wins"],
                by_name["balanced_natural"]["wins"],
            )


if __name__ == "__main__":
    unittest.main()
