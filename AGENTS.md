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
  managers/           business logic domains (audio/model/transcription/editor/media/export/project)
  commands/           Tauri command handlers
  audio_toolkit/      lower-level audio/VAD/text helpers
```

## Repository layout

Authoritative top-down map. Re-read this before grep-storming for "where does X live?".

```text
toaster/
├── AGENTS.md                  # canonical agent guidance (this file)
├── CLAUDE.md / CRUSH.md       # pointer files → AGENTS.md (per canonical-instructions skill)
├── README.md / LICENSE        # standard GitHub root files
├── SECURITY.md / CONTRIBUTING.md / CONTRIBUTING_TRANSLATIONS.md
├── PRD.md                     # product requirements (transcript-first editor scope)
├── docs/
│   ├── build.md               # platform build setup (was BUILD.md)
│   └── build-macos.md         # macOS NSPanel / private API notes
├── eval/                      # evaluation ecosystem
│   ├── fixtures/              # committed fixture media (mp4/png) — see eval/fixtures/README.md
│   └── output/                # gitignored eval run outputs (audio-boundary/, multi-backend-parity/)
├── features/                  # spec-driven planning bundles (see "Spec-driven development" below)
│   └── .templates/            # starter templates for REQUEST/PRD/BLUEPRINT/coverage/tasks
├── scripts/                   # PowerShell tooling
│   ├── setup-env.ps1          # MSVC + LLVM + Vulkan env (run first on Windows)
│   ├── scaffold-feature.ps1   # create features/<slug>/ from templates
│   ├── promote-feature.ps1   # coverage + tasks.sql gates; promotion to "planned"
│   ├── launch-toaster-monitored.ps1
│   ├── eval-edit-quality.ps1
│   ├── eval-audio-boundary.ps1
│   ├── eval-multi-backend-parity.ps1
│   ├── check-feature-coverage.ps1  # PM coverage gate (every AC -> verifier)
│   ├── check-feature-tasks.ps1     # tasks.sql schema gate (columns/statuses)
│   ├── feature-board.ps1           # terminal Kanban over features/*/STATE.md
│   └── lib/                   # shared PS modules (AudioBoundary.psm1, ...)
├── src/                       # React + TypeScript + Tailwind frontend
│   ├── App.tsx
│   ├── bindings.ts            # generated Tauri command bindings (do not hand-edit)
│   ├── components/            # editor/, player/, settings/, shared/
│   ├── stores/                # Zustand state
│   ├── lib/                   # frontend utilities + types
│   └── i18n/locales/          # 20 locale files, gated by check-translations.ts
├── src-tauri/                 # Rust backend (Tauri 2.x)
│   ├── Cargo.toml / tauri.conf.json
│   ├── src/
│   │   ├── lib.rs             # app entry, plugin registration
│   │   ├── audio_toolkit/     # timing, forced_alignment, vad/, text helpers, constants
│   │   ├── commands/          # Tauri command handlers (transcribe_file/, waveform/, ...)
│   │   └── managers/          # business logic
│   │       ├── transcription/ # adapter trait + backend implementations
│   │       ├── editor/        # keep-segments, time mapping (backend authority)
│   │       ├── cleanup/       # filler-word removal, post-processing
│   │       ├── model/         # ASR model lifecycle
│   │       ├── project/       # project save/load
│   │       └── export/        # FFmpeg-driven render pipeline
│   └── tests/                 # Rust integration tests
│       └── fixtures/          # alignment/, boundary/, parity/, mock_transcription_sample.json
├── tests/                     # Playwright E2E (app.spec.ts, skipSchedule.spec.ts)
├── nix/                       # Nix module variants (hm-module.nix, module.nix)
├── .nix/                      # bun2nix output (bun.nix, bun-lock-hash) — tracked
├── flake.nix / flake.lock     # root Nix flake (convention: stays at root)
└── .github/
    ├── skills/                # project skills — invoke per AGENTS.md "Skills and agents"
    ├── agents/                # project agents (repo-auditor, eval-harness-runner, waveform-diff, cut-drift-fuzzer, toaster-review-addendum)
    └── workflows/             # CI
