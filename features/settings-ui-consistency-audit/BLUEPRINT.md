# Blueprint: Settings UI consistency audit

## High-level architecture

```
bun run build (dev server on :1420)
    │
    ▼
scripts/migrate/audit-settings-ui.ps1  ── invokes ──▶  playwright test tests/settingsUIAudit.spec.ts
    │                                                      │
    │                                                      │ emits JSON via custom reporter
    │                                                      ▼
    │                                          test-results/settings-ui-audit/raw.json
    │                                                      │
    └── post-processes (sort, severity triage) ────────────┘
                                 │
                                 ▼
    features/settings-ui-consistency-audit/audit-report/{audit.json, audit.md, screenshots/*.png}
```

## Single-source-of-truth placement

Per AGENTS.md "Single source of truth for dual-path logic":

| Rule                | SSOT module                                         | Consumers            |
|---------------------|-----------------------------------------------------|----------------------|
| Layout rule list    | `tests/settingsUIAudit.spec.ts` (const `RULES`)     | audit spec + wrapper |
| Route list          | `tests/settingsUIAudit.spec.ts` (const `ROUTES`)    | audit spec           |
| Viewport list       | `tests/settingsUIAudit.spec.ts` (const `VIEWPORTS`) | audit spec           |
| Severity classifier | `tests/settingsUIAudit.spec.ts`                     | report writer        |
| Report schema       | `scripts/migrate/audit-settings-ui.ps1`                     | wrapper (validation) |

Audit attributes (`data-testid`, `data-setting-role`) live on existing
UI primitives (`SettingContainer`, `CaptionPreviewPane`) — they are the
SSOT for "this DOM node is a setting row / label / control / preview".
No duplicate selectors in the spec.

## Decisions per requirement

### R-001 Route + viewport enumeration

- Hardcode routes in the spec (`const ROUTES: ReadonlyArray<Route>`) so
  adding a settings page in the future is a compile-time break. Auto-
  discovery (e.g. crawling the sidebar) was rejected: it masks new
  routes that fail the audit.
- Viewports fixed at desktop-1280×800 and mobile-portrait-390×844.
  Additional mobile-landscape / tablet deferred — bigger is additive
  and out-of-scope for this bundle.

### R-002 Outer-padding rule

- Baseline measured once per spec run from About (not hardcoded) — if
  the canonical page's padding ever legitimately changes, the audit
  follows.
- Tolerance 8 px accommodates subpixel rounding and scrollbar-width
  differences between Chromium builds.
- File-path resolution: use Playwright's `page.$$eval` with the
  `data-testid="settings-outer"` attribute (added to the `<main>` of
  every settings page) + a parallel map in
  `tests/settingsUIAudit.spec.ts` from testid → source file.

### R-003 Two-column row rule

- `SettingContainer` is the single React primitive rendering a setting
  row. Attribute it once with
  `data-testid="setting-row"` and `data-setting-role="row"`; its
  children are attributed with `data-setting-role="label" | "control"
   | "description"`.
- Existing rows that bypass `SettingContainer` (observed in
  `ExportSettings.tsx`) are **not** migrated in this feature — their
  violations are the whole point of the audit.
- Mobile-portrait collapse is allowed (column direction) so we don't
  flag legitimately responsive layouts.

### R-004 Caption preview clamp

- Clamp rule is enforced on the rendered pane, not the CSS — measurement
  uses `getBoundingClientRect()` after `networkidle + 200 ms`.
- Percentages are of viewport, not parent — this catches the reported
  "spill over" case where the pane is 72 % of viewport in a 50 %-
  column.
- 320 px viewport narrow-width test is a third viewport asserted only
  for R-004-c, not the full spec matrix (keeps runtime under budget).

### R-005 Contract compliance

- Label text regex rejects known-bad patterns (all-caps snake, all-
  lowercase camel) but does not enforce positive prose quality — that's
  beyond machine scope. Humans review the flags in `audit.md`.
- Slider affordance check looks for a sibling editable node within the
  same setting row, consistent with the slider pattern already in use
  in the caption profile form.
- Colour audit uses Chromium's `getComputedStyle` converted to HSL via
  a small pure-JS converter injected at spec start. No new npm dep.

### R-006 Dual output

- JSON emitted by a custom Playwright reporter that appends to an in-
  memory array; serialised at suite end.
- Markdown rendered by the wrapper script (PowerShell; no new deps).
  Screenshot links are relative paths so the report is portable.
- Exit-code mapping: `critical > 0 → 1`, else `0`. `major` / `minor`
  are report-only to prevent the audit from blocking unrelated CI.

### R-007 Non-behavioural injection

- `data-*` attributes never affect rendering. Audit gate keeps
  `cargo check` green as a boundary canary.
- No new translation keys (data attributes are not user-visible).

### R-008 Docs

- Append-only to `docs/settings-placement.md`. The existing heuristic
  (frequency-of-use) stays intact.

## Data flow

1. Dev server running (`bun run dev` or monitored launcher).
2. Wrapper script pre-clears
   `features/settings-ui-consistency-audit/audit-report/`.
3. Wrapper invokes Playwright with the audit spec + custom reporter,
   passing output dir via env var.
4. Spec iterates ROUTES × VIEWPORTS, runs each RULE against each page,
   accumulates violations, writes screenshots.
5. Reporter emits `raw.json`.
6. Wrapper sorts (page → severity → selector) and writes `audit.json` +
   `audit.md`.
7. Wrapper prints a summary to stdout; exits 0/1 per critical count.

## Risk register

| ID  | Risk                                               | Mitigation                                                           |
|-----|----------------------------------------------------|----------------------------------------------------------------------|
| X-1 | Audit flakes on animation (caption preview)        | Emulate reduced motion; wait for networkidle + 200 ms                |
| X-2 | Chromium subpixel differences across OSes          | 8 px tolerance on padding; screenshot comparison uses % not px       |
| X-3 | Report grows unboundedly on a broken page          | Wrapper caps total artefact size at 50 MB; aborts with error        |
| X-4 | Dev server not running when wrapper invoked        | Wrapper runs `scripts/check-dev-server.ps1` pre-flight (new helper) |
| X-5 | Playwright browser binaries missing on fresh clone | Wrapper runs `bunx playwright install --with-deps chromium` once   |
| X-6 | Audit drifts from AGENTS.md rules over time        | R-008 doc appendix links both ways; CI runs audit weekly            |
| X-7 | New settings page added without audit coverage     | Hardcoded ROUTES list → compile break in spec                        |
| X-8 | False positive on dynamic model list (Models page) | Spec waits for `[data-testid="models-loaded"]` before measuring      |

## Out of scope (re-stated)

- Fixing any violation. That's the `settings-ui-consistency-fix`
  follow-on bundle, which consumes `audit-report/audit.md` as its PRD
  appendix.
- Visual regression (screenshot diffing) — audit is rule-based, not
  pixel-based, so we can evolve UI freely without re-baselining.
- Translating any audit output — the report is developer-facing.

## Open questions (tracked in journal.md)

- J-1: Should the audit be wired into CI (`bun run audit:settings`
  step) or remain an on-demand gate for the fix-feature author?
  Default: on-demand for this bundle; CI wiring is a decision for the
  fix bundle.
- J-2: Do we want to extend `RULES` to cover the keyboard-shortcut
  settings (Handy-era) that remain in the sidebar? Default: no — those
  are in the `handy-legacy-pruning` scope and may be removed entirely.
