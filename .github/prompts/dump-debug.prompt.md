---
name: dump-debug
description: Dump current Toaster debug state (settings, FFmpeg, project, caption styles)
agent: agent
tools:
  - execute/runInTerminal
  - execute/getTerminalOutput
---

# Dump Debug State

Run the diagnostic scripts and report findings:

1. Run `pwsh -NoProfile -File scripts/dev/dump-debug-state.ps1` to print current settings, FFmpeg status, and project state.
2. Run `pwsh -NoProfile -File scripts/dev/dump-caption-style.ps1` for ASS subtitle style reference and troubleshooting.
3. Summarize any anomalies or misconfigurations found.
