# PRD: Settings UI consistency fix

## Problem & Goals

Resolve the violation clusters surfaced by the `settings-ui-consistency-audit`
baseline report (`features/settings-ui-consistency-audit/audit-report/`) and
burn down AGENTS.md "Settings UI contract" drift to zero across the settings
surface. Primary success metric is a clean re-run of
`scripts/audit-settings-ui.ps1` (critical = 0; targeted rule counts = 0 per
R-007).

## Scope

### In scope

- Fixes under `src/components/settings/**`, `src/components/ui/SettingContainer.tsx`,
  `src/components/settings/captions/**`.
- Caption preview re-design (horizontal/vertical designer) on the same
  CaptionProfileShared surface.
- i18n additions for all new labels/descriptions.

### Out of scope (explicit)

- New settings, new sidebar entries, new feature toggles.
- Backend (`src-tauri/**`) logic changes (unless dual-path SSOT move is
  required — see BLUEPRINT R-001 risk register).
- Completion of the 84 missing-description rows where stakeholder copy is
  not ready; those rows descope to a follow-up bundle per R-006.

## Requirements

### R-001 — Caption preview: sized correctly and rebuilt as orientation designer

- Description: The caption preview pane in `CaptionProfileShared.tsx` exceeds
  50% desktop width (audit R-004-desktop-width, 1 critical). Rebuild as a
  horizontal/vertical-orientation designer per
  `eval/fixtures/caption-mock-h-and-w.png` — arrow boundary lines + center
  lines, no screen-dimension text.
- Rationale: User-verbatim complaint; AGENTS.md dual-path SSOT rule requires
  preview to share the sizing policy with export.
- Acceptance Criteria
  - AC-001-a — Re-running `scripts/audit-settings-ui.ps1` shows zero
    `R-004-desktop-width` violations on the captions page at 1280x800.
  - AC-001-b — The preview pane visually matches the `caption-mock-h-and-w.png`
    reference: arrow boundaries + center lines only, toggleable horizontal
    vs vertical via radio.
  - AC-001-c — Preview width ≤ 50% of `max-w-5xl` container on desktop and
    does not overflow onto neighboring settings at any tested viewport.
  - AC-001-d — Caption sizing uses a single source of truth shared with
    export per AGENTS.md "Single source of truth for dual-path logic"; no
    duplicated layout constants in the frontend that also exist in the
    backend or vice versa.

### R-002 — Export page: Left = label, Right = control

- Description: Export rows violate the `SettingContainer` Left/Right
  pattern (audit R-003-export-two-column = 4; R-003-layout also present
  on Export page). Restructure Export rows to flow through
  `SettingContainer` or an equivalent two-column layout.
- Rationale: User-verbatim complaint; matches About's conformant pattern.
- Acceptance Criteria
  - AC-002-a — Re-running `scripts/audit-settings-ui.ps1` shows zero
    `R-003-export-two-column` violations on the export page.
  - AC-002-b — Every user-editable export setting appears as a row with a
    label (left) and control (right) at desktop viewport, matching About's
    layout token usage.

### R-003 — Page padding and density match About (reference page)

- Description: Advanced, Models, Export, Post-process pages cut up against
  container borders. About is the reference (audit returned zero padding
  violations on About). Normalize outer container padding and vertical
  rhythm tokens.
- Rationale: User-verbatim complaint ("cutting right up against borders");
  audit R-003-layout = 10.
- Acceptance Criteria
  - AC-003-a — Re-running `scripts/audit-settings-ui.ps1` shows zero
    `R-003-layout` violations across Advanced, Models, Export, Post-process.
  - AC-003-b — Each settings page outer div uses the same padding/spacing
    token (same class pair as About; no page uses `space-y-4` while a
    sibling uses `space-y-6`).

### R-004 — Sliders expose keyboard-entry sibling

- Description: 28 `range-editable` violations — sliders without an adjacent
  numeric input. AGENTS.md "Settings UI contract" requires typed-entry
  support alongside drag.
