# Feature request: Caption designer orientation

## 1. Problem & Goals

The current caption designer renders a static creator-photo image as the preview backdrop (`src/components/settings/CaptionSettings.tsx:9` imports `captionPreviewFrame from "@/assets/caption-preview-frame.png"`, used at line 288 inside `CaptionPreviewPane`). This forces users to picture-match their caption pill against an unrelated still image and offers no signal about how the caption will sit in different output orientations / aspect ratios.

User feedback: *"Change the caption designer from the static image of me (the creator haha!) to be more of a configurable horizontal or vertical orientation designer, use this image (eval/fixtures/caption-mock-h-and-w.png) as an example of a design - we don't need to list their screen dimensions just the arrow boundary designs and center lines."*

The reference mock at `eval/fixtures/caption-mock-h-and-w.png` shows five framed device rectangles of mixed orientations with double-headed arrows and centerline arrows indicating boundary axes. Pixel dimensions in the mock are illustrative only and **must not** be displayed in the new designer.

**Goal:** replace the static photo preview with an orientation-aware mock frame (horizontal vs vertical, selectable) drawn with arrow boundaries and centerlines, so users can see exactly where their caption pill will land in either orientation. Caption layout authority must remain in the backend (single source of truth) so the redesign is purely a *visualization* change of an unchanged underlying contract.

## 2. Desired Outcome & Acceptance Criteria

- The caption preview pane shows an empty frame outline (rounded rectangle) with horizontal + vertical centerlines and double-headed boundary arrows on each axis. No background photo. No pixel dimensions written on the frame.
- An "Orientation" toggle / select switches the preview between Horizontal (16:9) and Vertical (9:16). The pill renders inside the chosen frame, scaled correctly.
- The same backend caption-layout contract drives both orientations - no orientation-specific layout fields in the React state. (Per AGENTS.md "Single source of truth for dual-path logic": the export pipeline's caption renderer must consume identical layout output regardless of preview orientation.)
- Existing caption settings (position %, font size, padding, transparency, etc.) keep working in both orientations.
- The static `caption-preview-frame.png` asset is removed (or replaced with a vector frame) so the dep-hygiene gate stays green.

(See `PRD.md` for the formalized AC list.)

## 3. Scope Boundaries

### In scope

- Replace the photo preview backdrop in `CaptionPreviewPane` with a vector frame (SVG or styled div) showing rounded-rectangle outline + horizontal centerline + vertical centerline + double-headed boundary arrows.
- Add an Orientation control (Horizontal / Vertical) inside `CaptionSettings.tsx`. Default = Horizontal (matches today).
- Recompute the preview pane's `aspectRatio` from the orientation choice (16/9 vs 9/16).
- Remove `src/assets/caption-preview-frame.png` (and its import) once the new frame ships.
- i18n keys: `settings.captions.preview.orientation.horizontal`, `.vertical`, label.

### Out of scope (explicit)

- Persisting the orientation choice as a setting (default to ephemeral preview state unless audit shows users want it sticky).
- Changing any caption layout setting (position, font, padding, color, etc.) - those keep their current keys and ranges.
- Updating the export pipeline's ASS rendering to produce orientation-specific styles. The backend contract is unchanged; export remains driven by user-supplied media aspect ratio.
- Multi-frame preview (showing multiple device sizes at once like the reference mock). The mock illustrates the *style* of the frames (arrows + centerlines, no labels); the designer surfaces ONE frame at a time toggled by orientation.

## 4. References to Existing Code

- `src/components/settings/CaptionSettings.tsx:9` - the import being replaced.
- `src/components/settings/CaptionSettings.tsx:174-179` - `VIRTUAL_FRAME_W` / `VIRTUAL_FRAME_H` / `VIRTUAL_FRAME_ASPECT` constants. Become orientation-derived.
- `src/components/settings/CaptionSettings.tsx:181-314` - `CaptionPreviewPane` component, target of redesign.
- `src/components/settings/CaptionSettings.tsx:286-296` - the `<img src={captionPreviewFrame}>` block to delete.
- `src/components/player/CaptionOverlay.tsx` - `<CaptionPill>` consumed by the preview; layout contract stays unchanged.
- `src-tauri/src/managers/captions/` - backend authority for caption font + layout (referenced at line 140 comment "CSS font stacks must mirror src-tauri/src/managers/captions/fonts.rs"). Must not regress.
- `src/assets/caption-preview-frame.png` - asset to delete after migration (run `dep-hygiene` to confirm no other importer).
- `eval/fixtures/caption-mock-h-and-w.png` - design reference (5 framed devices with arrow boundaries; do NOT replicate pixel labels).
- `AGENTS.md` "Single source of truth for dual-path logic" - the rule that forbids orientation-specific layout state in React.

## 5. Edge Cases & Constraints

- The `<CaptionPill>` currently scales from frame pixels to CSS pixels via `containerSize.h / VIRTUAL_FRAME_H` (line 220). This must keep working when the frame flips to vertical (9:16); the scale formula likely needs to switch to `containerSize.w / VIRTUAL_FRAME_W` in vertical mode, *or* the virtual frame swaps W/H. Pick whichever keeps the pill physically the same size on screen at 100% font setting in both orientations.
- The `position` setting is a percentage from the top of the frame (currently `bottomPx = ((100 - positionPct) / 100) * containerSize.h`). In vertical orientation the percent semantics must remain "% from the top of the frame" - users will be confused if the same numeric value moves the pill to a totally different visual location. Keep the formula identity; only the frame aspect changes.
- Existing tests / fixtures that reference the preview image must be updated (search `caption-preview-frame` across the repo before deleting the asset).
- Color / contrast: arrow + centerline strokes must use existing token colors (`#EEEEEE` rest, accent on hover) per AGENTS.md Settings UI contract. No invented greys.
- ASCII only in artifacts.

## 6. Data Model (optional)

No persisted-setting change. Orientation is preview-pane local state (React `useState<'horizontal'|'vertical'>('horizontal')`). If user feedback later asks for a remembered choice, add `caption_designer_orientation: "horizontal"|"vertical"` in a follow-up.

## Q&A

Resolved 2026-04-18 - **scope expands**:

- **Persistence & project import:** Caption settings are written to the Toaster project file. When a project is imported with different caption settings, respect the project's values (project overrides app defaults). Implementers: caption config is project-scoped state, not just an app-level setting.
- **Dual-view model (new scope):** The user wants to configure **both desktop and mobile views** up front. The editor surfaces a radio toggle (e.g. Desktop / Mobile) to switch between them while editing. On video import, the system inspects source orientation/aspect and intelligently selects the matching view as the active one - without losing the other view's configuration. This replaces the original "single orientation toggle" framing.
- **Reference mock (5 frames):** Still the visual vocabulary (axis arrows + centerlines, no pixel labels). Not a "compare all" mode - the radio toggle drives which view is shown at a time.
- **Arrows / centerlines visibility:** Always shown when designing a view; they are the feature, not decoration.

> **Blocker before promoting to `planned`:** PRD (`PRD.md`) and BLUEPRINT (`BLUEPRINT.md`) were written against the original single-toggle scope. They need a revision pass to cover: two stored caption profiles (desktop + mobile) per project, persistence into the project file format, auto-selection-on-import logic, and the editor radio toggle. Coverage.json + tasks.sql will grow. Do not start execution until this revision lands.
