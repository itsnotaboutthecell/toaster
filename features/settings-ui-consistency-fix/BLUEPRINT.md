# BLUEPRINT: Settings UI consistency fix

## Architecture decisions per requirement

### R-001 — Caption preview resize + orientation designer

- **Where the logic lives.** Preview and export already diverge in practice;
  that is why the user's complaint exists. The correct home for caption
  sizing policy (font size multiplier, bounding-box margin, anchor position
  per orientation) is the backend (`src-tauri/src/managers/captions/` if
  present, otherwise the FFmpeg caption renderer path). Frontend consumes
  via a Tauri command + the generated `bindings.ts`. Any duplicated constants
  in the frontend are deleted as part of this task.
- **Designer UI.** Rebuild `CaptionProfileShared.tsx` preview pane to render
  one of two simple SVG layouts (horizontal or vertical) per
  `eval/fixtures/caption-mock-h-and-w.png`: arrow boundary lines on the
  outer edges, cross-hair center lines, single caption bounding box
  positioned by the current orientation + anchor setting.
- **Width.** Preview container gets `max-w-[calc(50%-1rem)]` at desktop
  breakpoint to guarantee audit R-004-desktop-width compliance; mobile
  layout collapses to single-column (existing behavior).
- **Radio.** Orientation radio lives on the same row as the preview; reads
  from project state; dispatches the existing "update caption orientation"
  action.

### R-002 — Export page two-column

- Route Export rows through `SettingContainer` (already the canonical
  two-column component). Any Export-specific custom row that cannot be
  expressed in `SettingContainer` gets a new lightweight wrapper in
  `src/components/ui/` that preserves the same `[data-setting-role="row|
  label|description|control"]` attributes.

### R-003 — Page padding normalization

- About uses the reference outer classes. Extract the current About outer
  div's Tailwind class list into a named constant in
  `src/components/settings/shared/pageLayout.ts` and apply to every
  settings page outer. No page overrides the padding/vertical-rhythm
  tokens.

### R-004 — Slider keyboard entry

- Wrap every existing `input[type=range]` usage in a small
  `RangeWithNumber` component in `src/components/ui/`. Props:
  `min,max,step,value,onChange`. Renders both inputs bound to the same
  state with two-way sync. Inherits the existing `SettingContainer`
  layout.

### R-005 — Color contrast

- Walk each violation in `audit.json` and map the offending class to an
  existing token (see `tailwind.config.js`). Forbidden transitions:
  any `text-gray-300` / `text-neutral-300` on a `bg-white`/`bg-stone-50`
  parent; any red on a dark background. Replace with tokens already used
  by About.

### R-006 — Missing descriptions burn-down

- Implementer iterates `audit.json` for `R-005-missing-description` rows.
  For each, check if the label key already has a matching
  `.description` sibling in `translation.json`; if yes, wire it. If the
  description copy does not exist, write the row selector + i18n key + the
  reason into `audit-report-after/descoped.md` and move on.
- Under no circumstances do we invent copy or ship lorem.

### R-007 / R-008 — Gates

- Re-run `scripts/audit-settings-ui.ps1` as the final machine gate.
- Run `bun run lint`, `bun run build`, `scripts/check-translations.ts`.
- Live-app pass per `scripts/launch-toaster-monitored.ps1`.

## Single-source-of-truth enforcement

- Caption sizing: backend authoritative; frontend reads via bindings. No
  parallel constants. Reviewed in spec-compliance review, not just code.
- Filler list: unchanged here, but the temptation will exist; if a
  future-self proposes hardcoding anything in frontend that also lives in
  backend, refuse and push to backend. AGENTS.md "Single source of truth
  for dual-path logic" is the rule.

## i18n policy

- Every new label/description adds a key under its page's namespace in
  `src/i18n/locales/en/translation.json`, then is mirrored to every other
  locale via the normal process. The `i18n-pruning` skill's parity script
  (`scripts/check-translations.ts`) is the gate.

## Risk register

1. **R-006 copy shortage — HIGH probability.** 84 rows is a lot; likely
   some have no product copy. Mitigation: descope protocol (`descoped.md`)
   spelled out in AC-006-b.
2. **Caption SSOT move — MEDIUM.** If the backend doesn't currently own
   caption sizing, moving it there is a larger change than a pure frontend
   fix. Mitigation: implementer must stop and escalate before inventing a
   new backend module; the alternative is to create a frontend helper that
   the existing export path can also import (i.e. shared `.ts` file under
   `src/lib/captions/`) rather than splitting the implementation.
3. **File-size cap regression — LOW.** Per-page refactors can push a file
   over 800 lines. Mitigation: split as they go; don't add to the
   allowlist.
4. **Audit flakiness — LOW.** Playwright timing. Mitigation: re-run twice
   on any deltas < 3 violations.

## Dependencies & tooling

- No new npm or cargo deps. `dep-hygiene` skill applies before adding any.
- Existing Playwright + the `scripts/audit-settings-ui.ps1` wrapper are
  sufficient.
