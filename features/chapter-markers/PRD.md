# PRD: chapter markers

## Problem & Goals

Exports produced by `export_edited_media` (Tauri command wired at
`src-tauri/src/lib.rs:294`, implementation at
`src-tauri/src/commands/waveform/commands.rs:386`) ship without
chapter navigation. Players that honour mp4/mov chapter atoms or
WebVTT chapter sidecars therefore show a single opaque timeline.
The transcript pipeline already carries paragraph-level structure;
this bundle turns that structure into chapter markers on every
export, written to both the container and a sidecar VTT, with no
new UI surface.

## Scope

### In scope

- Backend `Vec<Chapter>` builder driven by the upstream paragraph
  grouping and the canonical keep-segment list.
- ffmetadata file generation + `-map_metadata` wiring in the
  existing export command.
- Sidecar `<basename>.chapters.vtt` writer.
- Fixture evals covering the happy path, title-truncation rule,
  short-paragraph merge rule, time-stretched-segment interaction,
  and the empty-paragraph-list case.

### Out of scope (explicit)

- Any new paragraph clustering heuristic. Paragraph boundaries are
  read from the upstream pipeline, not invented here.
- Per-export opt-out UI toggle.
- Custom chapter thumbnails or art.
- Source-time chapter timestamps (explicitly forbidden).
- Any hosted-inference path for title generation.

## Requirements

### R-001 — Emit chapters to both container and sidecar

- Description: On every call to `export_edited_media`, build a
  chapter list from the transcript paragraph grouping and the
  canonical keep-segments, then emit chapter markers into the mp4
  or mov container (via ffmpeg `-f ffmetadata` + `-map_metadata`)
  AND into a sidecar `<basename>.chapters.vtt` file.
- Rationale: WebVTT wins on portability (YouTube, browser
  `<track kind="chapters">`); container metadata wins on
  embedded-player support (QuickTime, VLC, Apple Podcasts). Users
  should not have to pick.
- Acceptance Criteria
  - AC-001-a — A pure-Rust unit test calls
    `build_chapters_for_export(paragraphs, keep_segments)` with a
    fixture of three paragraphs and asserts a `Vec<Chapter>` of
    length 3 whose `start_us`/`end_us`/`title` fields match the
    fixture expectations.
  - AC-001-b — A fixture export is produced; running
    `ffprobe -show_chapters` on the output reports chapter atoms
    whose start/end match the backend's `Vec<Chapter>` within 1 ms
    per boundary.
  - AC-001-c — A pure-Rust unit test invokes the VTT writer on a
    `Vec<Chapter>` fixture and asserts the output begins with
    `WEBVTT\n\n`, contains one `<start> --> <end>\n<title>\n\n` cue
    per chapter, uses `HH:MM:SS.mmm` timestamps, and contains no
    `NOTE` lines that were not present in the input.

### R-002 — Title derivation and short-paragraph merge

- Description: Chapter titles come from the first ~6 words of the
  paragraph's text, trimmed, trailing punctuation removed. If the
  resulting string exceeds 64 chars, truncate to 60 chars and
  append an ellipsis (U+2026, `…`). Any paragraph whose duration
  (in edit time) is less than 5,000,000 us (5 s) is merged into
  the preceding chapter; if the short paragraph is the first one,
  it is merged into the next instead.
- Rationale: 6 words is the established YouTube/podcast convention
  and keeps titles scannable. The 5 s floor prevents a scrub bar
  littered with micro-chapters from short transcript fragments.
- Acceptance Criteria
  - AC-002-a — A pure-Rust unit test exercises the title
    derivation with (i) a short sentence (keeps all <=6 words),
    (ii) a long sentence whose first 6 words exceed 64 chars (must
    be truncated to 60 chars + `…`), and (iii) a sentence ending
    in punctuation (trailing `.`, `,`, `?`, `!`, `;`, `:` removed).
  - AC-002-b — A pure-Rust unit test builds chapters from a
    fixture of four paragraphs whose durations are
    `[4s, 30s, 3s, 20s]` and asserts the resulting chapter list
    has length 2, the first chapter absorbs paragraphs 1 and 2,
    and the second chapter absorbs paragraphs 3 and 4.

