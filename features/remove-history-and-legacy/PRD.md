# PRD: remove history and legacy

## Problem & Goals

Toaster ships several Handy-era surfaces (transcription history list,
"Advanced" settings grab-bag, several dictation-era toggles) that have
no role in the transcript-first editor product. They cost
maintenance, screen real estate, translator effort across 20 locales,
DB migrations, and Tauri command surface. This feature deletes them
in one coherent pass and proves the editor still works end-to-end.

## Scope

### In scope

- Delete the project transcription-history surface (backend + frontend
  + i18n + Tauri commands + settings fields).
- Delete the "Advanced" sidebar panel; relocate or delete each child.
- Delete dictation-era settings whose only host was the Advanced panel
  or the History panel.
- Drop orphaned Cargo crates and npm packages revealed by the
  deletion.
- Remove the affected i18n keys from every locale.
- Update `AGENTS.md` repository-layout block (one line) to drop the
  `managers/history/` reference.
- Live QC of the editor confirms transcript undo/redo still works.

### Out of scope (explicit)

- Transcript edit undo/redo (`managers::editor` undo stack). PRESERVED.
- Caption-settings UI relocation (owned by `caption-settings-preview`).
- Any logic change to transcription, splice, export, or model managers.
- Deleting `app_data_dir/history.db` from existing installs (orphaned
  but harmless; left for a future janitor pass).

## Requirements

### R-001 — Remove the project transcription-history surface

- Description: Delete the Handy-era saved-transcription list end to
  end. Backend manager + tests + Tauri commands + event payload + the
  settings fields that only powered it (`history_limit`,
  `recording_retention_period`). Frontend sidebar entry, panel
  components, settings store mutators.
- Rationale: The transcript-first editor never reads this surface.
  Keeping it costs DB migrations, ~40 KB of Rust, ~410 lines of React,
  and 7 Tauri commands.
- Files to delete (full): `src-tauri/src/managers/history.rs`,
  `src-tauri/src/managers/history_tests.rs`,
  `src-tauri/src/commands/history.rs`,
  `src/components/settings/history/HistorySettings.tsx`,
  `src/components/settings/HistoryLimit.tsx`,
  `src/components/settings/RecordingRetentionPeriod.tsx`.
- Files to edit: `src-tauri/src/managers/mod.rs:7`,
  `src-tauri/src/commands/mod.rs:7`,
  `src-tauri/src/lib.rs:19,135-144,299-307`,
  `src-tauri/src/settings/types.rs:243-244`,
  `src-tauri/src/settings/defaults.rs:98,411`,
  `src-tauri/src/settings/io.rs:55-57`,
  `src-tauri/src/settings/mod.rs:26`,
  `src-tauri/src/managers/cleanup/mod.rs:7` (comment),
  `src-tauri/src/managers/transcription_mock.rs:5` (comment),
  `src-tauri/src/managers/transcription/adapter.rs:109` (comment),
  `src/components/Sidebar.tsx:3,5-10,26-57`,
  `src/components/settings/index.ts:3`,
  `src/stores/settingsStore.ts:82,94`,
  `AGENTS.md` (repo-layout block: drop `managers/history/`).
- **Preserved (do not touch):** `src-tauri/src/managers/editor/mod.rs`
  undo/redo logic — this is the in-editor history that powers every
  cut/keep. Confirmed via grep at `editor/mod.rs:16,36`.
- Acceptance Criteria
  - AC-001-a — `cd src-tauri && cargo check` exits 0 after the
    deletion (no dangling `pub mod history;`, no missing
    `HistoryManager` references).
  - AC-001-b — `rg --files-with-matches "HistoryManager|HistoryEntry|HistoryUpdatePayload|transcription_history|history_limit|recording_retention_period" src-tauri/src src` returns no results outside this feature directory.
  - AC-001-c — `npm run build` exits 0; `bindings.ts` is regenerated
    and contains none of the seven `history*` command names.

### R-002 — Remove the "Advanced" settings panel

