# Blueprint: time stretch segments

## Architecture decisions

- **R-001 (data model):** persist `segment_stretches: Vec<SegmentStretch>`
  on `ProjectSettings` with `#[serde(default)]`, mirroring the
  `caption_profiles` precedent at `src-tauri/src/managers/project.rs:45-46`.
  Rejected alternative: adding `stretch: f32` to `Word` — stretch is
  segment-scoped, not word-scoped, and `Word` already serialises on
  every project save. Anchor stretch entries by
  `(anchor_start_us, anchor_end_us)` captured from the derived
  keep-segment at assign time; on load, resolve each anchor to the
  best-matching derived segment by overlap. Unresolved anchors
  (segment boundary moved past the anchor) collapse to `stretch=1.0`
  and the stale entry is dropped on next save.
- **R-002 (plumbing):** promote internal segment representation from
  `Vec<(i64, i64)>` to
  `Vec<CanonicalKeepSegment { start_us, end_us, stretch }>` in
  `src-tauri/src/commands/waveform/mod.rs`. Tuple sprawl is already a
  maintainability smell across this file
  (`build_audio_concat_filter_with_fade(segments: &[(i64, i64)])`,
  `build_audio_segment_filter(i, n, start_us, end_us, seam_fade_us)`);
  the struct lets stretch ride alongside without adding parallel
  `Vec<f32>` arguments. Existing call-sites update mechanically.
- **R-002 (audio graph):** extend `build_audio_segment_filter`
  (`commands/waveform/mod.rs:170-205`) to append `,atempo={stretch:.6}`
  after `asetpts=PTS-STARTPTS` and before the fade chain. Rationale:
  `atempo` after `atrim` + `asetpts` is the FFmpeg idiom and keeps
  fade math in the stretched timebase where the seam actually lives.
  For `stretch == 1.0` the filter is omitted so the emitted graph
  is byte-identical to today (regression-safe).
- **R-002 (video graph):** in the export video branch
  (`commands/waveform/mod.rs:650-694`) append
  `,setpts=(PTS-STARTPTS)/{stretch:.6}` after the existing
  `setpts=PTS-STARTPTS` so video PTS matches the stretched audio.
  For `stretch == 1.0` the factor is omitted.
- **R-002 (seam fades):** fade duration stays authored in source
  microseconds (`seam_fade_duration_seconds`,
  `commands/waveform/mod.rs:161-168`). The rendered fade naturally
  scales by `1/stretch` because it runs *after* `atempo`. Document
  this as the chosen policy in the doc-comment adjacent to
  `build_audio_segment_filter`; do not re-scale the fade argument.
- **R-003 (time mapping):** update
  `map_edit_time_to_source_time_from_segments`
  (`commands/waveform/mod.rs:393-405`) and `map_source_to_edit`
  (`managers/captions/layout.rs:437-455`) to consume stretch via a
  single helper `edit_duration_of(segment)` so both walkers share
  the same arithmetic. Keep them pure so the existing precision
  tests (`tests/dual_track_regression.rs`,
  `precision_benchmarks.rs:518+`) continue to cover the identity
  path.
- **R-004 (UI):** add the numeric input + slider + reset to the
  existing segment context menu component (follow the pattern of the
  current filler-action menu in `src/components/editor/`); surface
  stretch through a new Zustand selector fed by the store's copy of
  `segment_stretches`. State mutation round-trips through a new Tauri
  command `set_segment_stretch(anchor, value)`.
- **R-005 (backward compat):** on `ToasterProject` load, if
  `segment_stretches` is absent (`#[serde(default)]` gives empty
  `Vec`), derived keep-segments expose `stretch=1.0`. Do not bump
  `PROJECT_VERSION` eagerly; only the first save that contains a
  non-default stretch writes the new field. Unit test loads a
  fixture project from `v1.1.0` and asserts `stretch == 1.0` on every
  derived segment.

## Component & module touch-list

Rust backend:

