---
name: launch-toaster
description: Launch Toaster in monitored dev mode with full Windows env setup
agent: agent
tools:
  - execute/runInTerminal
  - execute/getTerminalOutput
---

# Launch Toaster

Enter live dev mode by running the monitored launch script.

1. Run `.\scripts\setup-env.ps1` in the shell first.
2. Start the app with `.\scripts\launch-toaster-monitored.ps1 -Duration 5m` (async mode, keep running).
   - The user may say "launch toaster <N><unit>" where `unit` is `ms | s | m | h` (e.g. `10m`, `1h`, `30s`). Translate directly to `-Duration <N><unit>`.
   - Bare "launch toaster" defaults to `5m`. Anything ambiguous (missing unit, typo) → ask, do not guess.
   - The launcher caps at 4 h; below 5 s it clamps with a warning.
3. Monitor startup output for compilation errors, 404s, runtime panics, or failed initialization.
4. On failure signals, immediately gather logs and do first-line debugging before reporting status.
5. On success, report the app is running and stay ready to inspect logs on demand.

Do **not** use bare `npm run tauri dev` — always use the monitored launch script.
Do **not** invent parameter names like `-DurationMinutes`; the launcher accepts only `-Duration` (string) and `-ObservationSeconds` (int, legacy).
