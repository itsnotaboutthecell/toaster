# Settings UI consistency audit — baseline report

This directory contains the baseline audit output produced by running
`scripts/migrate/audit-settings-ui.ps1` against the repo at the time
`settings-ui-consistency-audit` was promoted to `reviewing`.

## Files

- `audit.json` — machine-readable violations list (schema v1). Ordered
  deterministically by (page, severity, rule, selector).
- `audit.md` — human-readable summary with per-page × severity table and
  per-violation details including screenshot links.
- `screenshots/` — one PNG per violation, named
  `<page>-<viewport>-<rule>-<index>.png`.

## Reading the baseline

The audit runs the rules defined in `tests/settingsUIAudit.spec.ts` and
documented in `docs/settings-placement.md § Layout invariants`.

### Top-level counts at baseline

- critical = 1 (R-004-desktop-width on the captions preview — user's
  "preview area is way too big" complaint, machine-confirmed).
- major = 130 (missing descriptions, slider-without-editable-sibling,
  Export / Advanced non-two-column rows, contrast issues).
- minor = 0.

### What happens next

This report is the input to the follow-on feature
`settings-ui-consistency-fix`. That bundle's PRD will prioritise the
critical + export-two-column clusters first (those were the user's
verbatim complaints) and treat the 84 missing-description rows as a
separate batch that the designer collaboration can triage.

### Re-running

```powershell
bun run dev                 # terminal 1
pwsh scripts/migrate/audit-settings-ui.ps1   # terminal 2, repo root
```

A passing run exits 0 (critical == 0). A failing run exits 1 and still
emits `audit.json` + `audit.md`.
