# Feature request: brand title sizing

## 1. Problem & Goals

User-verbatim:

> Make the toaster brand title bigger on the main page, we still have
> excess padding on the left and right that would help increase our
> awareness

The Toaster wordmark currently renders as a 120 px-wide SVG inside the
160 px sidebar (`src/components/Sidebar.tsx:76`). At the typical
1280x800 viewport the inner content area is further constrained by
`max-w-4xl` (editor, 896 px) / `max-w-3xl` (settings, 768 px) wrappers,
plus an outer `p-4` on `src/App.tsx:205` -- leaving large left/right
gutters. The user perceives that wasted horizontal real estate as both
a layout problem (gutters) and a brand visibility problem (the wordmark
is small).

## 2. Desired Outcome & Acceptance Criteria

- The Toaster wordmark renders measurably larger on the main view at
  the default 1280x800 viewport.
- The unused left/right gutter on the main content area is reduced by
  a measurable amount at 1280x800.
- The transcript editor and settings panes still render correctly and
  remain usable at narrow (<768 px), default (1280 px), and wide
  (>=1920 px) viewports.
- No new color tokens or layout primitives introduced.

## 3. Scope Boundaries

### In scope

- Width / margin tokens on the existing `<img>` inside
  `src/components/Sidebar.tsx`.
- Outer page-padding token on `src/App.tsx:205` wrapper
  (`<div className="flex flex-col items-center p-4 gap-4">`).
- `max-w-*` cap tokens on the root containers of `EditorView.tsx`,
  `AboutSettings.tsx`, `AdvancedSettings.tsx`,
  `settings/history/HistorySettings.tsx`,
  `settings/models/ModelsSettings.tsx`.

### Out of scope (explicit)

- Sidebar restructuring (width/layout/section list unchanged).
- TranscriptEditor internals, EditorToolbar, FillerDashboard layout.
- New brand asset; we keep `toaster_text.svg` (already imported at
  `src/components/Sidebar.tsx:4`).
- Color tokens, dark-mode tweaks, hover-state changes.
- i18n keys (no new strings added; the wordmark is an SVG with
  `alt="Toaster"`).
- Onboarding view, footer, error-boundary fallback.

## 4. References to Existing Code

- `src/components/Sidebar.tsx:75-76` -- current wordmark (`w-[120px] m-4`)
  inside `w-40 ... px-2` sidebar; the only literal "brand title" on the
  main view.
- `src/App.tsx:205` -- main content wrapper:
  `<div className="flex flex-col items-center p-4 gap-4">`. The `p-4`
  is the only horizontal padding the main wrapper applies; combined
  with `items-center` plus child `max-w-*` it produces the gutters the
  user wants reclaimed.
- `src/components/editor/EditorView.tsx:400` --
  `<div className="max-w-4xl w-full mx-auto space-y-6">` (editor cap).
- `src/components/settings/about/AboutSettings.tsx:31`,
  `src/components/settings/advanced/AdvancedSettings.tsx:20`,
  `src/components/settings/history/HistorySettings.tsx:242`,
  `src/components/settings/models/ModelsSettings.tsx:200,209` --
  shared `max-w-3xl w-full mx-auto` settings cap.
- `toaster_text.svg` (repo root) -- already vector, scales without
  rasterization artifacts.
- Pattern reference for "increase a Tailwind size token without
  restructuring": none new -- this is a token-only diff.

## 5. Edge Cases & Constraints

- AGENTS.md "Settings UI contract" -- color tokens (`#EEEEEE`, accent
  orange) must not be touched (`AGENTS.md:155-159`).
- AGENTS.md "file-size cap 800 lines" -- all touched files must remain
  <= 800 lines after the edit (`AGENTS.md:142`).
- AGENTS.md "Verified means the live app" -- visible-layout ACs must be
  driven through `scripts/launch-toaster-monitored.ps1`, not
  `cargo check` (`AGENTS.md:151-153`).
- Sidebar inner usable width is 160 - 2*8 = 144 px (w-40 minus px-2);
  any wordmark width past 144 px would clip or force sidebar restructure
  (out of scope).
- At <768 px viewport, raised `max-w-*` caps must be no-ops (the inner
  `w-full` already collapses to viewport width).
- Existing `tests/app.spec.ts`, `tests/transcriptEdit.spec.ts`,
  `tests/exportPipeline.spec.ts`, `tests/settingsRoundTrip.spec.ts`,
  `tests/skipSchedule.spec.ts` must keep passing -- these use selectors,
  not pixel widths.

## 6. Data Model

n/a -- pure presentational change.

## Q&A

The user request is unambiguous on the goal but underspecified on the
exact target sizes. The PRD pins concrete values:

- R-001 target wordmark width: **144 px** (current 120 px, +20 %).
  Rationale: 144 px is the maximum that fits inside the existing
  `w-40` (160 px) sidebar after `px-2` (8 px each side) without any
  sidebar restructuring.
- R-002 target reclaimed horizontal padding: outer wrapper drops
  horizontal component of `p-4` (16 px each side -> 0 px), and inner
  view caps move from `max-w-3xl/4xl` (768 / 896 px) to
  `max-w-5xl/6xl` (1024 / 1152 px). Effective editor content width at
  1280x800 (sidebar 160, no outer `px`): min(1120, 1152) = 1120 px,
  vs. current min(1088, 896) = 896 px. Net gain: +224 px (+25 %).
- R-003 responsive rule: caps are upper bounds; below them `w-full`
  collapses naturally. No new media queries.
- R-004 asset: keep the existing `toaster_text.svg` import. No new
  asset, no rasterization.
- R-005 regression scope: editor + each settings pane must continue to
  render their existing controls; verified via the live-app smoke
  steps in `coverage.json`.

No interactive Q&A needed; the user delegated these decisions to the
PM agent (see "Decisions you must make in the PRD" in the task brief).
