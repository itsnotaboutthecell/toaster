# PRD: Unreachable Surface Purge

## Problem & Goals

The Toaster tree still carries dead flags, orphan i18n keys, and
component trees that no sidebar entry can reach. They inflate the
audit surface, break one-to-one scripts/check-translations.ts parity,
and make the "post-processing" and "debug" subsystems silently
non-functional. This PRD closes
`features/product-map-v1/PRD.md` Section 6 item 1.1 and covers
Section 3 items 1, 4, 5, 6, 9, 15.

## Scope

### In scope

- Delete `experimental_simplify_mode` end-to-end.
- Purge `sidebar.general` and `overlay` i18n namespaces across all
  locales.
- Verify and (if needed) finish `check_apple_intelligence_available`
  deletion.
- Restore `sidebar.postProcessing` as a reachable route with a
  "Local LLM only" loopback label.
- Restore `sidebar.debug` behind the `Ctrl+Shift+D` debug-mode gate,
  re-mounting `DebugPaths`, `LogLevelSelector`,
  `WordCorrectionThreshold`, and `LogDirectory`.

### Out of scope (explicit)

- Any change to loopback enforcement, provider logic, or model
  catalog.
- Any new experimental registry (handled by
  `features/ui-experimental-and-cleanup`).
- Audio/export pipeline.
- Net-new i18n keys for pre-existing labels.

## Requirements

### R-001 - Delete `experimental_simplify_mode`

- Description: Remove the flag from `settings/types.rs`, any default
  value file, all consumer branches in
  `src-tauri/src/commands/waveform/mod.rs`, and any serde migration
  paths. Regenerate `src/bindings.ts`.
- Rationale: No UI, no docs, no eval; a silent dead toggle is more
  dangerous than no toggle.
- Acceptance Criteria
  - AC-001-a - `rg -n 'experimental_simplify_mode' src src-tauri`
    returns zero matches (outside vendored or generated build
    artifacts).
  - AC-001-b - `cargo test --manifest-path src-tauri/Cargo.toml
    --workspace` passes.
  - AC-001-c - A settings.json saved before the removal that still
    contains `"experimental_simplify_mode": true` loads without
    error in the live app launched via
    `pwsh scripts/launch-toaster-monitored.ps1`, and no console
    error is emitted.

### R-002 - Purge i18n orphans (`sidebar.general`, `overlay`)

- Description: Remove the `sidebar.general` key and the entire
  `overlay` namespace from every
  `src/i18n/locales/*/translation.json`. Touch only the `sidebar.*`
  subtree when deleting `general`; do not remove `settings.general.*`.
- Rationale: Orphan keys break the one-to-one parity scripts expect
  and pollute grep output during audits.
- Acceptance Criteria
  - AC-002-a - `bun run scripts/check-translations.ts` exits 0.
  - AC-002-b - `rg -n '"sidebar"' src/i18n/locales` shows the
    `sidebar` block contains only the keys that correspond to live
    `SECTIONS_CONFIG` entries (`editor`, `models`, `postProcessing`,
    `advanced`, `about`, `debug`), with zero `general` key under
    `sidebar`.
  - AC-002-c - `rg -n '"overlay"' src/i18n/locales` returns zero
    matches.

### R-003 - Confirm `check_apple_intelligence_available` purge complete

- Description: Verify no Rust command, Tauri invocation, frontend
  caller, or generated binding references the stub. If any residue
  exists (frontend bindings, unused `invoke` wrapper), delete it.
- Rationale: The stub always returned false and was already partly
  removed by the earlier `ui-experimental-and-cleanup` bundle; this
  bundle closes the loop.
- Acceptance Criteria
  - AC-003-a - `rg -n 'check_apple_intelligence_available' src
    src-tauri` returns zero matches.
  - AC-003-b - `cargo test --manifest-path src-tauri/Cargo.toml
    --workspace` passes.
  - AC-003-c - `rg -n 'appleIntelligence|APPLE_INTELLIGENCE' src
    src-tauri` returns zero matches outside vendored deps.

### R-004 - Post-Processing sidebar reachable with loopback label

- Description: `sidebar.postProcessing` remains in
  `src/components/Sidebar.tsx` `SECTIONS_CONFIG` and its panel
  renders a "Local LLM only - endpoints must be loopback
  (127.0.0.1 / localhost / ::1)" label. The UMC `ModelsSettings`
  embed continues to mount unchanged.
- Rationale: Q3 decision; closes
  `features/product-map-v1/PRD.md` Section 3 item 15 and subsumes
  Milestone 2 item 2.7 (`restore-or-delete-post-processing-ui`).
- Acceptance Criteria
  - AC-004-a - In the live app, clicking the Post Process sidebar
    entry renders a visible "Local LLM only" loopback label above
    or adjacent to the provider form.
  - AC-004-b - In the live app, the embedded `ModelsSettings`
    filter/section still renders post-processing models within the
    Post Process panel.
  - AC-004-c - `rg -n "is_local_host" src-tauri/src/llm_client.rs`
    returns at least one match (the enforcement point the UI label
    cites stays intact).

### R-005 - Debug sidebar gated by `settings.debug_mode`

- Description: Add a `SECTIONS_CONFIG.debug` entry in
  `src/components/Sidebar.tsx` whose `enabled()` returns
  `settings.debug_mode === true`. The panel mounts `DebugPaths`,
  `LogLevelSelector`, `WordCorrectionThreshold`, and `LogDirectory`.
  The `Ctrl+Shift+D` handler in `src/App.tsx:67-89` remains the
  sole way to toggle debug mode.
- Rationale: The three stray components are already on disk; gating
  them behind the existing debug toggle preserves production surface
  cleanliness while recovering developer ergonomics.
- Acceptance Criteria
  - AC-005-a - Launch via
    `pwsh scripts/launch-toaster-monitored.ps1`; with debug mode
    off, the Debug sidebar entry is NOT rendered.
  - AC-005-b - Press `Ctrl+Shift+D`, confirm the Debug sidebar
    entry appears, click it, and confirm the panel renders all four
    components (`DebugPaths`, `LogLevelSelector`,
    `WordCorrectionThreshold`, `LogDirectory`).
  - AC-005-c - Press `Ctrl+Shift+D` again; confirm the Debug entry
    disappears and the active section falls back to a default
    (e.g. `editor`) without a runtime error in the devtools
    console.

## Edge cases & constraints

- serde must tolerate older settings.json carrying
  `experimental_simplify_mode`; prefer silent drop over migration
  warning to avoid UX noise.
- Debug mode toggled while the Debug panel is the active section
  must re-route to a valid section (use
  `src/components/Sidebar.tsx` defensive-fallback helper at
  line 59+).
- File-size cap 800 lines per `src/` / `src-tauri/src/` file.
- i18n deletions must be scoped to the `sidebar.*` subtree to avoid
  clobbering `settings.general.*`.

## Data model (if applicable)

- `settings/types.rs`: remove `pub experimental_simplify_mode: bool`.
- Locale JSON: remove `sidebar.general` (if present) and the
  `overlay` top-level namespace (if present) from each
  `src/i18n/locales/*/translation.json`.

## Non-functional requirements

- No hosted network calls.
- `bun run check:file-sizes` exits 0.
- `bun run scripts/check-translations.ts` exits 0.
- `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
  exits 0.
