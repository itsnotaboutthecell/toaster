# AGENTS.md

Guidance for AI coding assistants working in this repository.

Toaster is a transcript-first video/audio editor ("edit video by editing text"), forked from Handy.  
Stack: Tauri 2.x (Rust backend) + React/TypeScript/Tailwind frontend.

## Core architecture

```text
src/                  React + TypeScript + Tailwind UI
  components/         editor/player/settings and shared UI
  stores/             Zustand state
  i18n/               localization files
src-tauri/src/        Rust backend
  managers/           business logic domains (audio/model/transcription/editor/media/export/project/history)
  commands/           Tauri command handlers
  audio_toolkit/      lower-level audio/VAD/text helpers
```

## Non-negotiable boundaries

- Backend managers own domain/business logic.
- Frontend calls Tauri commands and renders state/events.
- Keep-segment/time-mapping behavior must come from backend authority.
- Never swap the video element source to an audio preview file; keep original video rendering source and sync preview audio separately.

## Development commands

```bash
bun install --frozen-lockfile
cargo tauri dev
cargo tauri build
npm run dev
npm run build
cd src-tauri && cargo check
cd src-tauri && cargo test
cd src-tauri && cargo test test_filter_filler_words -- --nocapture
cd src-tauri && cargo clippy
npm run lint
```

## Launch protocol

When the user says **"launch toaster"** (or equivalent), enter live dev mode:

1. Run `.\scripts\setup-env.ps1` in the shell first.
2. Start the app with `.\scripts\launch-toaster-monitored.ps1 -ObservationSeconds 120` (async mode, keep running).
3. Monitor startup output for compilation errors, 404s, runtime panics, or failed initialization.
4. On failure signals, immediately gather logs and do first-line debugging before reporting status.
5. On success, report the app is running and stay ready to inspect logs on demand.

Do **not** use bare `npm run tauri dev` — always use the monitored launch script.

## Windows requirements

- Run `.\scripts\setup-env.ps1` in the same PowerShell session before Cargo/Tauri commands.
- Use MSVC Rust toolchain target (`stable-x86_64-pc-windows-msvc`), not GNU.
- Prefer running direct Cargo commands from `src-tauri\`.
- Stop running `toaster-app.exe`/`toaster.exe` before rebuilds to avoid DLL/link lock failures.

## Conventions

- Rust: run `cargo fmt` + `cargo clippy`; avoid `.unwrap()` in production paths.
- TypeScript: strict typing, no `any`, functional components.
- UI strings must use i18next keys.
- Backend timestamps use microseconds.

## Precision and UX guardrails

- Audio-edit acceptance gate: do not call timestamp/playback/export fixes complete until midstream deletions stay clean during replay (including long edits and delete/undo cycles) with no audible remnants.
- Preserve precise transcription timing (per-word/per-segment); never synthesize equal-duration timestamps.
- Detect actions must highlight only matched words; destructive actions (Delete) must apply only to that highlighted subset.
- For dark theme highlights, use high-contrast but low-noise styling (avoid hard-to-read red-on-dark combinations).

## Debugging tools

- `.\scripts\dump-debug-state.ps1` — Print current settings, FFmpeg status, and project state for diagnostics.
- `.\scripts\dump-caption-style.ps1` — ASS subtitle style reference and troubleshooting guide.

## Skills and agents

The following skills and agents are available under `.github/skills/` and `.github/agents/`.
Invoke them at the appropriate time — they are not optional suggestions.

### Required workflow skills

- **verification-before-completion** — Invoke before claiming ANY work is complete, fixed, or passing. No completion claims without fresh verification evidence (command output, not assumptions). This is non-negotiable.
- **systematic-debugging** — Invoke when encountering any bug, test failure, or unexpected behavior. Root cause investigation must complete before proposing fixes. No random guess-and-check.
- **test-driven-development** — Invoke when implementing any feature or bugfix. Write the failing test first, watch it fail, then write minimal code to pass. No production code without a failing test.
- **receiving-code-review** — Invoke when receiving code review feedback. Evaluate technically before implementing. No performative agreement or blind implementation.
- **canonical-instructions** — Invoke whenever editing an AI-instruction file (AGENTS.md, CLAUDE.md, .github/copilot-instructions.md, CRUSH.md). AGENTS.md is the single source of truth; other files are pointers.

### Build and environment

- **build-and-test** — Invoke for compile/test/lint runs, toolchain issues, and Windows build environment troubleshooting.
- **dep-hygiene** — Invoke before adding a dependency, after removing a module, and on any PR claiming "dead code cleanup". Enforces `cargo machete` / `knip` / `depcheck` gates.

### Legacy pruning and product scope

- **handy-legacy-pruning** — Invoke before editing any Handy-era dictation module (actions.rs, shortcut/, overlay.rs, tray*.rs, clipboard.rs, input.rs, audio_feedback.rs, apple_intelligence.rs, recorder.rs, vad/, PushToTalk.tsx, AudioFeedback.tsx, HandyKeysShortcutInput.tsx). Forces the "is this still on the transcript-editor path?" question before extending dead code.
- **i18n-pruning** — Invoke when deleting or renaming any i18next key. Ensures all 22 locale files stay in sync.
- **transcript-precision-eval** — Invoke when touching word operations, keep-segment logic, time mapping, transcription post-processing, or export.

### Review and audit agents

- **code-reviewer** — Invoke after completing a major feature, fix, or project step. Reviews implementation against the original plan, architecture boundaries, and coding standards. Catches plan deviations, boundary violations, and missing verification.
- **repo-auditor** — Invoke for whole-repository health audits (dead modules, monoliths, instruction drift, dep bloat, workflow duplication). Complements code-reviewer, which is diff-scoped.
- **eval-harness-runner** — Invoke to run the precision / midstream / export evals with one command and produce a pass/fail JSON for CI.
