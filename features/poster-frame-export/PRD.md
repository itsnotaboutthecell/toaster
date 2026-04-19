# PRD: poster-frame-export

## Problem & Goals

Toaster's mp4/mov exports have no embedded cover image. File browsers
and video players that honor attached-picture metadata fall back to
whatever frame the encoder happens to select (often a black frame
near the first keyframe), which looks unfinished. `product-map-v1`
flags this as Launch-Ready polish (NH4 / F10).

Goal: let the user pick one frame from the edited timeline as the
video's poster frame and embed it in mp4/mov exports via FFmpeg's
attached-picture stream, with zero behavior change for projects that
do not set one.

## Scope

### In scope

- New optional field on the project storing an edit-time timestamp.
- One new context-menu entry in the transcript editor to set the
  poster frame.
- Backend-side extraction (from the already-rendered edited output)
  and attachment (via FFmpeg `-attach`) at export time.
- Backward-compatible project load/save.
- i18n parity (one new key across all 20 locales).

### Out of scope (explicit)

- Any UI beyond the single context-menu entry (no picker view, no
  thumbnail preview, no settings page toggle).
- Auto-picked poster frames (no scene/face detection).
- webm or audio-only poster-frame support.
- Runtime network calls or hosted inference.
- Cross-OS file-explorer thumbnail guarantees.

## Requirements

### R-001 -- Persist and surface a poster-frame selection

- Description: The project file stores one optional edit-time
  timestamp identifying the poster frame. Users set it from the
  transcript editor's word-level context menu; unset is the default.
- Rationale: The minimal persistent state required to make
  poster-frame embedding a project property rather than a per-export
  argument.
- Acceptance Criteria
  - AC-001-a -- `ProjectSettings` gains a `poster_frame_ms:
    Option<u64>` field with `#[serde(default)]`; a project saved
    with `Some(ms)` round-trips that exact value through
    save-then-load, and a project written by an older version
    (missing the field) loads with `poster_frame_ms == None` without
    error.
  - AC-001-b -- The transcript editor's word-level context menu
    includes a "Set as poster frame" entry wired to an i18next key
    that is present and non-empty in every `src/i18n/locales/*/translation.json`.

### R-002 -- Embed the chosen frame in mp4/mov export

- Description: When a project has `poster_frame_ms = Some(ms)` and
  the selected export format is mp4 or mov, the export pipeline
  extracts a PNG from the edited output at `ms` and attaches it as a
  cover image via FFmpeg `-attach` with `mimetype=image/png`. When
  the field is `None`, the export command line is byte-for-byte
  identical to today's (no attachment).
- Rationale: The visible deliverable. Attachment must be driven by
  the backend alone to stay consistent with the "one backend
  authority, two consumers" rule for dual-path logic.
- Acceptance Criteria
  - AC-002-a -- Given a fixture project with `poster_frame_ms =
    Some(ms)` in range of the edited timeline, exporting to mp4
    produces a file for which `ffprobe -show_streams` reports at
    least one stream with `codec_type=attachment` and
    `TAG:mimetype=image/png`.
  - AC-002-b -- Given the same fixture with format switched to mov,
    the export produces an output for which `ffprobe -show_streams`
    reports an attachment stream tagged `image/png`.
  - AC-002-c -- Given a fixture project with `poster_frame_ms =
    None`, exporting to mp4 produces a file whose `ffprobe
    -show_streams` output contains no stream with
    `codec_type=attachment`; the FFmpeg argv logged by
    `export_edited_media` is byte-for-byte identical to the argv
    produced for the same fixture built on the pre-feature code
    path (recorded as a golden in the eval).
  - AC-002-d -- Given a project with `poster_frame_ms = Some(ms)`
    and a `ms` value greater than the edited timeline's total
    duration, the extractor clamps to the last available frame and
    the resulting mp4 still carries a valid attached-picture stream
    (no export error, no panic).

### R-003 -- Correctness against other export paths

- Description: The feature must not interfere with audio-only
  exports, with webm (if and when it is ever emitted), or with the
  existing time-stretch-segments and chapter-markers bundles.
- Rationale: Poster-frame storage is edit-time, which matches the
  coordinate system of every downstream path, but we still need a
  gate against regressions in those adjacent features.
- Acceptance Criteria
  - AC-003-a -- An audio-only export (mp3 / wav / m4a / opus) with
    `poster_frame_ms = Some(ms)` produces a file whose `ffprobe
    -show_streams` output reports no attachment stream; no FFmpeg
    `-attach` argument appears in the argv.
  - AC-003-b -- For any export run (with or without a poster
    frame), no file matching `toaster_poster_*.png` remains in
    `std::env::temp_dir()` after the export command returns; the
    assertion holds both on success and on a forced FFmpeg failure.

<!--
  AC IDs MUST NOT be bold. Use `AC-001-a` verbatim, never `**AC-001-a**`.
  The coverage gate regex (scripts/feature/check-feature-coverage.ps1:57) matches
  `^\s*-?\s*AC-\d{3}-[a-z]\b` -- a leading `**` breaks extraction and the
  gate fails with "PRD.md has no AC-NNN-x entries".
-->

## Edge cases & constraints

- Timestamp is edit-time (ms into the rendered edited output); no
  source-time remapping is required.
- Timestamps past the edited duration clamp to the last frame rather
  than erroring.
- Temp PNG must be deleted on both success and failure paths.
- mp4/mov are the only containers that receive the attachment;
  others ignore the field silently.
- Backward compatibility: v1.0.0 and v1.1.0 project files load
  cleanly; `PROJECT_VERSION` bumps to 1.2.0 on next save.
- 800-line cap on all touched files.
- i18n parity: exactly one new key; mirrored in 20 locales.
- No new runtime network calls.

## Data model (if applicable)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    // ...existing fields...

    /// Edit-time timestamp (ms into the rendered edited timeline)
    /// of the frame to embed as the poster/cover image on mp4/mov
    /// export. `None` means "no poster frame". Pre-1.2.0 projects
    /// deserialize with this field as `None` via `#[serde(default)]`.
    #[serde(default)]
    pub poster_frame_ms: Option<u64>,
}
```

`PROJECT_VERSION` bumps from `"1.1.0"` to `"1.2.0"`.

## Non-functional requirements

- No hosted-inference or network dependency introduced.
- Temp-file hygiene: zero orphaned `toaster_poster_*.png` entries
  across repeated exports.
- Single source of truth: poster-frame argv construction lives in
  `build_export_args` (backend), never duplicated into frontend
  logic.
- Export argv for the "no poster frame" case stays byte-identical
  to the pre-feature baseline (guards against accidental flag drift
  in unrelated exports).
