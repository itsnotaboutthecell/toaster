# PRD: time stretch segments

## Problem & Goals

Let users speed up or slow down individual keep-segments of an edit
without changing pitch, so the transcript-first edit loop can
rebalance pacing without re-recording. Source: Milestone 2 of
`features/product-map-v1/PRD.md` §6.

Goals:

1. Per-segment stretch factor, stored persistently on the project,
   default `1.0`, clamped to `[0.5, 2.0]`.
2. Preview playback and exported media apply the same stretch via a
   single backend authority (no dual-path drift).
3. Caption time boundaries stay aligned with their words after
   stretch.
4. Existing `.toaster` files load unchanged with stretch=1.0 on
   every segment.
5. Users adjust stretch through a segment context menu
   (numeric input + slider + reset) in the editor.

## Scope

### In scope

- New persisted per-segment stretch user intent on `ProjectSettings`.
- Stretch-aware canonical keep-segments and time-map helpers in
  `commands::waveform` and `managers::captions::layout`.
- `atempo` injection in the shared preview/export audio graph and
  matching `setpts` on the export video stream.
- Segment context-menu UI with numeric input, slider, reset.
- Preview cache-key update to include stretch factors.
- Backward-compat project fixture test.
- Eval fixtures extended to cover stretched segments.
- i18n parity for new user-visible strings.

### Out of scope (explicit)

- Chained `atempo` for factors outside `[0.5, 2.0]`.
- Keyboard shortcuts for stretch adjustment.
- Chapter markers (consumes stretch-aware time map when shipped).
- Speed-leveling suggestions or ML-driven auto-stretch.
- Alternative pitch-preserving algorithms beyond FFmpeg `atempo`.
- Swapping the `<video>` source to an audio preview file
  (forbidden by AGENTS.md).

## Requirements

### R-001 — Persisted stretch data model

- Description: `ProjectSettings` gains a new
  `segment_stretches: Vec<SegmentStretch>` field, serde-default so
  existing project files still deserialize. Each entry stores a
  stretch factor in `[0.5, 2.0]` keyed by source-time boundary
  anchors.
- Rationale: Keep-segments are derived from `Vec<Word>` at load time
  (`managers/editor/mod.rs:305`); per-segment user intent must live
  on a persisted surface independent of derivation. The
  `#[serde(default)]` pattern mirrors
  `ProjectSettings.caption_profiles` (`managers/project.rs:45-46`).
- Acceptance Criteria
  - AC-001-a — `ProjectSettings` exposes a
    `segment_stretches` field annotated with `#[serde(default)]`;
    its element type carries a `stretch: f32` defaulting to `1.0`.
  - AC-001-b — Writes with stretch outside `[0.5, 2.0]` are rejected
    (or clamped) at the setter boundary; the invariant is documented
    in a rustdoc comment adjacent to the setter and enforced by a
    unit test.

### R-002 — Single-source-of-truth stretch plumbing

- Description: `canonical_keep_segments_for_media` returns the
  authoritative stretch factor alongside each `(start_us, end_us)`
  pair. `build_audio_segment_filter` inserts `atempo=<stretch>`
  exactly once per segment; export video graph applies the matching
  `setpts` factor. Preview and export consume the same producer.
- Rationale: AGENTS.md "NEVER duplicate dual-path logic" — preview,
  export, and time maps already route through
  `canonical_keep_segments_for_media`; extending that one producer
  keeps the invariant.
- Acceptance Criteria
  - AC-001-c — Exporting a project whose segment has stretch `s`
    produces an audio track whose duration for that segment equals
    `source_duration / s` within 1 audio sample at the output sample
    rate.
  - AC-002-a — Rendering the preview for the same project produces a
    preview audio duration for that segment matching
    `source_duration / s` within 1 audio sample, and matches the
    export-path duration from AC-001-c to within 1 sample.

### R-003 — Caption alignment through stretched segments