- Description: Delete the `Advanced` sidebar entry and the
  `AdvancedSettings` panel. Each child either relocates to a
  single-purpose home or is removed (R-003). The Advanced panel is a
  grab-bag of unrelated controls; with the History panel gone the
  remaining children fit better elsewhere.
- Disposition per child of `AdvancedSettings.tsx`:
  - `ModelUnloadTimeoutSetting` -> moved into `ModelsSettings` panel.
  - `ExperimentalToggle` -> deleted (see R-003; default per Q2).
  - `DiscardWords`, `AllowWords` -> moved into `EditorView` settings
    surface (single-purpose "Words" group). The exact placement
    follows whatever pattern `EditorView` already exposes; Blueprint
    cites the file:line.
  - `CaptionSettings` -> placement deferred to
    `caption-settings-preview` feature; this feature ensures the
    component still mounts somewhere reachable (temporary host:
    `ModelsSettings` "Captions" group, replaced by the parallel
    feature on land).
  - `ExperimentalSimplifyModeToggle`, `AccelerationSelector` ->
    deleted (R-003, default per Q2).
  - `HistoryLimit` -> deleted (R-001).
- Files to delete: `src/components/settings/advanced/AdvancedSettings.tsx`
  (and the empty `advanced/` directory).
- Files to edit: `src/components/Sidebar.tsx` (remove `advanced`
  SECTIONS_CONFIG entry + `Cog` import), `src/components/settings/index.ts`
  (drop the `AdvancedSettings` re-export), the new host panels for
  each relocated child.
- Acceptance Criteria
  - AC-002-a — Sidebar no longer renders an "Advanced" entry; verified
    by Playwright assertion or manual screenshot in live QC.
  - AC-002-b — `rg "AdvancedSettings|sidebar\.advanced|settings\.advanced" src` returns no results outside this feature directory.
  - AC-002-c — Each preserved child (`ModelUnloadTimeoutSetting`,
    `DiscardWords`, `AllowWords`, `CaptionSettings`) is reachable from
    a non-Advanced panel in a live `cargo tauri dev` run.

### R-003 — Remove dictation-era controls whose only host was Advanced or History

- Description: Files exist whose sole consumer was the Advanced or
  History panel. With those panels gone, these become dead code.
  Delete each, with single-grep verification of zero remaining
  consumers.
- Per-file decision (each must be re-grepped at execution time; if a
  non-Advanced consumer is found, the file is preserved and the
  finding logged in `journal.md`):
  - `src/components/settings/ExperimentalToggle.tsx` -> delete.
  - `src/components/settings/ExperimentalSimplifyModeToggle.tsx` -> delete.
  - `src/components/settings/AccelerationSelector.tsx` -> delete only
    if grep confirms no non-Advanced consumer.
  - `src/components/settings/HistoryLimit.tsx` -> delete (covered by
    R-001).
  - `src/components/settings/RecordingRetentionPeriod.tsx` -> delete
    (covered by R-001).
- Settings struct fields whose only readers were the deleted controls
  must also drop (`experimental_enabled`, `acceleration_*`,
  `simplify_mode_*`, etc. — exact list compiled at execution by
  grepping `src-tauri/src/settings/types.rs` against the surviving
  consumers).
- The classic Handy file list from `handy-legacy-pruning/SKILL.md`
  (`actions.rs`, `shortcut/`, `overlay.rs`, `tray*.rs`, `clipboard.rs`,
  `input.rs`, `audio_feedback.rs`, `apple_intelligence.rs`,
  `audio_toolkit/audio/recorder.rs`, `audio_toolkit/vad/`,
  `PushToTalk.tsx`, `AudioFeedback.tsx`, `AccessibilityPermissions.tsx`,
  `HandyKeysShortcutInput.tsx`) was re-grepped during PRD authoring
  and **none of them exist** in the current tree. No deletion action
  needed; finding logged in `journal.md`.
- Acceptance Criteria
  - AC-003-a — `repo-auditor` agent run reports zero new dead-module
    warnings after the deletion (baseline captured before deletion in
    `journal.md`).
  - AC-003-b — `rg "ExperimentalToggle|ExperimentalSimplifyMode|experimental_enabled" src src-tauri/src` returns no results.

