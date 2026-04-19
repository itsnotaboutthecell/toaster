---
name: run-eval
description: Run the full Toaster eval harness (precision, boundary, parity)
agent: eval-harness-runner
tools:
  - execute/runInTerminal
  - execute/getTerminalOutput
  - read/readFile
---

# Run Eval Harness

Execute the full evaluation suite in order and report results.

1. Set up the Windows build environment: `. .\scripts\setup-env.ps1 *>&1 | Out-Null`
2. Run precision eval: `pwsh -NoProfile -File scripts/eval/eval-edit-quality.ps1`
3. Run boundary eval: `pwsh -NoProfile -File scripts/eval/eval-audio-boundary.ps1`
4. Run export parity eval: `pwsh -NoProfile -File scripts/eval/eval-multi-backend-parity.ps1`
5. Summarize pass/fail status for each eval. Stop on first infrastructure failure.
