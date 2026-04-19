# AGENTS.md

Guidance for AI coding assistants working in this repository.

Toaster is a transcript-first video/audio editor ("edit video by editing text"), forked from Handy.
Stack: Tauri 2.x (Rust 1.82+), React 18, TypeScript 5, Vite 6, Tailwind CSS 3, Zustand 5. FFmpeg 7 for export. whisper-rs for local ASR.

Path-scoped conventions live in nested `AGENTS.md` files — nearest-file wins per the [agents.md spec](https://agents.md/):

- [`src-tauri/AGENTS.md`](src-tauri/AGENTS.md) — Rust backend (cargo commands, Windows env, DLL pitfalls, style).
- [`src/AGENTS.md`](src/AGENTS.md) — frontend (TypeScript, React, Tailwind, i18n, bindings.ts, settings UI).

## How this file is structured

Structured, frequently-referenced data (rules, commands, testing matrix,
boundaries, hygiene steps, skills & agents index) lives in
[`.github/registry/`](.github/registry/) as JSON and is queryable via a
reader CLI:

```bash
bun scripts/registry/reader.ts list                    # section names
bun scripts/registry/reader.ts rules --verb NEVER      # filtered rules
bun scripts/registry/reader.ts commands --tier fast    # iteration commands
bun scripts/registry/reader.ts render commands         # markdown render
```

Narrative, architecture, and load-bearing **critical rules** stay in this
file and in the nested AGENTS.md files. Rendered human-readable versions of
the registry live under [`docs/`](docs/).

## Critical rules (non-negotiable)

These stay inlined on purpose: they're the rules that get violated most and
cost the most when missed. Full verb-indexed rule list:
`bun scripts/registry/reader.ts rules`.

- **NEVER** make runtime network calls to hosted ASR / LLM / caption APIs — Toaster is local-only inference.
- **NEVER** swap the `<video>` element source to an audio preview file. Sync preview audio independently against the original video stream.
- **NEVER** duplicate dual-path logic (caption layout, word grouping, filler lists, keep-segments, time mapping). One backend implementation; two consumers.
- **NEVER** retry a failing > 2 min cargo command a third time with the same incantation — pivot (`cargo clean -p toaster`, scope swap, live-app, or ask) and log it under `plan.md` > Retry log (see hygiene R4).
- **ALWAYS** invoke cargo from a single PowerShell call: `. .\scripts\setup-env.ps1 *>&1 | Out-Null; cd src-tauri; cargo <cmd>`. Env does not persist across tool calls.
- **ALWAYS** verify audio/caption/preview/export fixes through the monitored live app (`scripts\launch-toaster-monitored.ps1`) or a fixture-based eval. A green `cargo check` is not verification.
- **ALWAYS** mirror every i18next key across all 20 locale files (`bun scripts/check-translations.ts`).
- **ALWAYS** respect the 800-line file cap for `.rs` / `.ts` / `.tsx` under `src/` and `src-tauri/src/` (`bun run check:file-sizes`). Split, don't allowlist.
- **FORBIDDEN:** hand-editing `src/bindings.ts` beyond a temporary one-line union patch — it is specta-generated (see [`src/AGENTS.md`](src/AGENTS.md)).

### Launch shortcut

When the user says `launch toaster [duration]` in chat, run `scripts\launch-toaster-monitored.ps1 -Duration <duration>`:

- Bare `launch toaster` → no `-Duration` (defaults to 5 minutes).
- `launch toaster <N><unit>` → pass through as `-Duration <N><unit>`, where `unit` is `ms | s | m | h` (e.g. `10m`, `1h`, `30s`, `500ms`). The launcher clamps below 5 s and caps at 4 h.
- Anything else (e.g. `launch toaster 10` with no unit, or `launch toaster 10foo`) → ask before invoking. Do not guess.

Do NOT invent other parameter names (`-DurationMinutes`, etc.) — the launcher accepts only `-Duration` (string) and `-ObservationSeconds` (int, back-compat); unknown params hang silently in async shells.

## Core architecture

```text
src/                  React + TypeScript + Tailwind UI
  components/         editor/player/settings and shared UI
  stores/             Zustand state
  i18n/               localization files
src-tauri/src/        Rust backend
  managers/           business logic domains (captions/cleanup/editor/export/filler/llm/media/model/project/splice/transcription)
  commands/           Tauri command handlers (thin IPC wrappers)
  audio_toolkit/      lower-level audio/forced-alignment/text helpers
                      (incl. `vad/` — reintroduced per R-002/R-003/R-004,
                      see `features/reintroduce-silero-vad/`)
```

Full tree: [`docs/repo-layout.md`](docs/repo-layout.md).

## Where to find things (registry + docs)

| Topic | How to reach it |
|-------|-----------------|
| Full rules list | `bun scripts/registry/reader.ts rules` |
| Commands (fast/full/live) | [`docs/commands.md`](docs/commands.md) or `bun scripts/registry/reader.ts commands --tier <fast\|full\|live>` |
| Design system (tokens, anatomy, CI gates) | [`docs/design-system.md`](docs/design-system.md) + `.github/skills/design-system/` |
| Testing matrix | `bun scripts/registry/reader.ts testing` |
| Boundaries (always/ask/never) | `bun scripts/registry/reader.ts boundaries` |
| Session hygiene R1–R5 | `bun scripts/registry/reader.ts hygiene` |
| PR verification gates | `bun scripts/registry/reader.ts verification` |
| Skills index | `bun scripts/registry/reader.ts skills` |
| Agents index | `bun scripts/registry/reader.ts agent <name>` / `skills` |
| Rust conventions + Windows env | [`src-tauri/AGENTS.md`](src-tauri/AGENTS.md) |
| Frontend conventions | [`src/AGENTS.md`](src/AGENTS.md) |
| Spec-driven development | [`docs/spec-driven.md`](docs/spec-driven.md) |
| Repository tree | [`docs/repo-layout.md`](docs/repo-layout.md) |
| Code-review addendum | [`.github/instructions/code-review.instructions.md`](.github/instructions/code-review.instructions.md) |

## Skills and agents

Toaster consumes two skill sources:

- **Upstream `superpowers:` skills** — discipline/workflow skills from [obra/superpowers](https://github.com/obra/superpowers), installed via the CLI plugin marketplace. Invoke by name (e.g. `superpowers:verification-before-completion`).
- **Local Toaster skills** in [`.github/skills/`](.github/skills/) — domain-specific gates. Invoke by short name (e.g. `transcript-precision-eval`).

Skills are not optional. If a skill applies to what you're doing, you must invoke it. User instructions in this file override skills where they conflict — per `superpowers:using-superpowers`.

Full list with descriptions:

```bash
bun scripts/registry/reader.ts skills
bun scripts/registry/reader.ts agents
```

Skill & agent indexes are auto-generated from `.github/skills/<name>/SKILL.md` and `.github/agents/<name>.agent.md` frontmatter by `scripts/registry/build.ts`. CI drift is gated by `scripts/registry/check.ts`.

When `superpowers:code-reviewer` reviews a Toaster PR, it must also apply the addendum in [`code-review.instructions.md`](.github/instructions/code-review.instructions.md). Architecture boundary violations, dual-path duplication, hosted-inference dependencies, and missing verification evidence are **Critical** findings that block merge.

## Precision and UX guardrails

- Audio-edit acceptance gate: do not call timestamp/playback/export fixes complete until midstream deletions stay clean during replay (including long edits and delete/undo cycles) with no audible remnants.
- Preserve precise transcription timing (per-word/per-segment); never synthesize equal-duration timestamps.
- Detect actions must highlight only matched words; destructive actions (Delete) must apply only to that highlighted subset.
- For dark theme highlights, use high-contrast but low-noise styling (avoid hard-to-read red-on-dark combinations).
- "Verified" = live app or fixture-based eval, not `cargo check`. Cite the command run and observed behavior in PR bodies.

## Git workflow & PR conventions

- **Branching:** feature branches off `main`. One feature per branch, one feature per session (see hygiene R1).
- **Commits:** imperative mood, scoped prefix preferred (`editor: fix midstream undo`, `captions: align export with preview`). Include the `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>` trailer.
- **PR title:** `[<slug>] <Description>` where `<slug>` matches `features/<slug>/` if one exists, or a short noun otherwise.
- **PR body:** must cite at least one `AC-NNN-x` from `features/<slug>/PRD.md` if the change implements a tracked feature. For untracked hotfixes, describe the repro and verification evidence.
- **PR body MUST show verification evidence**, not claims. Full gate list: `bun scripts/registry/reader.ts verification`.
- **Do NOT** add "Generated with Claude Code" or similar provenance footers; only the `Co-authored-by: Copilot` trailer above.
- **Do NOT** mark PRs "ready for review" (`gh pr ready`). Leave in draft; the user promotes.
- Full verification checklist: [`code-review.instructions.md`](.github/instructions/code-review.instructions.md).

## Session & workspace hygiene

Toaster's session-state (`~/.copilot/session-state/<id>/`) is cheap to spin up and expensive to let grow. These rules exist because the same session has repeatedly stacked 6+ unrelated features, accumulated 70+ todos, drifted `plan.md` 2+ hours from reality, and burned 15+ minute cargo rebuild loops.

- **R1. One feature per session** — if `plan.md`'s current-focus slug would need to change, it's a new session.
- **R2. Todo hygiene at every checkpoint** — reap stale rows with [`scripts/sql/reap-stale-todos.sql`](scripts/sql/reap-stale-todos.sql); keep active count under ~15.
- **R3. `plan.md` current-focus is mandatory and time-stamped** — rewrite the 5-line block per [`.github/templates/plan-active-work.md`](.github/templates/plan-active-work.md) every checkpoint. If the conversation summary is newer than `plan.md` by > 30 min, first act on resume is to reconcile.
- **R4. Retry budget on long cargo commands** — a cargo/tauri command > 2 min failing with the same error hash is not re-run a third time without a strategy change. Log pivots per [`.github/templates/retry-log-entry.md`](.github/templates/retry-log-entry.md).
- **R5. Async shell accounting** — before ending any turn with an async shell still running, either `stop_powershell` it OR record `shellId` + PID + purpose under `plan.md` > Live shells. First act on resume: `list_powershell` and reap orphans.

Open a fresh session if any of: a different feature slug; `todos` > 30 rows; `plan.md` timestamp > 4 hours old on an active day; conversation auto-compacted more than once; > 2 async shells running.

Full rule detail + triggers/checks/fixes: `bun scripts/registry/reader.ts hygiene`.

## Hooks

Tool-call enforcement for the rules above lives in [`.github/hooks/`](.github/hooks/). See `.github/hooks/README.md` for the list and bypass env vars.