### R-004 — Drop orphaned Cargo crates and npm packages

- Description: Run dep-hygiene tools after the deletion and remove
  any crate or package that is no longer referenced.
- Acceptance Criteria
  - AC-004-a — `cargo machete` (run from `src-tauri`) reports zero
    unused dependencies, **or** any reported crates are removed from
    `src-tauri/Cargo.toml` and the tool is re-run clean.
  - AC-004-b — `npx knip` and `npx depcheck` (or whichever the repo
    uses; see `dep-hygiene` skill) report zero unused dependencies,
    **or** they are removed from `package.json` and the tool re-runs
    clean.

### R-005 — Drop orphaned i18n keys across all locales

- Description: Every removed user-visible string drops its key from
  every locale file in `src/i18n/locales/*/translation.json`. Keys to
  remove (compiled by re-grepping `en/translation.json` for the
  removed components):
  - `sidebar.history`, `sidebar.advanced`
  - `settings.advanced.*` (entire subtree)
  - `settings.history.*` (entire subtree)
  - `settings.recordingRetention.*`
  - `settings.modelSettings.advanced` if present
  - any sub-key referenced only by deleted components
- Acceptance Criteria
  - AC-005-a — `bun run scripts/check-translations.ts` (or
    `npx tsx scripts/check-translations.ts`) exits 0.
  - AC-005-b — `rg "settings\.advanced|settings\.history|sidebar\.advanced|sidebar\.history|recordingRetention" src` returns no results outside the deleted files.

### R-006 — Live verification: editor + undo + redo still work

- Description: A monitored `cargo tauri dev` launch must reach the
  Vite-ready signal, open the editor on a fixture media file, perform
  one cut, press Undo, press Redo, and observe the expected state
  transitions. This is the non-negotiable "editor still works" gate.
- Acceptance Criteria
  - AC-006-a — `scripts/launch-toaster-monitored.ps1
    -ObservationSeconds 300` reports "Vite ready" and no Rust panics
    in the first 60 seconds of stdout.
  - AC-006-b — Manual step (recorded in `journal.md` with timestamp +
    operator initials): open `eval/fixtures/<picked-fixture>.mp4`,
    delete the third word in the transcript, press Cmd/Ctrl+Z, observe
    the word reappear; press Cmd/Ctrl+Y (or Cmd+Shift+Z), observe the
    word disappear again.
  - AC-006-c — `pwsh scripts/eval/eval-edit-quality.ps1` exits 0
    (regression gate that the editor's keep-segment + time-mapping
    logic still satisfies the precision invariants).
  - AC-006-d — `pwsh scripts/eval/eval-audio-boundary.ps1` exits 0
    (regression gate that splice boundaries are unaffected).

## Edge cases & constraints

- Existing users have a populated `history.db` SQLite file. Leave it
  on disk; do not migrate or delete from a running app.
- Settings JSON forward-compat: serde must tolerate unknown keys for
  the dropped settings fields (default-on-missing semantics). The
  Blueprint cites the existing serde pattern.
- AGENTS.md repo-layout block currently mentions `managers/history/`
  and `managers/project/` (line 71). Drop the `history/` reference and
  re-verify the line still matches reality (no other inaccurate
  pointers introduced).
- The `caption-settings-preview` feature is being planned in parallel.
  This feature's CaptionSettings relocation must be a temporary host
  to avoid blocking the parallel feature; the Blueprint specifies the
  contract.

## Data model

No additions. Removals tabulated in REQUEST.md section 6.

## Non-functional requirements

- Net file count strictly decreases. PR description should report the
  delta (files added: 0; files deleted: >= 6).
- No file in the diff exceeds 800 lines (AGENTS.md cap).
- All planning artifacts and code comments are ASCII; no smart quotes.
- Single source of truth: nothing about the deletion is duplicated
  into `.github/copilot-instructions.md`;
  AGENTS.md is updated and the others remain pointer files.