```

## Non-negotiable boundaries

- Backend managers own domain/business logic.
- Frontend calls Tauri commands and renders state/events.
- Keep-segment/time-mapping behavior must come from backend authority.
- Never swap the video element source to an audio preview file; keep original video rendering source and sync preview audio separately.
- **Single source of truth for dual-path logic.** Any rendering or logic that lives on both the preview path (React) and the export path (FFmpeg/Rust) — caption layout and sizing, word grouping, filler/allow word lists, keep-segments, time mapping — must have one authoritative implementation in the backend, consumed verbatim by both paths. Duplicating it in the frontend (or hardcoding a list in Rust that also exists in the UI) is a defect, not a shortcut. The caption preview↔export mismatch and the hardcoded filler list both came from violating this rule.
- **Local-only inference.** Toaster performs all transcription and cleanup locally. No runtime network calls to hosted LLM/transcription/caption APIs. Adding a dependency that phones home — or a feature flag that enables one — is a breaking product change and requires explicit approval before landing.

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
2. Start the app with `.\scripts\launch-toaster-monitored.ps1 -ObservationSeconds 300` (async mode, keep running).
3. Monitor startup output for compilation errors, 404s, runtime panics, or failed initialization.
4. On failure signals, immediately gather logs and do first-line debugging before reporting status.
5. On success, report the app is running and stay ready to inspect logs on demand.

Do **not** use bare `npm run tauri dev` — always use the monitored launch script.

## Windows requirements

- Run `.\scripts\setup-env.ps1` in the same PowerShell session before Cargo/Tauri commands.
- Use MSVC Rust toolchain target (`stable-x86_64-pc-windows-msvc`), not GNU.
- Prefer running direct Cargo commands from `src-tauri\`.
- Stop running `toaster-app.exe`/`toaster.exe` before rebuilds to avoid DLL/link lock failures.

## Cargo runtime expectations

Cold full-workspace `cargo clippy` / `cargo check` on this repo's dependency tree (whisper-rs-sys + ffmpeg-sys + the Tauri stack) on Windows MSVC routinely takes **2–10+ minutes per invocation**. A running cargo process under 10 minutes is almost certainly compiling, not hung.

- During iteration, scope cargo runs narrowly: `cargo check -p <crate>`, `cargo clippy -p <crate> --lib`, or a single `cargo test <name>`.
- Run the full-workspace `cargo check` + `cargo clippy` sweep at most once per milestone, not after every small edit.
- Do not cancel, retry, or re-invoke a cargo run under 10 minutes unless there is concrete evidence of a deadlock (e.g., zero CPU for minutes, stuck linker lock on a known-running `toaster-app.exe`).
- If a long cargo run exhausts your time budget, report its state honestly rather than silently starting another one — this is what caused the 20-minute stall in session history.

## Conventions

- Rust: run `cargo fmt` + `cargo clippy`; avoid `.unwrap()` in production paths.
- TypeScript: strict typing, no `any`, functional components.
- UI strings must use i18next keys.
- Backend timestamps use microseconds.
- **File-size cap: 800 lines** for `.rs` / `.ts` / `.tsx` under `src/` and `src-tauri/src/` (excluding generated `bindings.ts`). Enforced by `bun run check:file-sizes` in CI. Existing offenders are grandfathered via `scripts/file-size-allowlist.txt`; the monolith-split plan removes entries as each file is carved up. Do not add new entries without an approved tracking issue — split the file instead.

## Precision and UX guardrails

- Audio-edit acceptance gate: do not call timestamp/playback/export fixes complete until midstream deletions stay clean during replay (including long edits and delete/undo cycles) with no audible remnants.
- Preserve precise transcription timing (per-word/per-segment); never synthesize equal-duration timestamps.
- Detect actions must highlight only matched words; destructive actions (Delete) must apply only to that highlighted subset.
- For dark theme highlights, use high-contrast but low-noise styling (avoid hard-to-read red-on-dark combinations).

### "Verified" means the live app, not `cargo check`

For any fix touching **audio edits, captions, preview↔export parity, or timeline rendering**, "done" / "fixed" / "verified" requires driving the exact failing input through the monitored live app (`scripts\launch-toaster-monitored.ps1`) or the fixture-based precision eval — not merely a successful compile/clippy/unit-test run. In the completion message, cite the command that was run and the observed behavior. Completion claims that skip this step have repeatedly turned out to be wrong ("precision edits are lying to you", caption export regressions, cleanup not deleting detected words) and erode user trust.

### Settings UI contract

- Every user-exposed setting must render a **human-readable label and one-line description**. Never surface raw flag or enum names (e.g. no `caption_bg_opacity_b3` — write "Background transparency" with a plain-language description).
- Numeric controls: sliders must support **smooth drag AND double-click-to-type keyboard entry**. Do not ship spinner up/down arrows as the primary editing affordance.
- Use existing color tokens (rest state `#EEEEEE`, accent orange on hover, etc.); do not invent new greys/reds. Never place red text on dark backgrounds or light-grey text on white — both have recurred as readability bugs.

