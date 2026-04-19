# Feature request: remove history and legacy

## 1. Problem & Goals

User-provided one-liner (verbatim):

> Remove the history components from the application, there's no need
> to have this and remove the advanced setting and any legacy code
> configuration as well.

Toaster is a transcript-first video/audio editor forked from Handy
(`AGENTS.md:5`). Several Handy-era surfaces — the saved-transcription
history list, an "Advanced" settings grab-bag, and assorted dictation-
era toggles — no longer serve the editor product. They cost
maintenance, screen space, translator effort (20 locales), DB
migrations, and Tauri command surface area, with zero user value on
the editor path.

Goal: delete the dead surfaces in one coherent cleanup pass and prove
the transcript editor still works end-to-end (open file -> edit ->
undo -> redo -> export).

## 2. Desired Outcome & Acceptance Criteria

- The "History" sidebar section, the SQLite-backed transcription
  history database, the seven `commands::history::*` Tauri commands,
  and the `HistoryUpdatePayload` event are gone from the codebase and
  from the running app.
- The "Advanced" sidebar section is gone; its still-relevant survivors
  are relocated to discoverable, single-purpose homes (no remaining
  "Advanced" grab-bag).
- Transcript edit undo/redo (the in-editor history kept by
  `managers::editor`) is **preserved** and verifiably functional in a
  live `cargo tauri dev` run.
- All locale files in `src/i18n/locales/*/translation.json` lose the
  removed keys; `scripts/check-translations.ts` exits 0.
- `cargo machete`, `knip`, and `depcheck` show no orphaned
  dependencies introduced or revealed by the deletion.

Becomes ACs in PRD R-001..R-006.

## 3. Scope Boundaries

### In scope

- Delete the project-history surface (backend + frontend + i18n + DB
  migration code path + settings fields).
- Delete the "Advanced" settings panel; relocate or delete each child.
- Delete remaining Handy-era settings whose only consumers were the
  removed surfaces (see PRD R-003 for the enumerated list).
- Run dep-hygiene tools and remove orphaned crates / npm packages.
- Remove the corresponding i18n keys from all 20 locale files.
- Verify the editor + undo/redo still works in a live monitored run.

### Out of scope (explicit)

- **Transcript edit history (undo/redo).** `managers::editor` owns the
  per-document undo/redo stack used by every cut/keep operation. This
  is product-critical and must be preserved. The user's "history" word
  refers to the Handy-era saved-transcriptions list, not editor undo.
  See "CRITICAL ambiguity" in journal.md and the disambiguation
  question at the bottom of this file.
- The caption settings UI relocation. The
  `caption-settings-preview` feature owns where caption controls land;
  this feature only ensures they survive the Advanced panel deletion.
- Any change to transcription, alignment, splice, export, or model
  manager logic.
- Adding new features. This is a cleanup-only feature; net file count
  must go down.

## 4. References to Existing Code

Backend:

- `src-tauri/src/managers/history.rs` — 24 KB SQLite-backed
  transcription-history manager (DB migrations, paginated list, retry,
  delete, retention sweep). Entire file deleted.
- `src-tauri/src/managers/history_tests.rs` — companion tests. Deleted.
- `src-tauri/src/commands/history.rs` — 7 Tauri commands. Deleted.
- `src-tauri/src/managers/mod.rs:7` — `pub mod history;` removed.
- `src-tauri/src/commands/mod.rs:7` — `pub mod history;` removed.
- `src-tauri/src/lib.rs:19,135-144,299-307` — manager construction,
  `manage(history_manager)`, command registrations, event collection.
- `src-tauri/src/settings/types.rs:243-244`,
  `src-tauri/src/settings/defaults.rs:98,411`,
  `src-tauri/src/settings/io.rs:55-57`,
  `src-tauri/src/settings/mod.rs:26` — `history_limit` and
  `recording_retention_period` settings fields and helpers.
- `src-tauri/src/managers/editor/mod.rs:16,36` — **PRESERVED** comment
  references confirming `EditorManager` owns the in-editor undo/redo
  stack. Untouched.
- `src-tauri/src/managers/cleanup/mod.rs:7` — comment refs
  history-retry consumer; comment updated, code path validated.
- `src-tauri/src/managers/transcription_mock.rs:5` — comment ref to
  `commands::history`; comment updated.
- `src-tauri/src/managers/transcription/adapter.rs:109` — comment ref
  updated.

Frontend:

- `src/components/Sidebar.tsx:3,5-10,26-57` — `History`, `Cog` icons;
  `HistorySettings`, `AdvancedSettings` imports; `history` and
  `advanced` `SECTIONS_CONFIG` entries.
- `src/components/settings/index.ts:2-3` — `AdvancedSettings`,
  `HistorySettings` re-exports.