- `src-tauri/src/managers/project.rs` — add `SegmentStretch` struct
  and `ProjectSettings.segment_stretches` with `#[serde(default)]`.
  Add a clamping setter with a unit test for R-001-b.
- `src-tauri/src/managers/editor/mod.rs` — store
  `segment_stretches` on `EditorState`, expose a
  `get_stretch_for_segment(anchor_start, anchor_end) -> f32` accessor
  used by `canonical_keep_segments_for_media`. Hydrate from project
  on load.
- `src-tauri/src/managers/editor/types.rs` — add `stretch: f32` to
  `TimingSegment` (and propagate through `TimingContractSnapshot`).
- `src-tauri/src/commands/waveform/mod.rs` — introduce
  `CanonicalKeepSegment`; thread through
  `canonical_keep_segments_for_media`,
  `canonical_keep_segments_for_media_with_options`,
  `select_raw_keep_segments_for_media`,
  `snap_segments_against_media`,
  `build_audio_segment_filter`, `build_audio_concat_filter_with_fade`,
  `map_edit_time_to_source_time_from_segments`, and the export
  video/audio graph at lines 641-694. Extend public `KeepSegment`
  specta type with `stretch: f32` (default 1.0).
- `src-tauri/src/commands/waveform/commands.rs` — new
  `set_segment_stretch` command; update `edit_version_token`
  (`~line 245`) to incorporate per-segment stretch factors.
- `src-tauri/src/managers/captions/layout.rs` — update
  `map_source_to_edit` to consume stretch.
- `src-tauri/src/lib.rs` — register the new command in the specta
  invoke handler (near line 291 where `map_edit_to_source_time` is
  already registered).

Frontend:

- `src/bindings.ts` — regenerate via specta (do not hand-edit).
- `src/stores/editorStore.ts` — new selectors for segment stretch;
  wire the new IPC command.
- `src/components/player/MediaPlayer.tsx` — when any segment has
  `stretch != 1.0`, resolve edit<->source conversions via the
  backend `commands.mapEditToSourceTime` IPC rather than the local
  `editTimeToSourceTime` helper. Drive `videoRef.playbackRate` from
  the stretch factor of the segment under the cursor.
- `src/components/editor/` — extend the segment context menu with
  the numeric input + slider + reset; gate commits through the new
  `set_segment_stretch` command.
- `src/i18n/locales/*/translation.json` — add strings for the new
  controls; mirror across all 20 locales (enforced by
  `scripts/check-translations.ts`).

Tests & evals:

- `src-tauri/src/managers/project.rs` unit tests — add a v1.1.0
  fixture load test for AC-004-a.
- `src-tauri/src/commands/waveform/tests/` — add stretch parity
  tests comparing preview and export segment durations sample-
  accurately for AC-001-c and AC-002-a.
- `.github/skills/transcript-precision-eval/` — extend fixture set
  with a stretched-segment case for AC-002-b.
- `.github/skills/audio-boundary-eval/` — extend seam checks to
  include stretched-segment seams.

## Single-source-of-truth placement

| Concern | Backend authority | Frontend consumer |
|---------|-------------------|-------------------|
| Stretch value for a segment | `EditorState::get_stretch_for_segment` (hydrated from `ProjectSettings.segment_stretches`) | `useEditorStore` selector; context-menu reads/writes via `set_segment_stretch` IPC |
| Audio graph `atempo` placement | `build_audio_segment_filter` | none (backend emits filter_complex; preview and export inherit) |
| Video PTS stretch | Export video graph in `commands/waveform/mod.rs:650-694` | none for export; preview uses `video.playbackRate` fed from the *same* stretch value via IPC |
| Edit -> source time | `map_edit_time_to_source_time_from_segments` via `commands::waveform::map_edit_to_source_time` | `MediaPlayer.tsx` calls the IPC when any stretch != 1.0; the local TS helper remains for the identity case only |
| Source -> edit time (captions) | `managers::captions::layout::map_source_to_edit` | n/a — caption layout is backend-only |
| Preview cache invalidation | `edit_version_token` folds stretch factors in | cache consumer in `MediaPlayer.tsx` just trusts the token |

