# Feature request: chapter markers

## 1. Problem & Goals

Exports produced by `export_edited_media`
(`src-tauri/src/commands/waveform/commands.rs:386`) have no chapter
navigation. Players that honour mp4/mov chapter atoms (QuickTime,
VLC, Apple Podcasts) or WebVTT chapters (YouTube, browser-native
`<track kind="chapters">`, Overcast, Pocket Casts) show a single
opaque timeline. Users producing longer-form talks or podcasts have
to scrub blindly. Toaster already has the structural information
needed to emit chapters — paragraph boundaries in the transcript —
so there is no reason to ship the edited file without them.

Goal: on every export, emit

1. chapter markers embedded in the mp4/mov container via the
   FFmpeg `-f ffmetadata` + `-map_metadata` pipeline, and
2. a sidecar WebVTT chapters file next to the media, named
   `<basename>.chapters.vtt`.

Both outputs derive from a single backend-computed chapter list so
there is no drift between them.

## 2. Desired Outcome & Acceptance Criteria

See `PRD.md` for numbered ACs. Summary:

- Backend emits a deterministic `Vec<Chapter>` during export.
- Container metadata reflects those chapters (verifiable via
  `ffprobe -show_chapters`).
- Sidecar WebVTT file is written alongside the media, valid per the
  WebVTT chapter grammar.
- Chapter titles are the first ~6 words of each paragraph,
  punctuation-trimmed, truncated to 60 chars + ellipsis above 64.
- Paragraphs shorter than 5 s are merged into the preceding chapter.
- Chapter timestamps live in **exported** (edited, possibly
  time-stretched) time space, not source time.
- An export whose transcript has no paragraphs produces neither
  container chapters nor a sidecar file, and does not fail the
  export.

## 3. Scope Boundaries

### In scope

- `Vec<Chapter>` builder on the backend that consumes the same
  paragraph data used elsewhere in the transcript pipeline.
- Edit-time remap of chapter timestamps via the existing
  `canonical_keep_segments_for_media` + `map_source_range_to_edit_time`
  helpers (`src-tauri/src/managers/export.rs:239`).
- ffmetadata writer + invocation wiring in the existing
  `build_export_args` / `export_edited_media` pipeline.
- WebVTT chapter-file writer reusing the chapter list verbatim.
- Fixture eval under `scripts/eval/` covering the happy path and
  the time-stretch interaction.

### Out of scope (explicit)

- Any new paragraph-clustering heuristic. The chapter list rides on
  whatever paragraph grouping the transcript pipeline already
  produces. If no such grouping exists at execution time, the
  bundle's first task MUST adopt an existing upstream signal (not
  invent one) or surface a blocker — see BLUEPRINT Risk R1.
- Per-export opt-out toggle. Chapters are always written when
  paragraphs exist. A future toggle is a separate bundle.
- Frontend UI (no settings page, no timeline badges, no i18n
  strings).
- Custom chapter art/thumbnails.
- Chapter timestamps in source-time space (explicitly forbidden by
  AC-003-a).
- Hosted-inference chapter-title generation (AGENTS.md
  non-negotiable).

## 4. References to Existing Code

- `src-tauri/src/commands/waveform/commands.rs:386-520` —
  `export_edited_media`; the chapter writer hooks in after
  `snap_segments_against_media` and before the ffmpeg invocation.
- `src-tauri/src/commands/waveform/mod.rs:571` —
  `build_export_args`; add `-f ffmetadata` secondary input + a
  `-map_metadata <idx>` argument. Keep the function's
  `#[allow(clippy::too_many_arguments)]` contract explicit.
- `src-tauri/src/commands/waveform/mod.rs:331` —
  `canonical_keep_segments_for_media`; chapter builder consumes
  this, never a re-derived list.
- `src-tauri/src/managers/export.rs:200-260` —
  `export_srt_for_edited_timeline` + `map_source_range_to_edit_time`;
  mapping pattern to mirror for chapter timestamps.
- `src-tauri/src/managers/transcription/adapter_normalize.rs` —
  where paragraph-level data would surface if/when the upstream
  transcript pipeline grows it.
- `features/ass-sidecar-export/` — sibling bundle that defines the
  `<basename>.<kind>.<ext>` sidecar naming convention. Chapter VTT
  sidecar follows the same shape: `<basename>.chapters.vtt`.

## 5. Edge Cases & Constraints

- Transcript has no paragraphs → no ffmetadata argument, no sidecar
  file written, export succeeds (AC-004-a).
- Single paragraph spanning the whole edit → exactly one chapter.
- Paragraph shorter than 5 s → merged into the preceding chapter.
  If it is the first paragraph, it absorbs the next instead (see
  PRD edge case).
- Time-stretched segment spans a paragraph boundary (interaction
  with `features/time-stretch-segments/`) → chapter timestamps are
  in exported time and therefore naturally include the stretch.
- Basename contains spaces, non-ASCII, or filesystem-reserved
  characters → sidecar path reuses the same `PathBuf::with_extension`
  pattern already used at
  `src-tauri/src/commands/waveform/commands.rs:438`.
- Output format is audio-only (e.g. mp3, wav) → container chapters
  may or may not be honoured per codec; still write the sidecar VTT
  unconditionally.
- ffmpeg not installed / ffmetadata write fails → the existing
  export error path already surfaces ffmpeg failures. Do not mask.

## 6. Data Model (optional)

```rust
pub struct Chapter {
    pub start_us: i64,  // microseconds in EDIT time
    pub end_us: i64,    // microseconds in EDIT time
    pub title: String,  // <= 64 chars, trimmed + truncated
}
```

Builder: `fn build_chapters_for_export(
    paragraphs: &[Paragraph],
    keep_segments: &[(i64, i64)],
) -> Vec<Chapter>`

`Paragraph` is provided by the upstream transcript pipeline.
Locating or defining it is task 1 of the bundle.

## Q&A

Seed pre-answered all clarifying questions. Recorded verbatim:

**Q1 — Chapter title source?**
A: First ~6 words of the paragraph, trimmed, trailing punctuation
removed. If > 64 chars, truncate to 60 chars + "…".

**Q2 — Output targets?**
A: Both the container metadata AND the `.chapters.vtt` sidecar.
WebVTT wins on portability, metadata wins on embedded-player
support.

**Q3 — Minimum chapter length?**
A: 5 seconds. Shorter paragraphs are merged into the preceding
chapter.

**Q4 — UI?**
A: Automatic on every export. A future per-export opt-out toggle
is explicitly out of scope.

**Q5 — Paragraph detection?**
A: Rides on whatever paragraph grouping the transcript pipeline
already produces. No new clustering heuristic in this bundle.
