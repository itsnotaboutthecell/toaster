# REQUEST: settings-ui-consistency-fix

## 1. Problem & Goals

The `settings-ui-consistency-audit` baseline report (2026-04-18) machine-
confirmed a large cluster of UI inconsistencies the user reported during live
QC of the current Advanced / Models / Export / Post-process / Captions surface.
User-verbatim complaints, with the audit rule each maps to:

- "The live preview area is way too big and on vertical it spills over to the
  other settings." → `R-004-desktop-width` (1 critical).
- "Export area it doesn't follow the same pattern of Left = setting, Right =
  configuration (drop downs, etc.)." → `R-003-export-two-column` (4) +
  `R-003-layout` (10) on Export.
- "The other menus seem to be cutting right up against borders — models as one
  example, advanced the entire captions area is really bad." → Export +
  Advanced page padding, same `R-003-layout` cluster on Advanced.
- (Pre-existing AGENTS.md rules not yet surfaced to the user as bugs but
  flagged by the audit:)
  - `R-005-range-editable` (28) — sliders with no typed-entry sibling.
  - `R-005-color-light-grey-on-white` (4) — low-contrast labels/descriptions.
  - `R-005-missing-description` (84) — setting rows with no one-line
    description.

Goal: resolve the user-reported clusters in full (critical + HIGH severity)
and burn down the AGENTS.md "Settings UI contract" violations to zero, with
the missing-description batch structured so it can split into a follow-up if
it grows out of scope.

## 2. Outcome & Acceptance Criteria

At the end of this feature:

- Re-running `scripts/migrate/audit-settings-ui.ps1` produces `critical == 0`.
- Targeted rule counts drop to 0 for: `R-004-desktop-width`,
  `R-003-export-two-column`, `R-003-layout`, `R-005-range-editable`,
  `R-005-color-light-grey-on-white`.
- `R-005-missing-description` count drops by at least the rows whose copy is
  already in `src/i18n/locales/*/translation.json`, or is formally descoped
  to a follow-up bundle if triage reveals missing product copy.
- Captions preview is rebuilt as a horizontal/vertical-orientation designer
  per the user's fixture reference, single-source-of-truth for preview↔export
  caption sizing.
- No new red-on-dark or light-grey-on-white text.
- i18n parity (`scripts/check-translations.ts`) remains green.

Full PRD AC list: `PRD.md`.

## 3. Scope

### In scope

- Layout fixes across `src/components/settings/**` pages and
  `src/components/ui/SettingContainer.tsx`.
- Caption preview pane re-implementation under
  `src/components/settings/captions/` (horizontal/vertical designer).
- `src/components/settings/export/**` row restructure to Left=label,
  Right=control.
- Slider keyboard-entry sibling input (per AGENTS.md "Settings UI contract").
- Color token cleanup on affected rows.
- i18n additions for any new labels/descriptions; no string left hardcoded.
- Re-run of `scripts/migrate/audit-settings-ui.ps1` as the final gate.

### Out of scope

- New settings, new feature toggles, new sidebar entries.
- Any backend (`src-tauri/**`) logic change, except where single-source-of-
  truth caption sizing forces a shared helper to move from frontend to
  backend per AGENTS.md "dual-path logic".
- Hosted inference / network features (hard No per AGENTS.md "Local-only
  inference").
- Full descriptions authoring for all 84 missing-description rows if
  stakeholder copy is not ready — descope to follow-up bundle instead.

## 4. Code references

- `features/settings-ui-consistency-audit/audit-report/audit.md` — authoritative
  violation list with per-row selector, rule, and screenshot.
- `features/settings-ui-consistency-audit/audit-report/audit.json` — machine-
  readable copy of the same; ordered.
- `tests/settingsUIAudit.spec.ts` — rule implementation; re-read before
  changing any rule threshold.
- `scripts/migrate/audit-settings-ui.ps1` — audit wrapper, used as a verifier here.
- `src/components/ui/SettingContainer.tsx` — 4 return paths, already
  data-testid decorated.
- `src/components/settings/{about,advanced,models,post-processing,export}/*`
  — target surface.
- `src/components/settings/captions/CaptionProfileShared.tsx` (preview pane).
- `docs/settings-placement.md` — `## Layout invariants` section is the
  normative contract.
- `AGENTS.md` → "Settings UI contract" and "Single source of truth for
  dual-path logic".

## 5. Edge cases

- Mobile viewport (390x844) must not regress; audit already covers this.
- Caption preview must use the identical sizing policy as export (dual-path
  rule). If the backend currently owns sizing, the frontend consumes it; if
  the frontend had an independent copy, it is deleted and the backend
  becomes authoritative.
- User previously stated caption orientation (horizontal vs vertical) is a
  project-level setting persisted into the Toaster project file and must be
  respected on import.
- 84 missing descriptions may lack product copy. Triage gate: any row
  without copy-ready text is descoped to a follow-up bundle issue rather
  than shipped with placeholder copy.
- No page may exceed the 800-line file cap introduced by AGENTS.md.

## 6. Data model

No new persisted data. Caption orientation already lives in the project file
(see `src-tauri/src/managers/project/` — not touched here; we only read it).

Any new i18n keys land in `src/i18n/locales/*/translation.json` for all 20+
locales. `scripts/check-translations.ts` must stay green.