- Description: `map_source_to_edit` in
  `managers/captions/layout.rs` consumes per-segment stretch so that
  caption start/end in the edited timeline equal
  `elapsed_edit + (source_offset_in_segment / stretch)`. The inverse
  used by preview cursor/export (`map_edit_time_to_source_time_*`)
  applies the same factor symmetrically.
- Rationale: Stretch drift in caption layout would misalign burned-in
  subtitles in export video, the user-visible failure mode Toaster
  must never ship.
- Acceptance Criteria
  - AC-002-b — For an exported video containing a stretched segment,
    every caption line's rendered start/end frame falls within 1
    frame of the stretched word's actual on-screen position, verified
    by a fixture run of `transcript-precision-eval`.

### R-004 — Segment context-menu UI

- Description: Right-clicking a keep-segment in the editor opens a
  context menu containing a numeric input (two-decimal precision) and
  a slider bound to the same stretch value. A "Reset to 1.0" button
  restores the default. Changes persist to the project and trigger a
  preview re-render.
- Rationale: Users need a discoverable, low-friction control; the
  context menu is the established editor convention.
- Acceptance Criteria
  - AC-003-a — Live-app: opening the segment context menu on a
    keep-segment exposes a numeric input + slider tied to stretch,
    clamped to `[0.5, 2.0]`, and a reset button that restores `1.0`
    and re-renders preview.

### R-005 — Backward compatibility

- Description: `.toaster` files saved before this feature must load
  without error and report stretch `1.0` for every keep-segment. The
  project version is bumped only when first re-saved with a
  non-default stretch.
- Rationale: `PROJECT_VERSION` is `1.1.0`
  (`managers/project.rs:17`); migration has to be non-destructive
  for legacy files, matching the `caption_profiles` precedent.
- Acceptance Criteria
  - AC-004-a — A pre-feature `.toaster` fixture loads successfully
    and a cargo test asserts every derived segment's effective
    stretch equals `1.0`.

## Edge cases & constraints

- Zero- or negative-width segments must not reach `atempo`. The
  existing guards in `canonical_keep_segments_for_media` remain the
  gate; the stretch-carrying variant preserves them.
- Seam-fade duration (`seam_fade_duration_seconds`) is specified in
  source microseconds; with stretch the rendered fade scales by
  `1/stretch`. Policy: fade duration is authored in source time;
  rendered duration scales accordingly. Documented in the blueprint.
- Preview cache key (`edit_version_token`) must include stretch
  factors so stale previews never play after a stretch change.
- Frontend `editTimeToSourceTime` helper is currently piecewise-
  linear without stretch. Stretched segments route through the
  backend `map_edit_to_source_time` IPC; no TS-side stretch math.
- `<video>` element `playbackRate` is set to the stretch of the
  segment under the edit-time cursor; source is never swapped.
- 800-line cap: `commands/waveform/mod.rs` is already dense. The
  blueprint's touch-list calls out a split if the added code pushes
  the file over the cap.

## Data model

```rust
// managers/project.rs — ProjectSettings additions
#[serde(default)]
pub segment_stretches: Vec<SegmentStretch>,

// managers/project.rs — new persisted type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SegmentStretch {
    pub anchor_start_us: i64,
    pub anchor_end_us: i64,
    // Invariant: 0.5 <= stretch <= 2.0. Setter clamps.
    pub stretch: f32,
}

// commands/waveform/mod.rs — canonical IPC type (specta)
pub struct KeepSegment {
    pub start_us: i64,
    pub end_us: i64,
    #[serde(default = "one_f32")]
    pub stretch: f32,
}
```

Internal `canonical_keep_segments_for_media` returns
`Vec<CanonicalKeepSegment { start_us, end_us, stretch }>` (blueprint
decides tuple-vs-struct trade-off).

## Non-functional requirements

- No hosted-inference dependency introduced.
- All new i18n keys mirrored across the 20 locale files
  (`bun scripts/check-translations.ts`).
- `bun run check:file-sizes` stays green.
- `bun run check:file-sizes` and `scripts/check-translations.ts`
  wired into existing CI paths; no new gates introduced.
- `src/bindings.ts` regenerated via specta; not hand-edited.