## Debugging tools

- `.\scripts\dump-debug-state.ps1` — Print current settings, FFmpeg status, and project state for diagnostics.
- `.\scripts\dump-caption-style.ps1` — ASS subtitle style reference and troubleshooting guide.

## Skills and agents

Toaster consumes two skill sources:

- **Upstream `superpowers:` skills** — general-purpose discipline and workflow skills from [obra/superpowers](https://github.com/obra/superpowers), installed via the CLI plugin marketplace. Invoke by name (e.g. `superpowers:verification-before-completion`). No files for these live in this repo.
- **Local Toaster skills** under `.github/skills/` — domain-specific gates that superpowers explicitly wants kept out of core and in a companion plugin. Invoke by short name (e.g. `transcript-precision-eval`).

Skills are not optional. If a skill applies to what you're doing, you must invoke it. User instructions in this file override skills where they conflict — per `superpowers:using-superpowers`.

### Upstream superpowers skills (invoke by `superpowers:<name>`)

**Discipline gates — apply per-trigger:**

- `superpowers:verification-before-completion` — Invoke before claiming ANY work is complete, fixed, or passing. Evidence before assertions. See the Toaster extension [Verified means the live app, not `cargo check`](#verified-means-the-live-app-not-cargo-check) below.
- `superpowers:systematic-debugging` — Invoke for any bug, test failure, build break, or unexpected behavior. Root cause before fixes; three-strike rule escalates to "question the architecture."
- `superpowers:test-driven-development` — Invoke when implementing backend features or bugfixes. Narrowed Toaster scope — see [Toaster TDD scope](#toaster-tdd-scope) below.
- `superpowers:receiving-code-review` — Invoke when receiving review feedback. Technical evaluation, no performative agreement. Toaster architecture boundaries (in the [code-review](#code-review-boundaries) table below) are additional hard rules.
- `superpowers:requesting-code-review` — Invoke when self-reviewing before asking for review.

**Planning and execution workflow:**

- `superpowers:brainstorming` — Invoke before implementing any feature larger than a bugfix. Produces a design doc before code.
- `superpowers:writing-plans` — Invoke when converting a spec into bite-sized (2–5 minute) implementation tasks.
- `superpowers:executing-plans` — Inline batch execution with human checkpoints.
- `superpowers:subagent-driven-development` — Fresh subagent per task with two-stage review (spec compliance, then code quality). Preferred for non-trivial plans.
- `superpowers:dispatching-parallel-agents` — Concurrent subagent workflows.
- `superpowers:using-git-worktrees` — Parallel branch isolation for larger features.
- `superpowers:finishing-a-development-branch` — Merge / PR / discard decision workflow.

**Meta:**

- `superpowers:writing-skills` — Invoke when creating or modifying any skill in this repo.
- `superpowers:using-superpowers` — Establishes skill-invocation discipline. "1% chance a skill might apply → you MUST invoke it."

### Local Toaster-specific skills

- **canonical-instructions** — Invoke whenever editing an AI-instruction file (AGENTS.md, CLAUDE.md, .github/copilot-instructions.md, CRUSH.md). AGENTS.md is the single source of truth; other files are pointers.
- **handy-legacy-pruning** — Invoke before editing any Handy-era dictation module (actions.rs, shortcut/, overlay.rs, tray*.rs, clipboard.rs, input.rs, audio_feedback.rs, apple_intelligence.rs, recorder.rs, vad/, PushToTalk.tsx, AudioFeedback.tsx, HandyKeysShortcutInput.tsx). Forces the "is this still on the transcript-editor path?" question before extending dead code.
- **dep-hygiene** — Invoke before adding or removing a Rust crate or npm package, after deleting a module, and on any "dead code cleanup" PR. Enforces `cargo machete` / `knip` / `depcheck` gates.
- **i18n-pruning** — Invoke when deleting, renaming, or adding a user-visible i18next key. Keeps all 22 locale files in sync with `scripts/check-translations.ts`.
- **transcript-precision-eval** — Invoke when touching word operations, keep-segment logic, time mapping, transcription post-processing, or export.
- **audio-boundary-eval** — Invoke on any PR that modifies `managers/editor`, `commands/waveform`, export splice logic, preview audio rendering, or boundary snapping. Extends `transcript-precision-eval` with seam-level gates: cross-seam leakage `xcorr < 0.15` over 0–80 ms, click-free seams (`z < 4.0`), and preview↔export within 1 sample / `-40 dBFS` RMS.
- **transcription-adapter-contract** — Invoke before merging any PR that adds or swaps an ASR / forced-alignment backend. Enforces the `NormalizedTranscriptionResult` schema (monotonic non-overlap, no zero-duration words, stripped non-speech tokens, no silent equal-duration synthesis) and requires a round-trip fixture test that keeps precision + boundary gates green with the new backend.
- **feature-pm** — Invoke whenever a user request would otherwise jump straight into code without a PRD/Blueprint/coverage map (new features, multi-file refactors, migrations, related-bug-fix batches). Forces invocation of the `product-manager` agent to produce `features/<slug>/` planning artifacts and a machine-checkable `coverage.json` before any production edit. Toaster's spec-driven discipline; afkode-inspired.

Build, lint, and test command reference lives in [Development commands](#development-commands) and [Windows requirements](#windows-requirements) above; see `docs/build.md` for Windows toolchain troubleshooting.

### Toaster TDD scope

`superpowers:test-driven-development` requires a failing test before production code. Toaster's harness reality narrows this:

- **Backend (`src-tauri/`):** full TDD applies — write a failing `#[test]` (or extend an eval fixture under `src-tauri/tests/fixtures/`) first. Verify with `cargo test`.
- **Audio / timeline / export:** the real gate is the fixture-based eval harness (`transcript-precision-eval`, `audio-boundary-eval`). Extend fixtures first, run the relevant eval script, then implement.
- **Frontend-only UI / styling:** no unit-test framework exists. `npm run lint`, `npm run build`, and a live-app check per `superpowers:verification-before-completion` are the gates. Playwright E2E for user-visible flow changes.

### Code-review boundaries

When `superpowers:code-reviewer` reviews a Toaster PR, it must also apply `.github/agents/toaster-review-addendum.md`. Architecture boundary violations, dual-path duplication, hosted-inference dependencies, and missing verification evidence are **Critical** findings that block merge.

### Local Toaster-specific agents

- **repo-auditor** — Invoke for whole-repository health audits (dead modules, monoliths, instruction drift, dep bloat, workflow duplication). Complements diff-scoped reviews.
- **eval-harness-runner** — Invoke to run the precision / midstream / export evals with one command and produce a pass/fail JSON for CI.
- **waveform-diff** — Invoke after audio-path milestones, or when a bug report sounds like "tiny remnants / clicks / drift". Renders preview and export to PCM, measures seam neighborhoods at sample level (cross-correlation, HF-burst energy, sample discontinuity, preview↔export parity), and emits JSON + human-readable findings. Does not fix code; reports only.
- **cut-drift-fuzzer** — Invoke before merging any edit-engine / time-mapping / undo-redo / export change. Runs seeded deterministic sequences (1000 ops) over synthetic beacon and real fixtures, asserting monotonic time maps, no cumulative duration drift (≤ 21 µs), no panics, and beacon preservation within 1 sample on PCM export. Emits pass/fail JSON.
- **toaster-review-addendum** (not an agent, consumed by `superpowers:code-reviewer`) — Toaster-specific architecture boundaries, verification gates, and hygiene rules the generic reviewer layers on top of its protocol.
- **product-manager** — Invoke (via the `feature-pm` skill) to turn an informal feature request into a complete `features/<slug>/` planning bundle: REQUEST (six-element) → PRD with `R-NNN` / `AC-NNN-x` IDs → Blueprint → task graph (`tasks.sql`) → per-task curated context briefings → `coverage.json` mapping every AC to a real verifier (skill / agent / cargo-test / script / live-app). Hands off to `superpowers:executing-plans` once `scripts/check-feature-coverage.ps1` is green. Does not write production code. **Dispatch via `agent_type: general-purpose` with `.github/agents/product-manager.md` inlined into the prompt, not `agent_type: product-manager` — the latter is registered in this CLI with a `view`-only tool restriction that makes file creation impossible (six documented narrate-instead-of-write failures).**

The previous local `code-reviewer` agent has been removed in favor of `superpowers:code-reviewer` + the addendum above.

## Hooks

Tool-call enforcement for the rules above lives in .github/hooks/. See .github/hooks/README.md for the list and bypass env vars.

## Spec-driven development (Product Manager agent)

Toaster runs an afkode-inspired ([afkode.ai/docs](https://afkode.ai/docs)) spec-driven loop on top of the superpowers chain. Any work above a single-file fix should go through it.

### Lifecycle

```
Define -> Plan -> Execute -> Review -> Ship
 user    PM       superpowers:        superpowers:    finishing-a-
 (slug + agent    executing-plans /   code-reviewer + development-
 6-elt           subagent-driven-     toaster-review-  branch
 REQUEST)        development           addendum
```

State lives in `features/<slug>/STATE.md`, one of: `defined`, `planned`, `executing`, `reviewing`, `shipped`, `archived`. Run `pwsh scripts/feature-board.ps1` for the terminal Kanban.

### Per-feature artifacts

Under `features/<slug>/`:

| File | Purpose | Tracked? |
|------|---------|----------|
| `STATE.md` | Lifecycle state (single line) | yes |
| `REQUEST.md` | Six-element user request (Problem & Goals / Outcome & AC / Scope / Code refs / Edge cases / Data model) | yes |
| `PRD.md` | Requirements with `R-NNN` IDs and `AC-NNN-x` acceptance criteria | yes |
| `BLUEPRINT.md` | Architecture decisions per R-ID, single-source-of-truth placement, risk register | yes |
| `tasks.sql` | `INSERT INTO todos / todo_deps` for the session SQL store | yes |
| `coverage.json` | Every AC -> verifier (skill / agent / cargo-test / script / manual live-app) | yes |
| `journal.md` | Operational journal (gitignored except for the example) | no |
| `tasks/<id>/context.md` | Curated per-task briefing for fresh subagents (gitignored except for the example) | no |

The `feature-pm` skill + `product-manager` agent generate this bundle; see [`features/example-pm-dryrun/`](features/example-pm-dryrun/) for a worked reference.

### Coverage gate

`scripts/check-feature-coverage.ps1 -Feature <slug>` (or `-All` in CI) verifies every `AC-NNN-x` in `PRD.md` has a real verifier in `coverage.json`. `scripts/check-feature-tasks.ps1 -Feature <slug>` validates the `tasks.sql` schema (column list, status literals, forbidden columns). Both gates run inside `scripts/promote-feature.ps1` and must exit 0 before `STATE.md` advances from `defined` to `planned`. This is the machine-enforced incarnation of the rule called out in the `transcript-precision-eval` skill ("must be machine-enforced, not agent-enforced").

### Curated context per task

Each `tasks/<id>/context.md` is the only file the dispatched subagent should load (plus the files it cites). This mirrors afkode's "fresh session per task" model so task 50 runs with the same precision as task 1, without dragging the full PRD into every context window.

### Project-wide testing knowledge

`docs/testing-kb.md` accumulates empirical testing facts across features (cargo timing, fixture regeneration, i18n parity, file-size cap, live-app verification). QC tasks should append discoveries here so feature N+1 does not re-hit feature N's walls.

