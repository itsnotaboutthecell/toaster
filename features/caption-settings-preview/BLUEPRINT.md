# Blueprint: Caption Settings Preview

## Architecture decisions

- **R-001 (static preview)**: render a static background frame plus a
  caption pill. No `navigator.mediaDevices.getUserMedia` is called.
  Camera-based preview is documented as a follow-up
  (provisional slug `caption-settings-camera-preview`).
- **R-002 (placement & geometry)**: a `CaptionPreviewPane` is the
  first child of the caption section in
  `src/components/settings/CaptionSettings.tsx`. It uses
  `position: sticky; top: 0` within the section and an
  `aspect-ratio: 16/9` container so contain-fit works without
  intrinsic-size math. Pattern reference:
  `src/components/settings/CaptionSettings.tsx:138` (existing
  `React.memo`-wrapped section root).
- **R-003 (text + background)**: three local string constants
  (short / two-line / long) plus a `Select` driven by `useState`.
  Background image is `eval/fixtures/toaster_example.mp4` first
  frame, decoded once at build time and bundled as a PNG asset
  imported by the preview component. Fallback: a `<div>` with
  `background-color: #1a1a1a`. Reuse `Select`
  (already imported in `src/components/settings/CaptionSettings.tsx:6`)
  and `useTranslation` for the dropdown legend and labels (line 2).
- **R-004 (renderer reuse)**: extract the caption-pill JSX from
  `src/components/player/CaptionOverlay.tsx` (around the
  `fittedVideoRect` consumer) into a new named export
  `CaptionPill` *within the same file* (no new file under
  `components/`). `CaptionPreviewPane` imports `CaptionPill`
  directly. The player continues to use `CaptionOverlay` and
  internally renders `CaptionPill`. Pattern reference:
  `src/components/player/CaptionOverlay.tsx:35-50` (existing
  `rgbaToCss` and `fittedVideoRect` helpers stay co-located with
  the pill).
- **R-005 (latency)**: the preview consumes settings via the same
  hook the controls use (`useSettings` per
  `src/components/settings/CaptionSettings.tsx:3`). React's natural
  render cycle satisfies the 16ms budget; no debounce or scheduling
  wrapper is introduced.

## Component & module touch-list

| File | Change |
|------|--------|
| `src/components/settings/CaptionSettings.tsx` | Add `CaptionPreviewPane` as first child; import `CaptionPill`; add `selectedSampleKey` state; add `Select` for sample text. |
| `src/components/player/CaptionOverlay.tsx` | Extract inner pill JSX into a new named export `CaptionPill`. `CaptionOverlay` continues to default-export and now renders `<CaptionPill ... />` internally. No behavior change for the player. |
| `src/i18n/locales/*/translation.json` (20 files) | Add keys: `settings.captions.preview.heading`, `settings.captions.preview.sample.short`, `.twoLine`, `.long`, `settings.captions.preview.sampleLegend`. The literal English `short` value is "looking crispy" per AC-003-a. Use the `i18n-pruning` skill to keep all 20 locales in sync. |
| `src-tauri/...` | **No changes.** Backend caption authority is read-only here. |
| `eval/fixtures/` | **No new assets.** Reuse `toaster_example.mp4` first frame. A pre-decoded PNG sibling (e.g. `toaster_example.first-frame.png`) may be added if needed - decided during execution; if added, gated by the `dep-hygiene`/asset-review reviewer. |

## Single-source-of-truth placement

- **Preview/export caption rendering**: backend authority remains
  `src-tauri/src/managers/captions/ass.rs` (export). Frontend
  authority remains `src/components/player/CaptionOverlay.tsx`
  (preview). The settings preview is a *third consumer* of the
  frontend authority, achieved by extracting `CaptionPill` from
  `CaptionOverlay.tsx` rather than re-implementing it. This keeps
  the dual-path-logic rule intact: there are still only two
  renderers (export and preview), and the settings preview shares
  the preview one verbatim.
- **Settings store**: the existing `useSettings` hook is the only
  source. The preview reads through it; no parallel store, no
  cached snapshot, no derived selector that reshapes caption
  fields.

## Data flow

```
useSettings (Zustand)
   |
   |-- consumed by CaptionSettings controls (slider/select)
   |       \-- onChange -> store update
   |
   |-- consumed by CaptionPreviewPane (same render cycle)
   |       \-- props passed to <CaptionPill ... />
   |
   |-- consumed by player CaptionOverlay (live timeline)

CaptionPreviewPane
   |
   |-- background <img src=fixture-frame onLoad=setBackgroundReady(true)>
   |       fallback when error: <div bg=#1a1a1a>
   |
   \-- <CaptionPill text={samples[selectedSampleKey]} settings={...} />
```

## Migration / compatibility

- No persistent state, no schema, no Tauri command surface change.
- Adds 5 i18n keys; `scripts/check-translations.ts` will require
  parallel updates across all 20 locale files (use `i18n-pruning`
  skill). Until those keys land in every locale, the gate fails.
- Extracting `CaptionPill` from `CaptionOverlay` is mechanical and
  must preserve byte-identical render output for the player. A
  before/after live-app comparison on
  `eval/fixtures/toaster_example.mp4` is the regression check
  (covered by the player-regression QC step).

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Engineer hand-rolls a CSS span instead of reusing `CaptionPill` (recreates the dual-renderer defect class) | Blueprint mandates extraction; AC-004-a/b are source-tree assertions, not just behavior checks | AC-004-a, AC-004-b |
| Debounce sneaks in "to smooth slider drag" | AC-005-b is a literal source-tree assertion (no `debounce`/`throttle`/`setTimeout` between store and pill) | AC-005-b |
| Background fixture removed/renamed in a future cleanup; preview pane goes blank | Fallback to `#1a1a1a` is part of the spec, not an afterthought | AC-003-c |
| Sticky preview overlaps section heading on small windows | `top` offset accounts for heading height; QC step resizes to a known small width | AC-002-b |
| Localized "long" sample overflows the pane and breaks layout in some locales | `overflow: hidden` on the pane; QC step inspects `de` and `ja` translations | AC-003-b (long-wrap behavior) |
| Extraction subtly changes player render output | Live-app A/B on `toaster_example.mp4` before merging the extraction commit | AC-001-a (preview matches expected behavior) plus manual player-regression step |
| Camera follow-up gets reopened mid-implementation, scope creeps | R-001 and PRD scope explicitly defer it; reviewers reject inline scope changes | AC-001-b |
