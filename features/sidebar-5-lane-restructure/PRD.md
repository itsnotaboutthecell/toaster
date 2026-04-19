# PRD: sidebar-5-lane-restructure

## R-001 — Sidebar is exactly 5 items

`SECTIONS_CONFIG` in `src/components/Sidebar.tsx` lists only: editor, models, postProcessing, advanced, about. In that order.

- AC-001-a — Live app: sidebar renders exactly 5 clickable items in the stated order.
- AC-001-b — Lint + tsc green after `export` and `experimental` keys are removed from SECTIONS_CONFIG.

## R-002 — Advanced renders 5 groups

Advanced contains: Words, Performance, Captions, Export, Experimental. In that order.

- AC-002-a — Live app: navigating to Advanced shows 5 SettingsGroup headers in order.
- AC-002-b — Words group still contains DiscardWords + AllowWords (no regression from prior feature).

## R-003 — Models page is pure catalog

Performance (ModelUnloadTimeoutSetting) and Captions (CaptionSettings) groups are removed from `ModelsSettings.tsx`.

- AC-003-a — `rg -n "ModelUnloadTimeoutSetting\|CaptionSettings" src/components/settings/models/` returns zero matches.
- AC-003-b — Live app: Models page shows only the catalog UI (language filter + model cards); no Performance or Captions sections.

## R-004 — Export relocates unchanged

Export content moves into an Advanced group. No behavior change.

- AC-004-a — Live app: Advanced > Export group exposes every control that used to be on the retired Export sidebar page.
- AC-004-b — `rg -n "SECTIONS_CONFIG.*export" src/components/Sidebar.tsx` returns zero matches.

## R-005 — Experimental is gated by a master toggle

New `experimental_enabled: bool` (default `false`) in AppSettings. When OFF, only the master toggle renders; the per-flag list is hidden. When ON, existing per-flag ToggleSwitch list is revealed.

- AC-005-a — Backend: `cargo check -p toaster --lib` green after `experimental_enabled` added.
- AC-005-b — Live app: flipping master OFF → ON reveals the per-flag list; flipping ON → OFF hides it.
- AC-005-c — Default on fresh install: master toggle is OFF, per-flag list hidden.

## R-006 — Defence-in-depth getter

When `experimental_enabled == false`, any code reading an individual experiment boolean must see `false`, regardless of the stored per-flag value.

- AC-006-a — cargo test `experiment_getter_returns_false_when_master_disabled` asserts: AppSettings with master=false and a per-flag stored=true → getter returns false.
- AC-006-b — cargo test `experiment_getter_returns_stored_value_when_master_enabled` asserts: master=true and per-flag=true → getter returns true.
- AC-006-c — Frontend wrapper (or Rust-side getter consumed via bindings) applies the same rule; hook unit test or live-app toggle sequence confirms a visible experimental feature turns off when master is flipped off without manually clearing the per-flag toggle.

## R-007 — i18n stays in sync

Retired keys removed in all 20 locale JSONs; new keys added in all 20.

- AC-007-a — `bun run scripts/check-translations.ts` exits 0.
- AC-007-b — `rg "sidebar\.export\|sidebar\.experimental" src/i18n/locales/` returns zero matches.
- AC-007-c — Every locale includes the `settings.advanced.groups.{words,performance,captions,export,experimental}.{title,description}` and `settings.advanced.experimentalMaster.{title,description}` keys.

## R-008 — Static gates green

- AC-008-a — `npm run lint` exits 0.
- AC-008-b — `npx tsc --noEmit` exits 0.
- AC-008-c — `cargo check -p toaster --lib` exits 0.

## R-009 — Live-app QC

- AC-009-a — Launch monitored; navigate Editor, Models, Post-Process, Advanced, About; no 404 or runtime error in log.
- AC-009-b — In Advanced, scroll through all 5 groups and confirm each section's add/remove/edit controls respond.
- AC-009-c — Flip `experimental_enabled` OFF → ON → OFF; confirm the per-flag list appears and disappears without requiring a reload.
