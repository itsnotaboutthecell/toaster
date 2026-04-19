"""Smoke tests for the disfluency-cleanup verifier runner."""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


HERE = pathlib.Path(__file__).resolve().parent
PKG_DIR = HERE.parent
RUNNER = PKG_DIR / "run_disfluency.py"


def _run(*extra, output_root=None):
    args = [sys.executable, str(RUNNER), "--backend", "mock", *extra]
    if output_root:
        args += ["--output-root", str(output_root)]
    return subprocess.run(args, capture_output=True, text=True, check=False)


class TestDisfluencySmoke(unittest.TestCase):
    def test_clear_survivors_wins(self):
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
                # Gap fixtures are aspirational: winner may differ from
                # expected without failing CI.
                if fx["status"] == "gap":
                    continue
                self.assertEqual(fx["winner"], fx["expected_winner"])

    def test_audio_clarity_breaks_the_tie(self):
        """clear_survivors and unclear_survivors have IDENTICAL
        group-collapse stats (both groups at 1 survivor, no content
        dropped, same monotonicity). The only thing separating them is
        audio-aware survivor_clarity. clear_survivors must outrank
        unclear_survivors strictly."""

        with tempfile.TemporaryDirectory() as tmp:
            res = _run(output_root=pathlib.Path(tmp))
            self.assertEqual(res.returncode, 0, msg=res.stderr)
            report = json.loads(
                sorted(pathlib.Path(tmp).glob("*/report.json"))[-1].read_text()
            )
            fx = report["fixtures"][0]
            by_name = {t["candidate"]: t for t in fx["trials"]}
            # Collapse parity precondition.
            self.assertEqual(
                by_name["clear_survivors"]["groups_ok"],
                by_name["unclear_survivors"]["groups_ok"],
            )
            self.assertEqual(
                by_name["clear_survivors"]["groups_with_zero"],
                by_name["unclear_survivors"]["groups_with_zero"],
            )
            # Clarity must order them strictly.
            self.assertGreater(
                by_name["clear_survivors"]["clarity_ratio"],
                by_name["unclear_survivors"]["clarity_ratio"],
            )
            self.assertLess(
                by_name["unclear_survivors"]["wins"],
                by_name["clear_survivors"]["wins"],
            )

    def test_content_dropper_ranks_below_collapsers(self):
        """drops_content (keeps [1,2]) has a clear survivor for each
        group but drops the non-group word 'part'. It MUST rank below
        clear_survivors."""

        with tempfile.TemporaryDirectory() as tmp:
            res = _run(output_root=pathlib.Path(tmp))
            self.assertEqual(res.returncode, 0, msg=res.stderr)
            report = json.loads(
                sorted(pathlib.Path(tmp).glob("*/report.json"))[-1].read_text()
            )
            fx = report["fixtures"][0]
            by_name = {t["candidate"]: t for t in fx["trials"]}
            self.assertLess(
                by_name["drops_content"]["wins"],
                by_name["clear_survivors"]["wins"],
            )
            # over_collapse drops an entire group -> also below.
            self.assertLess(
                by_name["over_collapse"]["wins"],
                by_name["clear_survivors"]["wins"],
            )
            # no_collapse leaves fillers -> also below.
            self.assertLess(
                by_name["no_collapse"]["wins"],
                by_name["clear_survivors"]["wins"],
            )


    def test_pacing_agreement_orders_candidates(self):
        """On the real-asset fixture, pacing_agreement must strictly
        rank the human-oracle candidate above any candidate that keeps
        glue-word content the human labeled 'delete'. This guards the
        pacing plumbing end-to-end (criterion present, hints emitted,
        mock backend dispatched) without asserting the full tournament
        winner — which is aspirational on this fixture."""

        with tempfile.TemporaryDirectory() as tmp:
            res = _run(output_root=pathlib.Path(tmp))
            self.assertEqual(res.returncode, 0, msg=res.stderr)
            report = json.loads(
                sorted(pathlib.Path(tmp).glob("*/report.json"))[-1].read_text()
            )
            real = [
                fx
                for fx in report["fixtures"]
                if fx["fixture"] == "toaster_example_candidates"
            ]
            self.assertEqual(len(real), 1, msg="real-asset fixture missing")
            by_name = {t["candidate"]: t for t in real[0]["trials"]}
            # Every candidate must have a non-empty scoreable set — else
            # the criterion isn't wired.
            for tr in real[0]["trials"]:
                self.assertGreater(
                    tr["pacing_scoreable"],
                    0,
                    msg=f"{tr['candidate']}: pacing not scoreable",
                )
            # Self-consistency: human_oracle matches itself perfectly.
            self.assertAlmostEqual(
                by_name["human_oracle"]["pacing_agreement"], 1.0, places=3
            )
            # Candidates that keep human-labeled-delete glue words must
            # score strictly lower.
            self.assertLess(
                by_name["toaster_today"]["pacing_agreement"],
                by_name["human_oracle"]["pacing_agreement"],
            )
            self.assertLess(
                by_name["keep_everything"]["pacing_agreement"],
                by_name["human_oracle"]["pacing_agreement"],
            )


if __name__ == "__main__":
    unittest.main()