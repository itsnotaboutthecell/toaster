# Feature request: Unreachable Surface Purge

## 1. Problem & Goals

The Handy-fork -> Toaster rebrand, `remove-history-and-legacy` purge, and
the Unified Model Catalog (UMC) shipments left several unreachable UI
surfaces, orphan i18n keys, and a dead Rust settings flag in the tree.
They show up in `rg` results, translation diffs, and settings
serialization, but have no user-visible path and no automated coverage.
This feature finishes the cleanup called out in
`features/product-map-v1/PRD.md` Section 6 item 1.1 and in
`features/product-map-v1/PRD.md` Section 3 items 1, 4, 5, 6, 9, 15.

Goals:

- Remove the dead `experimental_simplify_mode` settings flag end-to-end
  (Rust types, defaults, consumers, migrations).
- Delete i18n orphans (`sidebar.general`, `overlay` namespace) from
  every `src/i18n/locales/*/translation.json` so
  `scripts/check-translations.ts` stays green.
- Confirm the `check_apple_intelligence_available` stub and its
  frontend caller are fully gone (the earlier
  `ui-experimental-and-cleanup` bundle was supposed to land this;
  verify and mop up any residue).
- Restore `sidebar.postProcessing` as a reachable sidebar route using
  the existing `PostProcessingSettings` component tree and the
  UMC-landed `ModelsSettings` embed, with a clearly rendered
  "Local LLM only" label.
- Restore `sidebar.debug` behind the existing `Ctrl+Shift+D` debug
  toggle (`src/App.tsx:67-89`) and re-mount `DebugPaths`,
  `LogLevelSelector`, `WordCorrectionThreshold`, and the already-moved
  `LogDirectory` inside it.

## 2. Desired Outcome & Acceptance Criteria

A code audit (`rg`, `bun run check:file-sizes`,
`bun run scripts/check-translations.ts`, `cargo test`) and a live-app
run (`pwsh scripts/launch-toaster-monitored.ps1`) produce zero
references to the purged flags/keys, and every restored sidebar entry
actually navigates to its panel. Exact testable statements appear as
`AC-NNN-x` entries in `PRD.md`.

## 3. Scope Boundaries

### In scope

- Chunk A: Delete `experimental_simplify_mode` from
  `src-tauri/src/settings/types.rs:259`,
  `src-tauri/src/settings/defaults.rs` (if referenced), and all five
  consumer sites in `src-tauri/src/commands/waveform/mod.rs:299, 331,
  334, 343, 352, 357, 363, 366`. Remove the flag from serde migration
  code and the generated `src/bindings.ts`.
- Chunk B: Purge `sidebar.general` and the entire `overlay` namespace
  from every `src/i18n/locales/*/translation.json` (20 files on disk).
  Confirm `scripts/check-translations.ts` is green.
- Chunk C: Verify `check_apple_intelligence_available` is gone from
  both `src-tauri/` and `src/`. If residue exists, delete it. Update
  `src/bindings.ts` if regeneration is needed.
- Chunk D: Verify `sidebar.postProcessing` is reachable from
  `src/components/Sidebar.tsx` `SECTIONS_CONFIG` (currently listed at
  line 39-44). Ensure the panel renders a "Local LLM only - endpoints
  must be loopback (127.0.0.1 / localhost / ::1)" label, citing
  `src-tauri/src/llm_client.rs` `is_local_host`. Confirm the UMC
  `ModelsSettings` embed still mounts (landed by
  `umc-post-process-embed`).
- Chunk E: Re-mount `sidebar.debug` gated by `settings.debug_mode`
  toggled via `Ctrl+Shift+D` (`src/App.tsx:67-89`). Panel shows
  `DebugPaths`, `LogLevelSelector`, `WordCorrectionThreshold`, and
  the existing `LogDirectory`.

### Out of scope (explicit)

- New experimental flag UI or registry (done in
  `features/ui-experimental-and-cleanup`; this bundle explicitly
  supersedes that feature's simplify-mode path by deleting the flag
  instead of surfacing it).
- Any change to post-processing provider logic, loopback enforcement,
  or model catalog entries.
- Any net-new i18n keys (panels being re-mounted already have keys).
- Audio/export pipeline changes.

## 4. References to Existing Code

- `src-tauri/src/settings/types.rs:259` - flag definition.
- `src-tauri/src/commands/waveform/mod.rs:299-366` - consumer sites.
- `src/components/Sidebar.tsx:26-57` - `SECTIONS_CONFIG`.
- `src/components/settings/debug/` - DebugPaths, LogDirectory,
  LogLevelSelector, WordCorrectionThreshold components.
- `src/components/settings/post-processing/PostProcessingSettings.tsx`
  and `src/components/settings/post-processing/PostProcessingSettingsPrompts.tsx`.
- `src/App.tsx:67-89` - existing `Ctrl+Shift+D` debug toggle.
- `src-tauri/src/llm_client.rs` - loopback `is_local_host` policy.
- `features/product-map-v1/PRD.md` Section 3 (items 1, 4, 5, 6, 9, 15)
  and Section 6 item 1.1 - originating mandate.
- `features/ui-experimental-and-cleanup/` - earlier partial cleanup
  (this bundle supersedes R-001 and confirms R-003 completion there).

## 5. Edge Cases & Constraints

- No hosted network calls (AGENTS.md local-only boundary).
- All i18n deletions touch every `src/i18n/locales/*/translation.json`.
- File-size cap 800 lines per `src/` / `src-tauri/src/` file
  (`bun run check:file-sizes`).
- Re-mounts must use existing i18n keys; no net-new keys for
  already-existing labels.
- Post-processing panel must not fetch from non-loopback hosts
  (enforced by `llm_client.rs` `is_local_host`).
- Debug panel must only be reachable while `settings.debug_mode` is
  `true` (set via `Ctrl+Shift+D`).
- `experimental_simplify_mode` has never been surfaced in settings
  UI and has no user-facing documentation; deletion is a pure dead-code
  removal with no migration for users.

## 6. Data Model (optional)

- `settings/types.rs` loses one `bool` field
  (`experimental_simplify_mode`).
- serde deserialization must tolerate older settings.json that still
  contains the field (use `#[serde(default)]` drop-through or a
  migration step that silently ignores unknown fields if
  `deny_unknown_fields` is not set).

## Q&A

(Q1, Q3, Debug-panel, and sidebar.general decisions were provided in
the user mandate; see Blueprint "Migration / compatibility" for
verbatim decisions.)

- Q1 [answered]: `experimental_simplify_mode` -> delete outright. No
  UI surface, no eval coverage; wire off permanently and remove the
  flag plus all five consumer branches in
  `commands/waveform/mod.rs`. This supersedes the earlier
  `ui-experimental-and-cleanup` plan to surface it under an
  Experimental panel.
- Q3 [answered]: Post-processing UI -> restore. Keep it reachable
  from `Sidebar.tsx` `SECTIONS_CONFIG` and add a "Local LLM only -
  endpoints must be loopback" label. Consumes the UMC `ModelsSettings`
  embed already landed by `umc-post-process-embed`.
- Debug panel [answered]: surface behind the existing `Ctrl+Shift+D`
  debug toggle, re-mounting `sidebar.debug` only when
  `settings.debug_mode` is true. Matches the `LogDirectory`
  relocation pattern.
- `sidebar.general` [answered]: pure orphan -> delete from every
  locale. Recon note: the `"general":` block that appears in all 20
  locale files is `settings.general` (a child of `settings`), not
  `sidebar.general`. Chunk B must scope deletion to the `sidebar.*`
  subtree only.
