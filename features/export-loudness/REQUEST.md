# Feature request: Export Loudness (preflight + loudnorm)

## 1. Problem & Goals

Closes PRD `product-map-v1` F1 (FFmpeg loudnorm) and Blocker B4
(Loudness preflight). Today export silently applies (or omits)
`loudnorm=I=-16:TP=-1.5:LRA=11`
(`src-tauri/src/commands/waveform/mod.rs:121`) based on a single
`normalize_audio_on_export` boolean
(`src-tauri/src/settings/types.rs:262`). The user gets no readout of
current LUFS / true-peak / LRA, no choice of target, and no preflight
warning if the source is already clipping or wildly off-target.

`splice::loudness::measure_loudness`
(`src-tauri/src/managers/splice/loudness.rs:40`) wraps the `ebur128`
crate and returns a deterministic `LoudnessReport { integrated_lufs,
true_peak_dbtp, lra }`. It is shipping but unused.

Goal: surface a real preflight readout in the export dialog and a
target-LUFS picker, with `splice/loudness.rs` as the single source of
truth for both the preflight numbers and the parameters that build the
`loudnorm` filter argument.

## 2. Desired Outcome & Acceptance Criteria

- A new Export Settings panel exists in the main Settings sidebar.
- The export dialog, before commit, runs an analysis-only EBU R128
  pass over the post-edit audio and renders integrated LUFS, true-peak
  dBTP, and LRA.
- A `Loudness normalization` control offers: Off / Podcast (-16 LUFS)
  / Streaming (-14 LUFS).
- The exported file's `loudnorm` filter parameters are derived from
  the same `LoudnessReport` and target the user-selected LUFS — no
  duplicate parameter math in the frontend.
- `audio-boundary-eval` skill still passes after the filter-chain
  changes.
- Backend authority: all loudness math is in Rust. The frontend
  consumes a serialized DTO; it never re-computes LUFS.

## 3. Scope Boundaries

### In scope

- New `ExportSettings` panel (sidebar entry + panel component).
- `loudness_preflight` Tauri command that calls
  `splice::loudness::measure_loudness` over the post-edit audio.
- Export dialog preflight readout (LUFS / dBTP / LRA, three numbers).
- Target picker (Off / -16 / -14) persisted in settings.
- Refactor `commands/waveform/mod.rs:121` so the `loudnorm=...` string
  is built by a Rust helper that reads target LUFS from a single
  settings field.

### Out of scope (explicit)

- Real-time `ebur128` overlay in the editor (PRD F12, deferred).
- Per-segment loudness control.
- Replacing `loudnorm` with an offline ebur128-based gain ramp (kept
  as PRD `product-map-v1` Q2 open question; deferred).
- Network-based loudness analytics (forbidden).

## 4. References to Existing Code

- `src-tauri/src/managers/splice/loudness.rs:25-40` — `LoudnessReport`
  struct and `measure_loudness` function (the authority).
- `src-tauri/src/commands/waveform/mod.rs:102-121` —
  `build_audio_post_filters`, where the current `loudnorm` string is
  pushed.
- `src-tauri/src/settings/types.rs:262` — `normalize_audio_on_export`
  boolean (to be replaced by a target enum).
- `src/components/settings/CaptionSettings.tsx:1-138` — settings panel
  pattern to follow.
- `eval/fixtures/toaster_example.mp4` — fixture used by
  `audio-boundary-eval` and the new round-trip test.

## 5. Edge Cases & Constraints

- Preflight runs on the post-edit audio (after keep-segments), not the
  raw source; otherwise the readout would not match what the user is
  about to export.
- Preflight cancel: a long file may take seconds; the user can
  continue without preflight (button stays enabled).
- Off-target by >12 LU: render a non-blocking warning in the dialog,
  do not block the export.
- ASCII-only changes; no hosted inference.
- File-size cap (800 lines) holds for new `.rs` and `.tsx`.

## 6. Data Model (optional)

Settings:

- `loudness_target: "off" | "podcast_-16" | "streaming_-14"`
  (default `"off"`; back-compat: existing `true` value of
  `normalize_audio_on_export` migrates to `"podcast_-16"`).

DTO returned by `loudness_preflight`:

- `{ integrated_lufs: f64, true_peak_dbtp: f64, lra: f64,
    target_lufs: Option<f64> }`.

## Q&A

Pre-answered by the orchestrator (no `ask_user` round needed). Captured
verbatim per Phase 5 protocol.

- Q: Does the Export panel get its own sidebar slot, or live under
  Models?
  - A: Own panel. Bundles 2 and 3 will pile on; an own panel keeps the
    Models panel focused.
- Q: Does loudness-preflight run on raw source or post-edit audio?
  - A: Post-edit. The user wants to know what the *export* will sound
    like, not the pre-edit source.
- Q: Replace `loudnorm` with offline ebur128 gain?
  - A: No. PRD `product-map-v1` Q2 is open and deferred. This bundle
    keeps `loudnorm` and only sources its target parameter from the
    Rust helper that reads the new settings field.
- Q: Default target?
  - A: Off. Migrating users who had `normalize_audio_on_export = true`
    land on `podcast_-16` to preserve current behavior.