- `src/components/settings/history/HistorySettings.tsx` — entire
  directory deleted.
- `src/components/settings/advanced/AdvancedSettings.tsx` — entire
  directory deleted.
- `src/components/settings/HistoryLimit.tsx` — deleted.
- `src/components/settings/RecordingRetentionPeriod.tsx` — deleted.
- `src/components/settings/ExperimentalToggle.tsx`,
  `ExperimentalSimplifyModeToggle.tsx`,
  `AccelerationSelector.tsx`,
  `ModelUnloadTimeout.tsx`,
  `LazyStreamClose.tsx`,
  `TranslateToEnglish.tsx`,
  `OutputDeviceSelector.tsx`,
  `PostProcessingToggle.tsx`,
  `UpdateChecksToggle.tsx` — Advanced-only consumers; status
  reviewed in PRD R-002 / R-003.
- `src/stores/settingsStore.ts:82,94` — `recording_retention_period`
  and `history_limit` mutator entries removed.
- `src/bindings.ts` — regenerated by `cargo tauri dev` after backend
  deletions; not hand-edited.

i18n:

- `src/i18n/locales/en/translation.json:11,13,165,172,334,372` — keys
  `sidebar.advanced`, `sidebar.history`, `settings.advanced.*`,
  `settings.history.*`, `settings.recordingRetention.*`. Mirror in 19
  other locales. Verifier: `scripts/check-translations.ts`.

Skills (consulted, per AGENTS.md):

- `.github/skills/handy-legacy-pruning/SKILL.md` — verified the
  classic Handy file list (`actions.rs`, `shortcut/`, `overlay.rs`,
  `tray*.rs`, `clipboard.rs`, `input.rs`, `audio_feedback.rs`,
  `apple_intelligence.rs`, `audio_toolkit/audio/recorder.rs`,
  `audio_toolkit/vad/`, `PushToTalk.tsx`, `AudioFeedback.tsx`,
  `AccessibilityPermissions.tsx`, `HandyKeysShortcutInput.tsx`)
  **does not exist in the tree** (grep confirms). Already pruned.
  This feature handles the residue.
- `.github/skills/dep-hygiene/SKILL.md` — gates `cargo machete`,
  `knip`, `depcheck`.
- `.github/skills/i18n-pruning/SKILL.md` — gates
  `scripts/check-translations.ts`.

## 5. Edge Cases & Constraints

- **Database file (`history.db`).** Existing user installs have a
  populated SQLite file at `app_data_dir/history.db`. Deleting the
  manager leaves the file orphaned but harmless. PRD R-001 keeps it
  on-disk (no destructive delete from a running app); a follow-up
  janitor task may sweep it later.
- **Settings JSON forward-compat.** Removed fields (`history_limit`,
  `recording_retention_period`) must be tolerated as unknown keys by
  serde on older user-saved settings files. `serde(default = ...)` +
  unknown-field tolerance must be confirmed (see BLUEPRINT.md).
- **Transcript edit undo/redo must keep working.** `managers::editor`
  is untouched; live QC must press undo/redo and observe the expected
  state transitions.
- **i18n key removal must be exhaustive.** `check-translations.ts`
  fails the build if a locale has a key that en lacks (or vice versa).
  Every removal must land in all 20 locales in the same commit.
- **Dependency removal must not regress builds.** `cargo machete` and
  `knip` are advisory; manual review before deleting any crate or npm
  package.

## 6. Data Model

No new data. Removed fields:

| Path | Field | Type | Removal action |
|------|-------|------|----------------|
| `src-tauri/src/settings/types.rs:243` | `history_limit` | `usize` | drop field + serde default |
| `src-tauri/src/settings/types.rs` (recording retention) | `recording_retention_period` | `Option<...>` | drop field + serde default |
| `app_data_dir/history.db` | (file) | SQLite | leave on disk for existing installs; not created on fresh installs |

## Q&A

(Phase 5 was deferred per the user's instruction — STATE.md remains
`defined`. Two disambiguation questions are surfaced to the user in
the PM hand-off message and journal.md so they can be answered before
any execution begins. They are listed here verbatim:)

- **Q1.** When you said "history components", you meant the saved-
  transcription list (the Handy-era History sidebar, SQLite database,
  retry + retention features) — **not** the in-editor undo/redo stack
  that powers every cut/keep on the transcript. Is that correct?
  Default applied: yes (project history removed; editor undo/redo
  preserved).
- **Q2.** When you said "any legacy code configuration as well", does
  that include the `Experimental` feature flag and its gated controls
  (`ExperimentalSimplifyModeToggle`, `AccelerationSelector`)? They
  exist only inside the Advanced panel today and have no other host.
  Default applied: yes — delete them with the Advanced panel.