### R-003 — Chapter timestamps are in exported (edit) time

- Description: Chapter `start_us` and `end_us` MUST be expressed
  in the edited, exported timeline. The builder consumes the same
  `canonical_keep_segments_for_media` list the rest of the export
  pipeline uses, and reuses
  `src-tauri/src/managers/export.rs::map_source_range_to_edit_time`
  (or a private equivalent in the chapters module) to remap each
  paragraph's source-time span onto the edit timeline. Any future
  time-stretch transform (bundle
  `features/time-stretch-segments/`) MUST compose with this
  mapping rather than bypass it.
- Rationale: If chapters were in source time, every scrub target
  in a player would point at the wrong spot in an edited export.
- Acceptance Criteria
  - AC-003-a — A fixture eval renders an edited export whose
    keep-segments include one segment marked with a synthetic 2x
    time-stretch spanning a paragraph boundary. The eval parses
    `ffprobe -show_chapters` and asserts that each chapter's
    reported start matches the analytically computed edit-time
    start within 1 ms, confirming that chapter timestamps track
    the stretch rather than source time.

### R-004 — Graceful empty-paragraph case

- Description: If the upstream transcript pipeline surfaces zero
  paragraphs for the edit (e.g. transcript is empty or the
  pipeline has not produced paragraph grouping yet), the export
  MUST succeed, MUST NOT pass a `-map_metadata` argument to
  ffmpeg, and MUST NOT create a `<basename>.chapters.vtt` file.
- Rationale: Chapters are an enhancement, never a blocker. Empty
  inputs must not break existing export behaviour.
- Acceptance Criteria
  - AC-004-a — A fixture export is produced from an edit whose
    paragraph list is empty. The eval asserts (i) the export
    command returned success, (ii) `ffprobe -show_chapters` on
    the output reports zero chapters, and (iii) no
    `<basename>.chapters.vtt` file exists next to the output.

## Edge cases & constraints

- Sidecar path derivation reuses the `Path::with_extension`
  pattern already in `commands/waveform/commands.rs:438`, setting
  the extension to `chapters.vtt` (two-component extension is
  intentional; matches the sibling `ass-sidecar-export` naming).
- WebVTT timestamps use the `HH:MM:SS.mmm` form. Hours are
  zero-padded to at least 2 digits.
- Title derivation operates on whatever text the paragraph
  exposes. Whitespace is collapsed to single spaces before the
  6-word cut.
- Short-paragraph merge preserves the **earlier** chapter's title;
  only the end timestamp grows.
- Chapter list MUST be deterministic: identical inputs produce
  byte-identical ffmetadata + VTT output (stable ordering, no
  HashMap iteration).
- No new i18n keys. Titles are user content, not UI chrome.

## Data model (if applicable)

```rust
pub struct Chapter {
    pub start_us: i64,   // microseconds, edit time
    pub end_us: i64,     // microseconds, edit time, > start_us
    pub title: String,   // UTF-8, <= 64 chars inc. ellipsis
}
```

Invariants:

- `start_us >= 0`
- `end_us > start_us`
- Chapters are non-overlapping, sorted by `start_us` ascending.
- `end_us[i] == start_us[i+1]` for adjacent chapters (no gaps).
- `end_us` of the last chapter equals the total edit duration.

## Non-functional requirements

- Chapter building + writing adds < 10 ms to an export of a
  60-minute edit on a mid-range machine (order-of-magnitude budget;
  tracked via the eval, not a hard gate).
- No new crate dependencies; VTT and ffmetadata are plain-text
  formats written with `std::fmt` / `std::fs`.
- No new i18n keys. `scripts/check-translations.ts` stays green by
  construction.
- `cargo machete` stays green (no unused deps introduced).
