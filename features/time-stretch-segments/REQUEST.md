# Feature request: time stretch segments

## 1. Problem & Goals

Per-keep-segment `atempo` time-stretch so users can speed up or slow
down individual segments of an edit without changing pitch. Typical
use-cases:

- Speed up filler-light sections to match the pace of punchy ones.
- Slow down technical explanations for clarity.

Today every kept segment plays back at 1.0x. Users that want uneven
pacing must cut and re-record, which defeats the transcript-first
edit loop. Source: Milestone 2 of `features/product-map-v1/PRD.md`
§6.

## 2. Desired Outcome & Acceptance Criteria

- Stretch factor is per-segment, default 1.0 (no stretch).
- Range clamped to `[0.5, 2.0]` — single-pass `atempo` bounds; chained
  `atempo` (for <0.5 or >2.0) is out of scope.
- Preview and export apply the same stretch. One backend
  implementation; two consumers (per AGENTS.md dual-path rule).
- Caption boundaries through stretched regions stay aligned with
  their words in the exported video.
- UI: numeric input + slider in the segment context menu; reset
  button restores 1.0.
- Backward-compat: existing `.toaster` files load with stretch=1.0 on
  every segment via `#[serde(default)]`.

## 3. Scope Boundaries

### In scope

- New persisted per-segment `stretch: f32` user intent.
- Backend plumbing: stretch-aware canonical keep-segments, time
  maps, preview audio renderer, export audio + video graph.
- Caption layout `map_source_to_edit` update to consume stretch.
- Segment context-menu UI control (numeric input + slider + reset).
- i18n parity for new user-visible strings.
- Fixture-based evals extended to cover stretched segments
  (`transcript-precision-eval`, `audio-boundary-eval`).

### Out of scope (explicit)

- Chained `atempo` for stretch factors outside `[0.5, 2.0]`.
- Keyboard shortcuts for stretch adjustment (tracked in
  `keyboard-shortcuts-cheatsheet`).
- Chapter-markers downstream — independent bundle; will consume the
  stretch-aware time map when shipped.
- Automatic stretch suggestions / speed leveling.
- Pitch-preserving algorithms beyond FFmpeg `atempo`'s defaults.
- Changing the `<video>` element source swap policy (forbidden by
  AGENTS.md).

## 4. References to Existing Code

- `src-tauri/src/commands/waveform/mod.rs:131-136` — public
  `KeepSegment { start_us, end_us }` specta type. Surface for the
  new `stretch` field on the IPC boundary.
- `src-tauri/src/commands/waveform/mod.rs:331-391` —
  `canonical_keep_segments_for_media`, the sole backend authority
  consumed by preview, export, and the edit<->source time map.
- `src-tauri/src/commands/waveform/mod.rs:170-220` —
  `build_audio_segment_filter` + `build_audio_concat_filter`; shared
  helpers where `atempo` must be inserted once.
- `src-tauri/src/commands/waveform/mod.rs:641-694` — export video
  graph (`trim` + `setpts`) that also needs stretching.
- `src-tauri/src/commands/waveform/mod.rs:393-405` —
  `map_edit_time_to_source_time_from_segments`.
- `src-tauri/src/managers/captions/layout.rs:435-455` —
  `map_source_to_edit`, the caption layout's time mapper.
- `src-tauri/src/managers/editor/mod.rs:299-305` —
  `EditorState::get_keep_segments`; derivation of segments from
  `words`.
- `src-tauri/src/managers/project.rs:17-46` — `ToasterProject` and
  `ProjectSettings`; see `caption_profiles` for the
  `#[serde(default)]` backward-compat pattern to follow.
- `src/bindings.ts:1203` — generated `KeepSegment` type; extend via
  specta regeneration, not by hand (AGENTS.md forbids bindings
  hand-edits beyond one-line union patches).
- `src/components/player/MediaPlayer.tsx:379-492` — frontend
  consumer that currently walks segments locally; must defer to
  backend `map_edit_to_source_time` IPC when any segment is
  stretched.

## 5. Edge Cases & Constraints

- Boundaries are half-open `[0.5, 2.0]`; values outside rejected or
  clamped at the write boundary. Invariant documented near the
  setter.
- Zero- and negative-width segments (`end_us <= start_us`) must not
  reach `atempo` (undefined behavior in FFmpeg). Existing guards in
  `canonical_keep_segments_for_media` already reject these; the
  stretch path must not re-admit them.
- Fade durations (`seam_fade_duration_seconds`,
  `commands/waveform/mod.rs:161-168`) measure source microseconds;
  with stretch applied the *rendered* fade duration shrinks/grows.
  Behavior must be deterministic and documented — pick one policy
  (fade specified in source time; blueprint chooses).
- `total_duration_s` (post-filter volume/fade) must be computed on
  the stretched timeline, not the raw source span.
- Preview cache key (`edit_version_token`,
  `commands/waveform/commands.rs:245`) must include stretch factors;
  otherwise stale cached previews will play after a stretch edit.
- Frontend `editTimeToSourceTime` helper in
  `MediaPlayer.tsx` is piecewise-linear and ignores stretch. Either
  extend it with stretch or route all conversions through the
  backend IPC. Dual-path duplication is forbidden.
- `<video>` element `playbackRate` must track the stretch factor of
  the segment currently under the edit-time cursor so lip-sync
  holds; audio is already baked. Transitions between segments with
  different stretch factors require a rate change; document seam
  smoothing policy.
- 800-line file cap (`bun run check:file-sizes`):
  `commands/waveform/mod.rs` is the most likely casualty once the
  export video graph also gains stretch; check current line count
  during blueprint and plan a split if the added code pushes it
  over.
- i18n: every new user-visible string needs all 20 locale files
  updated (`bun scripts/check-translations.ts`).
- `bindings.ts` is specta-generated; regenerate, do not hand-edit.

## 6. Data Model (sketch)

```rust
// Persisted on ProjectSettings; serde-default so v1.1.0 files load.
#[serde(default)]
pub segment_stretches: Vec<SegmentStretch>,

pub struct SegmentStretch {
    // Stable identity: source-time start of the segment as observed
    // at save time. Survives non-structural edits.
    pub anchor_start_us: i64,
    pub anchor_end_us: i64,
    // Clamped to [0.5, 2.0]; default 1.0.
    pub stretch: f32,
}
```

Alternative (evaluated in BLUEPRINT): add `stretch: f32` to `Word` at
keep-segment-start boundaries. Rejected because stretch is a
segment-level, not word-level, concept.

## Q&A

Treated as pre-answered by the seeding request. Recorded verbatim
for audit.

- **Q1 — Stretch bounds?** A: `[0.5, 2.0]` (single-pass `atempo`;
  chained ranges out of scope).
- **Q2 — Default per-segment value?** A: `1.0` (no stretch).
- **Q3 — UI surface?** A: Numeric input + slider in the segment
  context menu (right-click on a keep-segment). Keyboard shortcuts
  deferred to `keyboard-shortcuts-cheatsheet`.
- **Q4 — Time-mapping authority?** A: Backend
  `canonical_keep_segments_for_media` updated to carry stretch; all
  edit<->source mappings flow through it. Preview uses stretched
  time map to sync the `<video>` element's `playbackRate` — never
  swap the video source.
- **Q5 — Caption alignment through stretched regions?** A: Caption
  time boundaries flow through the stretched time map so they do
  not drift.
- **Q6 — Chapter markers?** A: Independent downstream bundle; will
  consume the stretched time map when shipped.
- **Q7 — Pre-feature project files?** A: Must load with stretch=1.0
  via `#[serde(default)]`. Backward-compat fixture required.

