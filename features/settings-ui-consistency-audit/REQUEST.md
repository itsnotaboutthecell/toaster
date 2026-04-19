# Feature request: Settings UI consistency audit

## 1. Problem & Goals

The settings pages in Toaster have inconsistent visual layout and spacing
that creates a poor user experience. Specific issues reported by the user
during live QC on 2026-04-18:

- **About** has correct padding from the viewport edges.
- **Models** cuts against the borders (insufficient outer padding).
- **Advanced → Captions** area is "really bad" — the live preview pane is
  way too big and on vertical orientation it spills over the other
  settings in the column, pushing controls off-screen.
- **Export** does not follow the label-left / control-right pattern used
  elsewhere; drop-downs and inputs are laid out inconsistently.
- No automated QA signal catches any of this — the regressions landed
  silently across multiple recent feature merges.

> "We really need playwright and a designer to go through our menus and
> find all the inconsistencies and to work with the PM to develop a
> plan."

**Goal:** produce a Playwright-driven audit that enumerates every
settings page at desktop + mobile viewports, captures layout violations
against a codified rule set (outer padding, two-column row, preview
clamping, contract compliance), and emits both a machine-readable
(`audit.json`) and human-readable (`audit.md`) report. Implementation
fixes are a separate follow-on feature bundle grounded in this audit's
concrete findings.

## 2. Desired Outcome & Acceptance Criteria

- `scripts/audit-settings-ui.ps1` examines all 5 settings routes (About,
  Models, Post-Process, Advanced, Editor-adjacent captions tab) at both
  desktop (1280x800) and mobile-portrait (390x844) viewports.
- For each page it asserts:
  - outer container matches the canonical pattern used by About
    (tolerance 8 px on measured `padding-inline` / `max-width`);
  - every `[data-testid^="setting-row"]` has label-before-control DOM
    order and a two-column flex/grid structure;
  - caption live preview width ≤ 50% of viewport and height ≤ 40% of
    viewport on mobile; no horizontal overflow at 320 px width;
  - every setting row has a human-readable label + one-line description
    (no raw enum names) per AGENTS.md "Settings UI contract";
  - no red text on dark backgrounds or light-grey text on white (contrast
    class check).
- Output: `features/settings-ui-consistency-audit/audit-report/audit.json`
  + `audit.md` with severity (critical / major / minor), per-violation
  selector, screenshot path.
- Script exits 0 when no `critical` violations; exits 1 otherwise.
- Total runtime ≤ 120 s on the reference dev machine.
- All 20 i18n locales still pass `scripts/check-translations.ts`.

See `PRD.md` for the formalised ACs.

## 3. Scope Boundaries

### In scope

- New Playwright spec `tests/settingsUIAudit.spec.ts` covering the rules
  above.
- New PowerShell wrapper `scripts/audit-settings-ui.ps1` that launches
  the spec, post-processes the Playwright JSON reporter output, and
  writes `audit.json` + `audit.md`.
- Extending `SettingContainer` and `CaptionPreviewPane` with
  audit-friendly `data-testid` attributes (non-behavioural change).
- Audit report committed under
  `features/settings-ui-consistency-audit/audit-report/` as the reference
  baseline for the follow-on fix feature.
- Documentation of the canonical layout rules in
  `docs/settings-placement.md` (appended; does not replace the existing
  frequency-of-use heuristic).

### Out of scope (explicit)

- **Fixing any defect the audit finds.** That is a separate feature
  bundle (`settings-ui-consistency-fix`) whose PRD is grounded in this
  audit's report.
- Sidebar restructure (already landed in `sidebar-5-lane-restructure`).
- Caption profile persistence / orientation designer (already landed).
- Colour or typography redesign — codify the existing tokens only; do
  not pick new ones.
- Hosted design-tool integrations (Figma, etc.). Local-only per
  AGENTS.md "Local-only inference" spirit.
- Modifying the Playwright baseline screenshots for any test other than
  the new `settingsUIAudit` spec.

## 4. References to existing code

- `src/components/settings/about/AboutSettings.tsx` — canonical padding
  pattern.
- `src/components/settings/models/ModelsSettings.tsx` — known padding
  violator.
- `src/components/settings/captions/CaptionSettings.tsx`,
  `CaptionProfileForm.tsx`, `CaptionProfileShared.tsx` — caption preview
  sizing offender (vertical orientation spill).
- `src/components/settings/export/ExportSettings.tsx` — two-column
  pattern violator.
- `src/components/settings/advanced/AdvancedSettings.tsx` — container
  for the Captions group.
- `src/components/ui/SettingsGroup.tsx`,
  `src/components/ui/SettingContainer.tsx` — existing two-column
  primitives; audit attaches to these.
- `tests/app.spec.ts`, `tests/skipSchedule.spec.ts`,
  `playwright.config.ts` — existing harness.
- `AGENTS.md` "Settings UI contract" — human-readable label +
  description + slider interaction + colour rules.
- `docs/settings-placement.md` — the frequency-of-use heuristic from
  `advanced-menu-restoration`; this feature appends layout rules.

## 5. Edge cases & constraints

- Dynamic content (Whisper model list, caption-preview animation) must
  not trigger false positives; audit waits for stable layout before
  measuring.
- Audit must not mutate app state (no save, no upload). Read-only.
- Audit total size on disk (screenshots + reports) ≤ 50 MB.
- Audit must run with the existing `bun run build` + `bun playwright`
  toolchain; no new npm dependencies.
- Report output must be deterministic across runs (sort violations by
  page → severity → selector).

## 6. Data model

Audit report JSON (one array element per violation):

```json
{
  "page": "settings/advanced/captions",
  "viewport": "mobile-portrait-390x844",
  "rule": "preview-clamp",
  "severity": "critical",
  "selector": "[data-testid='caption-preview-pane']",
  "expected": { "maxWidthPct": 50, "maxHeightPct": 40 },
  "actual":   { "widthPct": 72.4, "heightPct": 58.1 },
  "screenshotPath": "audit-report/screenshots/advanced-captions-mobile.png"
}
```
