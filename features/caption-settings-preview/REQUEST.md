# Feature request: Caption Settings Preview

## 1. Problem & Goals

Quoted user request:

> For the caption configuration under Advanced, users would like to see a
> mock of the caption. Consider our options it could be a static preview
> window dimensions with the caption "looking crispy" or we could turn
> on their system camera if they allowed and show a live preview of them
> with the caption - as they change settings it should display the
> adjustments within the preview.

Today, every knob in `src/components/settings/CaptionSettings.tsx`
(font size, padding, opacity, radius, color, family) is changed
sightless: the user must close settings, scrub the timeline to a
captioned region, and visually compare. This is a friction wall every
time someone tunes captions, and it discourages experimentation with
the very controls Advanced exposes.

Goal: render a faithful caption pill inside the settings panel that
updates in real time as controls are dragged, so users can see the
effect of every change without leaving Advanced.

## 2. Desired Outcome & Acceptance Criteria

- A preview pane sits at the top of the caption section, shows a
  bundled background image at 16:9 contain-fit, and renders a caption
  pill on top using the same renderer the live player uses.
- Dragging any caption control updates the preview within one render
  frame (<= 16ms). No debounce.
- A small dropdown lets the user switch between three sample texts
  (short / two-line / long) so they can see line-wrap behavior.
- No new caption renderer is introduced. The preview pill is the same
  React component the player uses.
- Camera-based live preview is out of scope for this feature and is
  documented as a follow-up.

## 3. Scope Boundaries

### In scope

- New `CaptionPreviewPane` UI inside `CaptionSettings.tsx`.
- Reuse of `CaptionOverlay` (or its inner pill component) over a
  fixed background frame.
- A small placeholder-text dropdown (3 fixed strings).
- Wiring the preview to the same Zustand settings store the live
  player consumes, so changes propagate without ceremony.
- Fallback to a flat dark-grey background when the bundled frame is
  unavailable at runtime.

### Out of scope (explicit)

- Camera / `getUserMedia` based live preview. Tracked as a follow-up
  feature (provisional slug `caption-settings-camera-preview`).
- Any change to caption rendering math itself (font_size, padding,
  bg_opacity, radius, color, family logic).
- Any change to the export-side renderer
  (`src-tauri/src/managers/captions/ass.rs`).
- Adding new fixture assets. Reuse what `eval/fixtures/` already ships.
- New translation keys beyond the few strings this feature exposes.

## 4. References to Existing Code

- `src/components/settings/CaptionSettings.tsx:1-138` - settings panel
  pattern to extend; `SliderWithInput` already meets the AGENTS.md
  "Settings UI contract" (smooth drag + double-click typing).
- `src/components/player/CaptionOverlay.tsx:1-50` and
  `fittedVideoRect` helper - the canonical caption pill renderer the
  preview must reuse.
- `src-tauri/src/settings/defaults.rs` - caption defaults
  (`font_size=40`, `padding_x`, `padding_y`, `bg_opacity`,
  `radius_px=0`).
- `src-tauri/src/managers/captions/ass.rs` - export-side renderer.
  Authority for box geometry (`BorderStyle=3`,
  `Outline=max(padding_x, padding_y)`). MUST NOT be touched by this
  feature; preview already matches it after the recent scaling fix.
- `eval/fixtures/toaster_example.mp4` - the bundled fixture; the
  preview uses its first frame as the background.
- `AGENTS.md` "Single source of truth for dual-path logic",
  "Settings UI contract", and "Verified means the live app, not
  `cargo check`" - the binding rules for this feature.

## 5. Edge Cases & Constraints

- The preview must consume the exact same caption-rendering code as
  the player. No third renderer. Violating this is the same defect
  class as the original preview-vs-export caption mismatch.
- Latency budget: <= 16ms (one render frame at 60fps) from a settings
  store update to a visible preview update. No `setTimeout` debounce,
  no lodash `debounce`.
- Background fixture might be missing in dev or in a stripped build.
  The preview must fall back to a flat `#1a1a1a` background and still
  render the pill.
- The preview must respect the panel's responsive width (caption
  section can be narrow on small windows). 16:9 contain-fit at the
  current panel width.
- ASCII-only changes (project rule).
- No hosted inference, no network calls.

## 6. Data Model (optional)

No persistent data. Two pieces of ephemeral component state:

- `selectedSampleKey: 'short' | 'twoLine' | 'long'`
- `backgroundReady: boolean` (false -> fallback grey)

The fixture filename and the three sample strings are local constants
in the preview component.

## Q&A

The user supplied directional answers in the planning request itself.
Captured here verbatim so future readers do not need to back-trace.

- Q: Static frame vs camera live preview?
  - A: Static frame. Camera is a follow-up. Rationale: lower
    complexity, no permissions UX, no privacy implications,
    immediate value.
- Q: Where in the panel does the preview live?
  - A: Above the controls, sticky on scroll, 16:9 contain-fit at the
    panel width. So users always see the effect of the slider they
    are currently dragging without scrolling.
- Q: Placeholder text and background?
  - A: Default text "looking crispy" (user's wording). Dropdown of
    three sample strings (short / two-line / long). Background:
    first frame of `eval/fixtures/toaster_example.mp4`. Flat
    `#1a1a1a` if fixture is unavailable.
- Q: Reuse strategy?
  - A: Import `CaptionOverlay` (or its inner pill) directly. The
    settings preview must NOT introduce a third caption renderer.
- Q: Live update latency?
  - A: <= 16ms. Measured by manual drag in the live app and by
    asserting no debounce wrapper exists in the wiring.
