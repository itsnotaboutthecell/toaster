# Copilot CLI hooks

Tool-call enforcement for the rules in [`AGENTS.md`](../../AGENTS.md). These
hooks run in the local Copilot CLI only; they are not CI. Per the
`canonical-instructions` skill, AGENTS.md is the source of truth — this
README only lists hooks and bypasses.

## Hooks

| Hook | File | Purpose |
| --- | --- | --- |
| `sessionStart` | `session-start.{ps1,sh}` | Observe session; warn if `toaster-app.exe`/`toaster.exe` is running (AGENTS.md rebuild-lock); note `launch toaster` prompts. |
| `sessionEnd`   | `session-end.{ps1,sh}`   | Append end-marker to log. |
| `userPromptSubmitted` | `log-prompt.{ps1,sh}` | Audit log of prompts + trigger phrases. **Output ignored by CLI — observational only.** |
| `preToolUse`   | `pretool-policy.{ps1,sh}` | Hard gates (see below). |
| `postToolUse`  | `posttool-reminders.{ps1,sh}` | Stderr reminders for `.rs` (cargo fmt) and locale JSON (check-translations). |

Log file: `~/.copilot/toaster-prompts.log` (`$env:USERPROFILE\.copilot\toaster-prompts.log` on Windows).

## preToolUse gates (all cite AGENTS.md)

1. **Bare tauri dev launches.** Denies `cargo tauri dev` / `npm run tauri dev`
   unless wrapped by `launch-toaster-monitored`. AGENTS.md → *Launch protocol*.
2. **Name-based process kills.** Denies `Stop-Process -Name`, `taskkill /IM`
   (with/without `/F`), `pkill -f`. AGENTS.md → *Windows requirements*.
   **No bypass.**
3. **Unscoped full-workspace cargo clippy/check.** Denies `cargo clippy` /
   `cargo check` without `-p <crate>` or `--package <crate>`. AGENTS.md →
   *Cargo runtime expectations*.
4. **New `*.md` at repo root.** Denies `create` for new root-level markdown
   outside the known allowlist (`AGENTS.md`, `CLAUDE.md`, `CRUSH.md`,
   `CONTRIBUTING.md`, `CONTRIBUTING_TRANSLATIONS.md`, `README.md`,
   `SECURITY.md`, `PRD.md`). Paths under `docs/`, `.github/`, `eval/`,
   `src-tauri/tests/fixtures/`, or the session workspace are unaffected.

## Bypass env vars

Set in the calling shell only when you consciously need the override:

| Variable | Gate it bypasses | When to use |
| --- | --- | --- |
| `COPILOT_ALLOW_BARE_TAURI_DEV=1` | Gate 1 | You already have monitoring in place another way. |
| `COPILOT_ALLOW_FULL_CLIPPY=1`    | Gate 3 | Milestone sweep (see AGENTS.md — "at most once per milestone"). |
| `COPILOT_ALLOW_ROOT_MARKDOWN=1`  | Gate 4 | Adding a new canonical root doc agreed with maintainers. |

Gate 2 has no bypass per AGENTS.md.

## Disabling

Rename or remove `.github/hooks/hooks.json` — the CLI no-ops when it is
missing.

## Local smoke test

From `.github/hooks/`, feed each script the documented shape and confirm
stdout is empty on allow and a single-line JSON on deny.

Example — bare tauri dev should be denied:

```powershell
echo '{"toolName":"bash","toolArgs":"{\"command\":\"cargo tauri dev\"}"}' | pwsh -File pretool-policy.ps1
```

Expected stdout:

```
{"permissionDecision":"deny","permissionDecisionReason":"AGENTS.md: use .\\scripts\\launch-toaster-monitored.ps1 ..."}
```

Allow path (monitored wrapper) — stdout empty, exit 0:

```powershell
echo '{"toolName":"bash","toolArgs":"{\"command\":\".\\\\scripts\\\\launch-toaster-monitored.ps1\"}"}' | pwsh -File pretool-policy.ps1
```

## Note on `userPromptSubmitted`

The CLI ignores this hook's stdout. It is purely observational — prompt
logging and trigger-phrase detection do **not** auto-run anything. All
behavioural guarantees come from `preToolUse` denial, which is
platform-independent via the `bash`/`powershell` entries in `hooks.json`.
