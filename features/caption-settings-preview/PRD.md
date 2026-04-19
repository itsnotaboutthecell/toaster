# PRD: Caption Settings Preview

## Problem & Goals

Caption-tuning controls in `CaptionSettings.tsx` are currently
"sightless" - the user must close settings and scrub the timeline to
verify each adjustment. This feature surfaces a faithful caption pill
inside the settings panel that updates in real time, so the user can
tune font size, padding, opacity, radius, color, and family without
leaving Advanced.

The dual-path-logic rule applies: the settings preview must reuse the
same renderer the live player uses, the same renderer that already
matches the export-side authority in
`src-tauri/src/managers/captions/ass.rs`. Adding a third caption
renderer is the defect class this PRD exists to prevent.

## Scope

### In scope

- A `CaptionPreviewPane` rendered above the existing controls in
  `src/components/settings/CaptionSettings.tsx`.
- Reuse of the existing `CaptionOverlay` component (or its inner
  pill) over a static background frame.
- A 3-option dropdown (short / two-line / long) of sample caption
  strings, defaulting to the user-quoted "looking crispy".
- Background: first frame of the bundled
  `eval/fixtures/toaster_example.mp4`. Fallback: flat `#1a1a1a`.
- Wiring through the existing Zustand settings store so changes
  propagate without debounce.
- New i18n strings limited to the dropdown legend, the three sample
  strings, and one preview-pane heading.

### Out of scope (explicit)

- Camera / `getUserMedia` live preview. Tracked as a follow-up
  feature (provisional slug `caption-settings-camera-preview`).
- Any modification to caption rendering math, ASS export, or backend
  caption authority.
- New fixture assets beyond what `eval/fixtures/` already ships.
- Persisting `selectedSampleKey` across app restarts.
- Reflowing the rest of Settings (only the caption section gains a
  pane).

## Requirements

### R-001 - Static preview is the chosen approach; camera is deferred

- Description: the settings preview renders a static frame plus a
  caption pill. Camera-based live preview is explicitly deferred as
  a follow-up feature.
- Rationale: equivalent user value at a fraction of the complexity;
  no `getUserMedia` permissions UX, no privacy implications, no
  device enumeration, no error states for revoked permission.
- Acceptance Criteria
  - AC-001-a - Opening Settings -> Captions in the live app shows a
    preview pane that uses a static background frame and a caption
    pill, with no camera permission prompt.
  - AC-001-b - `PRD.md` and `BLUEPRINT.md` for this feature both
    explicitly mark camera-based preview as out of scope and name a
    follow-up feature slug placeholder.

### R-002 - Placement, geometry, and visibility

- Description: the preview sits at the top of the caption section
  and stays visible while the user manipulates the controls below.
  It is rendered at 16:9 contain-fit at the current panel width.
- Rationale: the user must see the effect of the slider currently
  under their finger; if the preview scrolls off-screen the feature
  loses its purpose.
- Acceptance Criteria
  - AC-002-a - In the live app, opening Settings -> Captions shows
    the preview pane immediately above the first caption control,
    with no controls above it within the caption section.
  - AC-002-b - In the live app, scrolling the caption section down
    to reach the last caption control keeps the preview pane
    visible (sticky positioning) without overlapping the section
    heading.
  - AC-002-c - Resizing the settings panel width changes the
    preview's pixel width while preserving 16:9 aspect (no letter-
    or pillarboxing of the background frame inside the pane).

### R-003 - Placeholder text and background fixture

- Description: the preview defaults to the user-quoted text
  "looking crispy" and exposes a 3-option dropdown for short /
  two-line / long sample strings. The background is the first frame
  of `eval/fixtures/toaster_example.mp4`; if unavailable at runtime
  the pane falls back to a flat `#1a1a1a` and still renders the
  pill.
- Rationale: line-wrap, padding, and contrast behave differently for
  one-line, two-line, and long captions; users tuning padding and
  background opacity need to see all three. Reusing an existing
  fixture avoids asset bloat.
