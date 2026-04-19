# PRD: brand title sizing

## Problem & Goals

The Toaster wordmark on the main view (the `toaster_text.svg`
rendered in the sidebar at `src/components/Sidebar.tsx:76`) is small,
and the main content area on the right of the sidebar reserves
significant unused horizontal space at the default 1280x800 viewport
(outer `p-4` plus inner `max-w-3xl`/`max-w-4xl` caps). The user wants
both addressed in one surgical token-only change: enlarge the
wordmark, reclaim horizontal gutter, do not restructure layout.

## Scope

### In scope

- Width / margin tokens on the wordmark `<img>` inside
  `src/components/Sidebar.tsx`.
- Horizontal padding token on the main content wrapper at
  `src/App.tsx:205`.
- `max-w-*` cap tokens on the root containers of `EditorView.tsx`,
  `AboutSettings.tsx`, `AdvancedSettings.tsx`,
  `settings/history/HistorySettings.tsx`,
  `settings/models/ModelsSettings.tsx`.

### Out of scope (explicit)

- Sidebar width or section list.
- TranscriptEditor / EditorToolbar / FillerDashboard internals.
- New brand asset, raster fallback, animations.
- Color tokens (AGENTS.md "Settings UI contract").
- i18n keys.
- Onboarding view, footer, error-boundary fallback.

## Requirements

### R-001 -- Brand wordmark renders larger on the main view

- Description: increase the rendered width of `toaster_text.svg`
  inside the sidebar from `w-[120px]` to `w-[144px]` (max that fits
  without sidebar restructuring), and tighten its margin so horizontal
  padding inside the sidebar does not eat into the bump.
- Rationale: the wordmark is the sole brand surface on the main view;
  +20 % width meaningfully increases brand presence at the default
  viewport without triggering a sidebar layout change.
- Acceptance Criteria
  - AC-001-a -- At a 1280x800 viewport, the rendered bounding box of
    `img[alt="Toaster"]` inside the sidebar has a measured CSS width
    of >=140 px and <=144 px.
  - AC-001-b -- The wordmark remains fully inside the sidebar (its
    right edge does not overflow the 160 px sidebar column at
    1280x800).
  - AC-001-c -- The wordmark uses the existing `toaster_text.svg`
    asset (imported at `src/components/Sidebar.tsx:4`); no new image
    asset is added to the repository.

### R-002 -- Reclaim horizontal gutter on the main content area

- Description: remove the horizontal component of the `p-4` on the
  main content wrapper (`src/App.tsx:205`) and raise the inner view
  caps from `max-w-3xl` / `max-w-4xl` (768 / 896 px) to `max-w-5xl` /
  `max-w-6xl` (1024 / 1152 px) on the editor and settings root
  containers.
- Rationale: at 1280x800, today's effective editor content width is
  capped at 896 px even though >1080 px is available; the user calls
  this "excess padding".
- Acceptance Criteria
  - AC-002-a -- At a 1280x800 viewport on the editor view, the
    measured CSS width of the editor root container
    (`<div class="max-w-... w-full mx-auto ...">` inside
    `EditorView.tsx`) is >=1024 px.
  - AC-002-b -- At a 1280x800 viewport on the editor view, the
    horizontal distance from the right edge of the sidebar to the
    left edge of the editor root container is <=8 px (i.e. the
    outer wrapper's horizontal padding is at most `px-2`).
  - AC-002-c -- The same measurement on the About, Advanced, History,
    and Models settings panes shows their root container width
    >=960 px at 1280x800.

### R-003 -- Responsive behavior holds at narrow and wide viewports

- Description: at <768 px the raised caps must be no-ops (inner
  `w-full` collapses to viewport width minus sidebar); at >=1920 px
  the caps still apply (content stays centered with surrounding
  whitespace).
- Rationale: the change must not introduce horizontal scrolling on
  small windows or full-bleed text on huge displays.
- Acceptance Criteria
  - AC-003-a -- At a 720x800 viewport, the editor view produces no
    horizontal page scroll bar (document.scrollWidth <=
    window.innerWidth).
  - AC-003-b -- At a 1920x1080 viewport, the editor root container
    measured CSS width is <=1152 px (the new `max-w-6xl` cap).
  - AC-003-c -- At a 720x800 viewport, the wordmark right edge does
    not overflow the sidebar column (same invariant as AC-001-b at a
    smaller viewport).

### R-004 -- No regression in editor / settings layout

- Description: the transcript editor and each settings pane continue
  to render every section / control they render today.
- Rationale: this is a token-only diff; behavior must not change.
- Acceptance Criteria
  - AC-004-a -- The editor view at 1280x800 still renders, in this
    order: the import-media block, the project actions block, the
    transcription start block, and the edit toolbar block (sections
    visible at `EditorView.tsx:402`, `:484`, `:500`, `:527`).
  - AC-004-b -- The About settings pane still renders its description
    paragraph (`AboutSettings.tsx:62`) without horizontal clipping.
  - AC-004-c -- All Playwright specs in `tests/` (`app.spec.ts`,
    `transcriptEdit.spec.ts`, `exportPipeline.spec.ts`,
    `settingsRoundTrip.spec.ts`, `skipSchedule.spec.ts`) pass.

### R-005 -- Discipline gates remain green

- Description: the diff must not push any touched file past the
  AGENTS.md 800-line cap and must not introduce new color tokens or
  i18n keys.
- Rationale: AGENTS.md "Settings UI contract" and "file-size cap 800
  lines" must hold.
- Acceptance Criteria
  - AC-005-a -- `bun run check:file-sizes` exits 0 after the diff
    (verifier wraps `scripts/check-file-sizes.ts`).
  - AC-005-b -- Grep over the diff for `#[0-9A-Fa-f]{3,8}` shows zero
    new color literals introduced (only `w-*`, `m-*`, `px-*`, `py-*`,
    `max-w-*` token churn permitted).

## Edge cases & constraints

- ASCII only in planning artifacts.
- No proprietary fixtures.
- Sidebar usable inner width = 160 - 2*8 = 144 px; the `w-[144px]`
  wordmark consumes it fully -- any further increase would require
  sidebar restructuring (out of scope).
- `items-center` on the App.tsx wrapper centers the inner view; with
  the outer `px` removed the inner `max-w-*` cap becomes the sole
  gutter source.
- The settings panes share one cap value via convention, not via a
  shared component; the diff touches each pane explicitly.

## Data model

n/a -- presentational only.

## Non-functional requirements

- Each touched `.ts` / `.tsx` file remains <= 800 lines (AGENTS.md
  file-size cap, `AGENTS.md:142`).
- No new dependencies (Rust crate or npm package); `dep-hygiene`
  skill is not triggered.
- No change to `superpowers:transcript-precision-eval` /
  `audio-boundary-eval` surface.
- "Verified" follows AGENTS.md "Verified means the live app, not
  cargo check" (`AGENTS.md:151-153`); visual ACs are gated through
  `scripts/launch-toaster-monitored.ps1`.