- Rationale: AGENTS.md hard rule; user-reported in prior session.
- Acceptance Criteria
  - AC-004-a — Re-running `scripts/audit-settings-ui.ps1` shows zero
    `R-005-range-editable` violations.
  - AC-004-b — Every `input[type=range]` has an adjacent `input[type=number]`
    bound to the same state; typing a value moves the slider and vice versa.

### R-005 — Color contrast cleanup

- Description: 4 `R-005-color-light-grey-on-white` violations. No
  red-on-dark or light-grey-on-white anywhere in settings.
- Rationale: AGENTS.md "Settings UI contract"; repeatedly reported
  readability regression.
- Acceptance Criteria
  - AC-005-a — Re-running `scripts/audit-settings-ui.ps1` shows zero
    `R-005-color-light-grey-on-white` violations.
  - AC-005-b — Only existing color tokens are used; no new greys or reds
    introduced (verified by reviewer via `git diff` inspection on
    `tailwind.config.js` + page files).

### R-006 — Missing descriptions (burn-down with explicit descope allowed)

- Description: 84 `R-005-missing-description` violations. Each `SettingContainer`
  usage must supply both `label` and `description` props with product-ready
  copy. If copy is not ready for a given row, that row's fix descopes to a
  follow-up bundle issue rather than shipping placeholder text.
- Rationale: AGENTS.md "Settings UI contract"; scope is large enough to
  explicitly accept partial delivery.
- Acceptance Criteria
  - AC-006-a — Every row whose copy IS ready gets a `description` prop with
    an i18n key (no hardcoded strings). Count reduction reported in QC.
  - AC-006-b — Any row descoped is tracked in
    `features/settings-ui-consistency-fix/audit-report-after/descoped.md`
    with selector + reason; the follow-up bundle slug is named in the file.
  - AC-006-c — `scripts/check-translations.ts` remains green; every new
    key exists in all locale files.

### R-007 — Final audit gate

- Description: Re-run the audit; numeric gates per cluster.
- Rationale: Single machine-enforced finish line.
- Acceptance Criteria
  - AC-007-a — `scripts/audit-settings-ui.ps1` exits 0 (`critical == 0`).
  - AC-007-b — `R-004-desktop-width`, `R-003-export-two-column`,
    `R-003-layout`, `R-005-range-editable`, `R-005-color-light-grey-on-white`
    all equal 0 in the new `audit.json`.
  - AC-007-c — `R-005-missing-description` count is either 0, or strictly
    less than baseline (130) with every remaining row listed in
    `audit-report-after/descoped.md`.

### R-008 — Static + live-app gates

- Description: Repository-wide sanity gates pass, and a live-app pass
  confirms the critical caption preview fix behaves correctly in the real
  app (AGENTS.md "Verified means the live app").
- Rationale: Multiple prior sessions shipped "green" compile runs that the
  live app invalidated.
- Acceptance Criteria
  - AC-008-a — `bun run lint`, `bun run build`, `scripts/check-translations.ts`
    all exit 0 after implementation.
  - AC-008-b — Live-app pass via `scripts/launch-toaster-monitored.ps1`:
    navigate to Captions, toggle horizontal↔vertical, drag the sliders,
    confirm no overflow and no click-to-type regression.

<!--
  AC IDs MUST NOT be bold. Use `AC-001-a` verbatim, never `**AC-001-a**`.
  Coverage gate regex: ^\s*-?\s*AC-\d{3}-[a-z]\b
-->

## Edge cases & constraints

- Mobile viewport (390x844) must not regress — audit covers this.
- Caption orientation is a project-level setting; preview must read from
  project state and respect imported projects.
- 800-line file cap per AGENTS.md; split if adjacent refactors push a file
  over.
- No hosted inference; no new dependencies from the dep-hygiene skill's
  blocklist.

## Data model

No new persisted state. Project-level caption orientation is already in the
project file schema; this feature reads it.

## Non-functional requirements

- i18n: all new user-facing strings behind i18next keys; parity gate green.
- Single-source-of-truth for preview↔export caption sizing.
- No backend logic changes unless SSOT move demands it (see BLUEPRINT risk
  register).
