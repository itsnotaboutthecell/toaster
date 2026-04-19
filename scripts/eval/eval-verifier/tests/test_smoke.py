"""Smoke test for scripts/eval-verifier against the real parity fixtures.

Uses the mock backend (no network) to exercise scoring + tournament + the
pass/fail policy. Asserts:

  * The runner completes with exit code 0 on the synthetic fixtures.
  * For both ``phrase_alpha`` and ``phrase_bravo``, the tournament picks
    ``parakeet`` over ``whisper`` — Parakeet's timings are closer to the
    oracle and claim authoritative word timestamps, so every criterion
    prefers it. This is the minimum behavior we need from the harness
    before trusting it to pick between real backends.
  * Injecting a synthesized-timings regression (equal-duration synthesis)
    flips the tournament OR trips the pass/fail gate.
"""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


HERE = pathlib.Path(__file__).resolve().parent
PKG_DIR = HERE.parent
REPO_ROOT = PKG_DIR.parent.parent
RUNNER = PKG_DIR / "run_parity.py"


def _run(*extra, fixtures_dir=None, output_root=None):
    args = [sys.executable, str(RUNNER)]
    if fixtures_dir:
        args += ["--fixtures-dir", str(fixtures_dir)]
    if output_root:
        args += ["--output-root", str(output_root)]
    args += ["--backend", "mock", *extra]
    res = subprocess.run(args, capture_output=True, text=True, check=False)
    return res


class TestParitySmoke(unittest.TestCase):
    def test_mock_backend_picks_parakeet_on_real_fixtures(self):
        with tempfile.TemporaryDirectory() as tmp:
            res = _run(output_root=pathlib.Path(tmp))
            self.assertEqual(
                res.returncode,
                0,
                msg=f"runner failed: stdout={res.stdout}\nstderr={res.stderr}",
            )
            # Find the written report
            run_dirs = sorted(pathlib.Path(tmp).glob("*/report.json"))
            self.assertTrue(run_dirs, "no report.json emitted")
            report = json.loads(run_dirs[-1].read_text(encoding="utf-8"))
            self.assertEqual(report["status"], "pass")
            for fx in report["fixtures"]:
                self.assertEqual(
                    fx["winner"],
                    "parakeet",
                    msg=(
                        f"expected parakeet to win {fx['fixture']}, got "
                        f"{fx['winner']} (wins={fx['wins']})"
                    ),
                )
                self.assertEqual(fx["status"], "pass")

    def test_synthesized_timings_trip_gate(self):
        """Inject a fake backend with equal-duration synthesis and verify
        the runner fails or ranks it last."""

        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = pathlib.Path(tmp)
            # Copy fixtures dir structure
            import shutil

            src = REPO_ROOT / "src-tauri" / "tests" / "fixtures" / "parity"
            dst = tmp_path / "parity"
            shutil.copytree(src, dst)

            # Add a third "fake" backend with equal-duration synthesis.
            fake_dir = dst / "backend_outputs" / "fake_synth"
            fake_dir.mkdir(parents=True, exist_ok=True)
            for oracle_path in dst.glob("*.oracle.json"):
                oracle = json.loads(oracle_path.read_text())
                first = oracle[0]["start_us"]
                last = oracle[-1]["end_us"]
                n = len(oracle)
                each = (last - first) // n
                words = []
                for k, ow in enumerate(oracle):
                    words.append(
                        {
                            "text": ow["word"],
                            "start_us": first + k * each,
                            "end_us": first + (k + 1) * each,
                            "confidence": 0.99,
                        }
                    )
                result = {
                    "words": words,
                    "language": "en-US",
                    # Authoritative lie: claim native timings but emit synth.
                    "word_timestamps_authoritative": True,
                    "input_sample_rate_hz": 16000,
                }
                stem = oracle_path.name.replace(".oracle.json", "")
                (fake_dir / f"{stem}.result.json").write_text(json.dumps(result))

            output_root = tmp_path / "out"
            res = _run(fixtures_dir=dst, output_root=output_root)
            run_dirs = sorted(output_root.glob("*/report.json"))
            self.assertTrue(run_dirs, "no report.json emitted")
            report = json.loads(run_dirs[-1].read_text(encoding="utf-8"))
            # Fake backend should never win; parakeet should still win.
            for fx in report["fixtures"]:
                self.assertNotEqual(
                    fx["winner"],
                    "fake_synth",
                    msg=(
                        f"fake synthesized backend unexpectedly won "
                        f"{fx['fixture']}: wins={fx['wins']}"
                    ),
                )
                # Confirm fake_synth was ranked strictly lower than parakeet
                by_name = {t["backend"]: t for t in fx["trials"]}
                self.assertIn("fake_synth", by_name)
                self.assertIn("parakeet", by_name)
                self.assertLess(
                    by_name["fake_synth"]["wins"],
                    by_name["parakeet"]["wins"],
                    msg=(
                        f"fake_synth wins={by_name['fake_synth']['wins']} "
                        f">= parakeet wins={by_name['parakeet']['wins']} "
                        f"on {fx['fixture']}"
                    ),
                )


if __name__ == "__main__":
    unittest.main()
