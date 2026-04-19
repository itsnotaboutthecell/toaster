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
2. Start the app with `.\scripts\launch-toaster-monitored.ps1 -ObservationSeconds 300` (async mode, keep running).
3. Monitor startup output for compilation errors, 404s, runtime panics, or failed initialization.
4. On failure signals, immediately gather logs and do first-line debugging before reporting status.
5. On success, report the app is running and stay ready to inspect logs on demand.

Do **not** use bare `npm run tauri dev` — always use the monitored launch script.
