# Blueprint: Caption designer orientation

## Architecture decisions

- **R-001** (vector frame): extract a new component `src/components/settings/captions/CaptionMockFrame.tsx` (props: `orientation`, optional class). Implementation = a single SVG sized to fill its container, drawing:
  - rounded-rect outline (stroke `#EEEEEE`, corner radius proportional to the short side)
  - horizontal centerline + vertical centerline (dashed, low-opacity stroke)
  - 4 axis arrows (double-headed, end caps) along the outer edges
  No `<text>` elements (no pixel labels). The `<CaptionPill>` remains a sibling positioned absolutely over the frame.
- **R-002** (orientation toggle): add a `<Select>` (matching the existing sample-text picker pattern at `CaptionSettings.tsx:247-275`) labelled `settings.captions.preview.orientation.label` with two options. Local `useState<'horizontal'|'vertical'>('horizontal')`. Recompute `aspectRatio` and the scale formula from this state.
- **R-003** (SSOT): the orientation value never crosses the Tauri command boundary. Backend caption layout (`src-tauri/src/managers/captions/`) stays untouched. This is enforced by NOT adding any new `change_caption_*_setting` Tauri command in this PR.
- **R-004** (cleanup + i18n):
  - delete `src/assets/caption-preview-frame.png` and the line-9 import after R-001 is wired and live-launched.
  - add the three new i18n keys via the `i18n-pruning` skill across all 22 locales.

## Component & module touch-list

- Add: `src/components/settings/captions/CaptionMockFrame.tsx` (~80-120 lines).
- Edit: `src/components/settings/CaptionSettings.tsx` - delete `captionPreviewFrame` import (line 9), delete the `<img>` block (lines 286-296), add `useState` for orientation, add `<Select>` for orientation, swap the aspect-ratio constant for an orientation-derived value, swap the scale formula to use the short axis.
- Delete: `src/assets/caption-preview-frame.png`.
- Edit: `src/i18n/locales/*/translation.json` (3 keys * 22 locales).
- Update: `features/caption-designer-orientation/journal.md`.

## Single-source-of-truth placement

This is the central architectural choice for this feature. Per AGENTS.md "Single source of truth for dual-path logic":

- **Preview** orientation is React local state, derived from a UI control. It governs only the preview pane's layout box (16:9 vs 9:16).
- **Export** orientation is determined by the user's source media aspect ratio, read in the export pipeline. It is **not** a setting and **not** plumbed from the React preview.
- The shared layout struct (caption position %, font size, padding, color, etc.) is the single source of truth and is the same struct consumed by both preview (`<CaptionPill>`) and export (`src-tauri/src/managers/captions/`).

If at any point this PR finds itself adding orientation as a Tauri command parameter or persisted setting, the architectural decision is wrong - stop and re-discuss.

## Data flow

```
preview path:
  CaptionSettings.tsx (orientation = "horizontal" | "vertical", local state)
    -> CaptionPreviewPane: aspectRatio + scale derived from orientation
    -> CaptionMockFrame: draws frame outline + arrows + centerlines
    -> CaptionPill: positioned over the frame using existing layout struct

export path (UNCHANGED):
  user clicks Export -> backend reads source media aspect ratio
    -> backend reads existing caption layout settings (position %, font, etc.)
    -> ASS / FFmpeg renders captions at native source aspect
```

The two paths share the layout struct and nothing else - exactly as today.

## Migration / compatibility

- No setting migration required.
- Existing caption settings keep their values and meanings.
- Users who had been picture-matching against the photo will notice the photo is gone; the new frame's centerlines + max-width arrow give them more useful alignment cues.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Pill scale visually inconsistent across orientations | Scale formula uses short axis in both modes; live-launch comparison | AC-002-b |
| Orientation accidentally plumbed to backend | BLUEPRINT explicitly forbids; code review check; cargo test stays green without changes | AC-003-a, AC-003-b |
| Asset deleted while still imported elsewhere | `rg "caption-preview-frame"` before deletion; AC-004-a checks zero matches | AC-004-a |
| i18n keys missing in some locales | `i18n-pruning` skill + CI gate | AC-004-b |
| `CaptionSettings.tsx` exceeds 800-line file cap | Extract `CaptionMockFrame` to its own file | n/a (process gate) |
| Pill goes off-frame in vertical at high font-size + max-width settings | Live-launch test pinning extreme values; clamp existing `caption_max_width_percent` semantics work in both | AC-002-c |

## Implementation order suggestion

1. R-001 (build vector frame, render side-by-side with photo behind a flag if helpful).
2. R-002 (add orientation toggle, wire aspect ratio + scale formula).
3. R-004 i18n keys (parallel with R-002).
4. Live-launch verification across both orientations + extreme settings.
5. R-003 audit (verify no new Tauri command, run `cargo test caption_layout`).
6. R-004 cleanup (delete the static asset, confirm `rg` is empty).