## Data flow

1. User right-clicks a keep-segment. Context menu reads stretch via
   `useEditorStore` (store hydrated from backend on project load).
2. User edits the numeric input or slider; component debounces and
   invokes `commands.setSegmentStretch(anchor_start_us,
   anchor_end_us, value)`. Command clamps to `[0.5, 2.0]`, updates
   `EditorState.segment_stretches`, bumps `timeline_revision`, and
   returns the canonical value.
3. Frontend triggers preview re-render. Backend recomputes
   `canonical_keep_segments_for_media`, `edit_version_token`
   (including stretch factors), and if the token changes renders a
   fresh preview with `atempo` baked in.
4. Playback: edit-time cursor converts to source-time via backend
   IPC; `MediaPlayer.tsx` sets `videoRef.playbackRate` to the
   stretch of the current segment and starts the preview audio in
   lockstep. On seam crossing, it re-reads the stretch for the new
   segment and updates `playbackRate`.
5. Export: `run_export` builds filter_complex from the same
   `canonical_keep_segments_for_media`, emitting `atrim + atempo`
   for audio and `trim + setpts/stretch` for video per segment.
6. Save: `ToasterProject.save` serialises
   `segment_stretches`. A non-default stretch triggers the first
   write of the new field; legacy files with all-default stretch
   omit it (empty `Vec` is the `serde(default)` value).

## Migration / compatibility

- `PROJECT_VERSION` stays at `1.1.0` until the blueprint is
  implemented; at that point the version bumps to `1.2.0` and the
  doc-comment at `managers/project.rs:12-17` is extended to describe
  the `segment_stretches` field with the same "loads cleanly via
  `#[serde(default)]`" note.
- Legacy (`1.0.0`, `1.1.0`) projects load cleanly because
  `serde_json` applies the default when the field is absent. AC-004-a
  guards this path.
- Stretch anchors are best-effort: if a user deletes words such that
  an anchored segment no longer exists, the orphaned entry is
  dropped on next save and logged at `info` level (mirrors how
  `caption_profiles` handles orphaned profile data).

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| `atempo` alters seam fade duration in unexpected direction, reintroducing audible clicks | Policy doc + audio-boundary-eval seam window fixtures with stretched segments | AC-002-a |
| Export video PTS stretch (`setpts`) desyncs against stretched audio | Side-by-side preview/export duration assertion within 1 sample | AC-001-c, AC-002-a |
| Preview cache serves stale audio after a stretch edit | Fold stretch factors into `edit_version_token`; covered by a cache-bust integration test | AC-002-a |
| Frontend TS `editTimeToSourceTime` keeps silently running on non-identity stretches, creating a dual-path divergence | Route through backend IPC when any stretch != 1.0; add a contract test that asserts the helper is bypassed when stretch is present | AC-002-b |
| 800-line cap breached by `commands/waveform/mod.rs` | If the added code exceeds the cap, split the filter-graph helpers into a sibling module `waveform/filtergraph.rs`; planned as part of `ts-waveform-split` task | n/a (file-size gate) |
| Caption layout rounding drifts > 1 frame over long stretched runs | Use integer-microsecond math throughout; add a precision-eval fixture with a 10+ minute stretched passage | AC-002-b |
| User enters out-of-range stretch via context menu | Clamp at setter (R-001-b); frontend slider bounds match backend; numeric input validated on blur | AC-001-b |
| Legacy `.toaster` files fail to load when serde encounters unknown field shape during later evolution | Use `#[serde(default)]` + `deny_unknown_fields = false` on `ProjectSettings` (current behavior); cover with fixture load test | AC-004-a |
| `<video>` playbackRate change at seam introduces a perceptible click | Preview audio is pre-rendered with `atempo` baked in; video rate change is silent. Document & verify manually | AC-003-a manual check |

