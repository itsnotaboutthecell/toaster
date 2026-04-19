# PRD: Advanced menu restoration

## Problem & Goals

Reverse the inline relocation of Allow / Discard word lists into the editor (`src/components/editor/EditorView.tsx:574-578`); restore an Advanced surface in Settings; produce a documented audit of which other "configured once" settings could live there. See `REQUEST.md` for the full rationale.

## Scope

### In scope

- Editor: remove the inline `<DiscardWords>` + `<AllowWords>` block.
- Settings: add an Advanced surface containing those two components.
- i18n: new keys propagated to every locale.
- Audit doc capturing the frequency-of-use heuristic and a recommendation table.

### Out of scope (explicit)

- Underlying setting-key changes.
- New word-list features.
- Forced relocation of any other setting (audit recommends only).

## Requirements

### R-001 - Remove inline word-list block from editor

- Description: delete the `SettingsGroup` titled `editor.sections.words` and its `<DiscardWords>` / `<AllowWords>` children from `EditorView.tsx`.
- Rationale: these are set-once controls and do not belong on the per-clip surface.
- Acceptance Criteria
  - AC-001-a - `EditorView.tsx` contains no `<DiscardWords>` or `<AllowWords>` import or JSX usage after this change (grep returns zero matches in `src/components/editor/`).
  - AC-001-b - Live launch (`scripts/launch-toaster-monitored.ps1`): the editor view renders without the words section, no console errors related to missing imports.

### R-002 - Add Advanced surface in Settings

- Description: introduce an Advanced panel in Settings, structured the same way as `ExperimentalSettings.tsx`, that hosts `<DiscardWords>` and `<AllowWords>` inside a single `<SettingsGroup>`.
- Rationale: provides a stable home for set-once configuration without polluting the editor.
- Acceptance Criteria
  - AC-002-a - Live launch: a new "Advanced" entry is reachable from the Settings navigation; clicking it renders both word-list components with full add / remove / conflict-detection behaviour.
  - AC-002-b - Settings keys (`custom_words`, `custom_filler_words`) read / write through unchanged: adding a word in the new location persists across an app restart.
  - AC-002-c - The Advanced surface uses the existing `<SettingsGroup>` / `<SettingContainer>` shells so its visual style matches Post-processing and Experimental.

### R-003 - i18n parity

- Description: add `settings.advanced.title` and `settings.advanced.description` (plus any nav-label key) to all 22 locale files in `src/i18n/locales/`.
- Rationale: Toaster ships translated; missing keys break `scripts/check-translations.ts` and surface as raw key strings to non-English users.
- Acceptance Criteria
  - AC-003-a - `bun run scripts/check-translations.ts` exits 0 after the change.
  - AC-003-b - English source key has a real human-readable string ("Advanced", plus a one-line description per `AGENTS.md` Settings UI contract). Other locales may use the English string as a placeholder if no translation is available, provided the script still exits 0.

### R-004 - Document the audit and heuristic

- Description: append to `features/advanced-menu-restoration/journal.md` (a) the frequency-of-use heuristic in one paragraph and (b) a table of every current Settings page surfacing each control's recommended placement (Main / Advanced / Experimental). Also add a new `docs/settings-placement.md` so the rule survives this PR.
- Rationale: the user asked us to identify other candidates; without a written rule the next contributor will repeat the regression.
- Acceptance Criteria
  - AC-004-a - `features/advanced-menu-restoration/journal.md` contains an "Audit" section with the rule and the table; the table covers every setting currently rendered in `src/components/settings/` (excluding `experimental/` and `debug/`).
  - AC-004-b - `docs/settings-placement.md` exists, states the heuristic in user-readable form, and links to the audit table.

## Edge cases & constraints

- Conflict detection between Allow and Discard lists must keep working in the new location.
- ASCII only in checked-in docs.
- No new persisted setting in this PR; "remember collapsed state across sessions" is deferred.
- Must not regress the `i18n-pruning` skill expectations.

## Data model

No new fields. Existing `custom_words: string[]` and `custom_filler_words: string[]` unchanged.

## Non-functional requirements

- No file may exceed 800 lines (AGENTS.md file-size cap). The new Advanced page should be small (~50 lines based on `ExperimentalSettings.tsx`).
