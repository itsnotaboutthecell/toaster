# PRD: Settings UI consistency audit

## Problem & Goals

Toaster's settings pages violate multiple implicit layout rules that the
About page alone satisfies. The user-visible result is inconsistent
outer padding, an oversized caption preview that spills over other
controls on vertical viewport, and an Export page that mixes label /
control axes. No automated gate catches any of this.

This feature ships an audit — not a fix. The audit produces a
prioritised, reproducible report that the follow-on
`settings-ui-consistency-fix` bundle consumes.

See `REQUEST.md` for verbatim user feedback.

## Scope

**In scope:** Playwright audit spec, PowerShell wrapper, audit report
artefacts, `data-testid` decoration on `SettingContainer` and
`CaptionPreviewPane`, docs appendix.

**Out of scope:** fixes, colour redesign, new deps, non-settings UI.

## Requirements

### R-001 — Audit enumerates every settings surface

- Rationale: user feedback spans About, Models, Captions, Advanced,
  Export; an audit that only covers a subset guarantees a recurring
  miss.
- Acceptance Criteria:
  - AC-001-a — Playwright spec visits all 5 routes: About, Models,
    Post-Process, Advanced, and the captions tab inside Advanced. Route
    list is hardcoded (not auto-discovered) so a missing route is a
    compile-time break.
  - AC-001-b — Every route is exercised at two viewports:
    desktop 1280×800 and mobile-portrait 390×844.
  - AC-001-c — Total wall-clock under 120 s on the reference dev
    machine; CI budget asserted by the wrapper script.

### R-002 — Outer-padding rule codified and enforced

- Rationale: About has correct padding; Models and Advanced do not.
- Acceptance Criteria:
  - AC-002-a — The audit extracts About's container padding +
    max-width as the canonical baseline and asserts every other route's
    outer container is within 8 px of that baseline on `padding-inline`
    and `max-width`.
  - AC-002-b — Violations are emitted with the exact measured
    values, the expected baseline, and the CSS selector.
  - AC-002-c — `audit.md` identifies the specific component file
    path (e.g. `src/components/settings/models/ModelsSettings.tsx`) for
    each violation so the fix-feature author can jump directly to the
    offending line.

### R-003 — Two-column row rule codified and enforced

- Rationale: Export violates left-label / right-control. Fixing Export
  in isolation is insufficient; the rule must be asserted for every
  setting row.
- Acceptance Criteria:
  - AC-003-a — Every `[data-testid^="setting-row"]` has a
    first-child element tagged `[data-setting-role="label"]` and a
    sibling `[data-setting-role="control"]`, with label preceding
    control in DOM order (`Node.compareDocumentPosition`).
  - AC-003-b — Computed style of the row has `display: flex` with
    `flex-direction: row` OR `display: grid` with ≥ 2 columns at
    desktop viewport. Mobile-portrait may collapse to column-direction
    — audit accepts either layout when viewport width < 500 px.
  - AC-003-c — Export page specifically is asserted to satisfy the
    rule (named test so a regression is unambiguous).

### R-004 — Caption preview clamped within its column

- Rationale: the user-reported spill is a critical failure.
- Acceptance Criteria:
  - AC-004-a — On desktop 1280×800, `[data-testid="caption-preview
    -pane"]` width ≤ 50 % of viewport and the pane fits within its
    parent's inline-size (no horizontal overflow).
  - AC-004-b — On mobile-portrait 390×844, the pane's height ≤
    40 % of viewport height and the pane does not push any sibling
    control below `document.documentElement.scrollHeight` (i.e. no
    cut-off).
  - AC-004-c — At 320 px viewport width, no horizontal scrollbar on
    `document.documentElement` or any ancestor of the preview pane.

### R-005 — Settings UI contract compliance

- Rationale: AGENTS.md already requires labels + descriptions + slider
  interaction + colour rules. The audit makes them machine-enforced.
- Acceptance Criteria:
  - AC-005-a — Every `[data-setting-role="label"]` has
    non-whitespace text that is not an ALL_CAPS snake / kebab / camel
    token (regex `^[A-Z][A-Z0-9_-]+$` or `^[a-z][a-zA-Z0-9]*$` flagged
    as "raw flag name leaked").
  - AC-005-b — Every label has a sibling `[data-setting-role=
    "description"]` with ≥ 1 rendered character. Empty-description
    rows are flagged.
  - AC-005-c — For each `input[type="range"]` inside a setting row,
    the audit asserts the adjacent node is an editable
    `<input type="number">` or contenteditable span (double-click-to
    -type affordance per AGENTS.md). Missing keyboard-entry affordance
    is flagged.
  - AC-005-d — Colour audit: no element with computed
    `color` in the red-family (H 350–20°, S ≥ 50 %, L ≤ 55 %) on an
    ancestor with `background-color` L ≤ 25 % (red-on-dark);
    no `color` with L ≥ 80 % on ancestor `background-color` L ≥ 90 %
    (light-grey-on-white).

### R-006 — Dual-format audit report

- Rationale: CI needs JSON; human reviewers need Markdown + screenshots.
- Acceptance Criteria:
  - AC-006-a —
    `features/settings-ui-consistency-audit/audit-report/audit.json`
    emitted with the schema defined in REQUEST.md §6 (page, viewport,
    rule, severity, selector, expected, actual, screenshotPath).
  - AC-006-b —
    `features/settings-ui-consistency-audit/audit-report/audit.md`
    emitted with a summary table (violations per page × severity) and
    per-page subsections embedding the screenshot paths.
  - AC-006-c — `scripts/migrate/audit-settings-ui.ps1` exits 0 when no
    `critical` violations, exits 1 otherwise. Non-critical (`major`,
    `minor`) do not fail the script — they go in the report for
    triage.

### R-007 — Audit attribute injection is non-behavioural

- Rationale: to measure the two-column rule, we need
  `data-setting-role` attributes. Those must not change rendered
  behaviour.
- Acceptance Criteria:
  - AC-007-a — `cargo check -p toaster --lib` still green after the
    Rust side is untouched (this is a frontend-only feature; gate
    ensures we didn't accidentally touch Tauri boundary).
  - AC-007-b — `bun run build` green; no new TypeScript errors.
  - AC-007-c — `bun run scripts/check-translations.ts` green; no
    new user-visible strings (audit attributes are data attributes only,
    not translatable content).

### R-008 — Documentation appendix

- Acceptance Criteria:
  - AC-008-a — `docs/settings-placement.md` gains a new section
    "Layout invariants" that names the four rules (padding, two-column,
    preview-clamp, contract) and links to the audit script.

## Edge cases & constraints

- Dynamic content: audit waits for `networkidle` + 200 ms debounce
  before measuring.
- Audit must be idempotent — re-running without code changes produces
  byte-identical `audit.json` (sort order guaranteed).
- Total audit artefact size (screenshots + reports) ≤ 50 MB; enforced
  by the wrapper which rejects oversized screenshot sets.
- No new npm dependencies. Use Playwright's built-in reporter API.

## Non-functional requirements

- Runs on Windows, macOS, Linux where Playwright's browsers install.
- Deterministic: fixed viewport sizes, disabled animations via
  `prefers-reduced-motion: reduce` emulation.
- Privacy: no external network calls during audit (Playwright launches
  Chromium against `http://localhost:1420`).
