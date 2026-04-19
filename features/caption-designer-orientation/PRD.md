# PRD: Caption designer orientation

## Problem & Goals

Replace the static creator-photo backdrop in the caption designer (`src/components/settings/CaptionSettings.tsx:9, 286-296`) with a vector orientation-aware mock frame matching the design reference at `eval/fixtures/caption-mock-h-and-w.png` (rounded rectangle + double-headed boundary arrows + centerlines, **no pixel labels**). Add a Horizontal / Vertical orientation toggle. See `REQUEST.md` for the full rationale.

## Scope

### In scope

- Vector mock frame replacing the photo backdrop.
- Orientation toggle (Horizontal default, Vertical alternative).
- Aspect-ratio swap (`16/9` <-> `9/16`) for the preview pane.
- i18n keys for the new toggle labels.
- Removal of the old static asset and its import.

### Out of scope (explicit)

- Persisting the orientation choice.
- Changes to caption layout settings or backend contract.
- Multi-frame side-by-side preview.
- Hide/show toggle for the arrow / centerline overlays.

## Requirements

### R-001 - Vector mock frame replaces photo backdrop

- Description: replace the `<img src={captionPreviewFrame}>` element in `CaptionPreviewPane` with a vector frame (SVG or styled divs) drawing a rounded rectangle outline, a horizontal centerline, a vertical centerline, and double-headed boundary arrows on each axis. No pixel labels rendered.
- Rationale: the static photo conveys nothing about the user's actual output frame and is the source of the user's regression complaint.
- Acceptance Criteria
  - AC-001-a - Live launch: opening Settings -> Captions shows a frame outline with the four-element overlay (rectangle + 2 centerlines + axis arrows). No human face / photo visible.
  - AC-001-b - The frame uses existing color tokens (`#EEEEEE` rest, `#E8A838` accent / hover) per AGENTS.md Settings UI contract; no invented greys / reds.
  - AC-001-c - No text labels on the arrows (no "1920x1080", no pixel counts).

### R-002 - Orientation toggle

- Description: add an "Orientation" select (or segmented control) above or beside the preview pane with options `Horizontal` and `Vertical`. The pane's aspect ratio recomputes from the choice (`16/9` for horizontal, `9/16` for vertical).
- Rationale: per the user's request, the designer must demonstrate caption placement in either orientation.
- Acceptance Criteria
  - AC-002-a - Live launch: toggling Orientation flips the preview pane between landscape (16:9) and portrait (9:16) without reloading the page.
  - AC-002-b - The `<CaptionPill>` renders correctly in both orientations: positioned by the same `position %` setting, scaled so its visual size at 100% font is comparable across orientations.
  - AC-002-c - The pill respects the existing `caption_max_width_percent` setting in both orientations (no overflow off the frame in either).
  - AC-002-d - Default orientation on first load = Horizontal.

### R-003 - Single source of truth preserved

- Description: orientation is preview-only state. The backend caption-layout contract (`src-tauri/src/managers/captions/`) must not gain orientation-specific fields. The export pipeline must produce identical caption ASS output regardless of which orientation was used in the preview.
- Rationale: AGENTS.md "Single source of truth for dual-path logic" - the preview-vs-export drift bug recurred when this rule was relaxed for caption layout previously.
- Acceptance Criteria
  - AC-003-a - `cd src-tauri; cargo test caption_layout` exits 0 (the existing caption-layout tests stay green; no new orientation field added to the layout struct).
  - AC-003-b - Code review confirms no new orientation parameter is plumbed across the Tauri command boundary; the React preview computes orientation locally from a `useState` only.

### R-004 - Asset cleanup + i18n

- Description: delete `src/assets/caption-preview-frame.png` and its import once the vector frame ships; add i18n keys for the orientation toggle in all 22 locales.
- Rationale: AGENTS.md `dep-hygiene` (orphan assets are dead weight); `i18n-pruning` (every user-visible key must propagate to all locales).
- Acceptance Criteria
  - AC-004-a - `rg "caption-preview-frame" src` returns zero matches after the change (asset and import both gone).
  - AC-004-b - `bun run scripts/check-translations.ts` exits 0; the new keys (`settings.captions.preview.orientation.label`, `.horizontal`, `.vertical`) exist in every `src/i18n/locales/*/translation.json`.

## Edge cases & constraints

- The `position` setting semantics ("% from top of frame") must hold in both orientations. The numeric value moves the pill the same proportional distance regardless of orientation.
- The pill's CSS font size at user-set 100% must look comparable in both orientations - not 1.78x larger in vertical because of the W/H swap. The scale formula in `CaptionPreviewPane` (`containerSize.h / VIRTUAL_FRAME_H` at line 220) likely needs to use the *short* axis as the scale denominator in both modes.
- ASCII only in artifacts.
- Settings UI contract: orientation control needs a label + one-line description.

## Data model

No persisted change. Local React state only.

## Non-functional requirements

- File-size cap: `CaptionSettings.tsx` is currently 522 lines; this change should not push it past 800. If the vector frame component is non-trivial, extract it to `src/components/settings/captions/CaptionMockFrame.tsx`.
- No new runtime network calls.