- Acceptance Criteria
  - AC-003-a - Opening Settings -> Captions in the live app shows
    the caption pill rendering the literal text "looking crispy" by
    default.
  - AC-003-b - The dropdown above the preview offers exactly three
    options labelled short / two-line / long; selecting each one
    swaps the rendered text immediately, the pill grows to two
    visible lines for the two-line option, and wraps to three or
    more visible lines for the long option.
  - AC-003-c - When the bundled background frame fails to load
    (simulated by renaming the source asset for the test session),
    the pane renders a flat `#1a1a1a` background and the caption
    pill still appears with no console errors.

### R-004 - Single source of truth: reuse the player's renderer

- Description: the preview pill MUST be rendered by importing the
  existing `CaptionOverlay` component (or extracting its inner pill
  into a sibling export and importing that). No new caption-
  rendering module is introduced. The preview consumes the same
  Zustand caption settings the player consumes.
- Rationale: AGENTS.md "Single source of truth for dual-path logic".
  The original preview-vs-export caption mismatch came from
  duplicating renderer logic; introducing a third renderer here
  would reproduce that defect class.
- Acceptance Criteria
  - AC-004-a - `src/components/settings/CaptionSettings.tsx` imports
    the existing caption pill renderer from
    `src/components/player/CaptionOverlay.tsx` (either the default
    export or a newly added named export of the inner pill); no new
    file under `src/components/` other than the preview-pane
    wrapper is added.
  - AC-004-b - No new file matching
    `src/**/Caption*Render*.{ts,tsx}` or
    `src/components/settings/CaptionPill*.{ts,tsx}` is introduced
    (i.e. the pill is not re-implemented inside the settings tree).
  - AC-004-c - The preview pane reads caption settings from the
    same Zustand store the player reads from, by reference to the
    same hook (`useSettings` per
    `src/components/settings/CaptionSettings.tsx:3`). No parallel
    store is added.

### R-005 - Live-update latency: one render frame, no debounce

- Description: changes made in the caption controls must appear in
  the preview within one render frame (<= 16ms at 60fps). No
  debounce wrapper, no `setTimeout`, no `requestIdleCallback`.
- Rationale: the value of the feature is real-time feedback while
  dragging. Anything that lags behind the cursor erodes that value.
- Acceptance Criteria
  - AC-005-a - In the live app, dragging the font-size slider with
    the preview visible shows the pill resizing in lockstep with
    the cursor; there is no perceptible lag and no "settle" frame
    after the user releases the slider.
  - AC-005-b - Source review of the preview-pane wiring shows no
    `debounce`, `throttle`, `setTimeout`, or `requestIdleCallback`
    between the settings store and the rendered pill props.
  - AC-005-c - In the live app, changing the background-opacity
    slider, the radius slider, and the font-family dropdown each
    updates the preview within the same frame as the control's own
    visual state change.

## Edge cases & constraints

- Bundled background fixture missing at runtime -> flat-grey
  fallback (AC-003-c).
- Settings panel resized very narrow -> preview keeps 16:9 contain
  (AC-002-c). Below a sensible minimum width (e.g. 240px) the pane
  may collapse, but must not overflow the panel.
- A user with very long localized strings could pick "long" and see
  many wrapped lines; the pill must not exceed the preview pane
  height (overflow hidden, no layout-breaking growth).
- ASCII-only source changes (project rule). The literal "looking
  crispy" is ASCII; no smart quotes.

## Data model (if applicable)

No persistent data. Component-local React state only:

- `selectedSampleKey: 'short' | 'twoLine' | 'long'` (default
  `'short'`)
- `backgroundReady: boolean` (default `false`, set on image load)

## Non-functional requirements

- AGENTS.md "Settings UI contract" continues to hold for all caption
  controls (label + one-line description; sliders smooth-drag and
  double-click-to-type).
- AGENTS.md "Verified means the live app, not `cargo check`" - every
  AC in this PRD is gated on either a live-app observation or a
  source-tree assertion; none is satisfied by `cargo check` alone.
- File-size cap: this PRD and `BLUEPRINT.md` stay under 800 lines.
