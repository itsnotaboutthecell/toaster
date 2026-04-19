# Blueprint: chapter markers

## Architecture decisions

- **R-001 / R-002 / R-003 — Chapter list lives in a new
  `src-tauri/src/managers/export/chapters.rs` submodule.**
  Pattern followed: the sibling file
  `src-tauri/src/managers/export.rs:200` already owns
  edit-timeline remap logic for SRT export
  (`export_srt_for_edited_timeline` + private
  `remap_words_to_edit_timeline` + `map_source_range_to_edit_time`).
  The chapters module reuses `map_source_range_to_edit_time`
  verbatim — do NOT fork it — which keeps the canonical
  keep-segment mapping in one place and stays consistent with the
  SRT path. If that helper is currently private, promote it to
  `pub(crate)` in `export.rs` rather than duplicating it.
- **R-001 — Sidecar naming.** Use
  `Path::with_extension("chapters.vtt")` against the resolved
  `output_path_buf` computed at
  `src-tauri/src/commands/waveform/commands.rs:464`. This matches
  the existing `with_extension("burn_captions.ass")` pattern at
  `commands.rs:438` and the convention the sibling bundle
  `features/ass-sidecar-export/` will formalize.
- **R-001 — ffmetadata injection.** Write a temp `*.ffmetadata`
  file (alongside the ASS temp at `commands.rs:438`) containing the
  FFmpeg ffmetadata v1 chapter block, then extend
  `build_export_args` (`src-tauri/src/commands/waveform/mod.rs:571`)
  to take the metadata path and, when `Some(path)`, insert
  `-f ffmetadata -i <path>` before the existing `-i <input>` and
  append `-map_metadata <new_input_index>` to the output-side
  args. The function already carries
  `#[allow(clippy::too_many_arguments)]`; this stays consistent.
- **R-003 — Time-stretch composition.** When
  `features/time-stretch-segments/` lands, it will expose a
  stretch-aware variant of the keep-segment model (not yet
  designed). The chapter builder therefore accepts the same
  opaque `&[(i64, i64)]` keep-segments that the rest of the
  export pipeline accepts, and consumes any stretch transform via
  the same helper the SRT path uses. If that helper is upgraded
  for stretch, chapters inherit the fix for free.
- **R-004 — Empty-paragraph path.** Guard at the top of
  `export_edited_media`: if `build_chapters_for_export` returns an
  empty vector, pass `None` for the ffmetadata path into
  `build_export_args` and skip the sidecar write. Do not create
  an empty `WEBVTT\n\n` file.

## Component & module touch-list

Backend (Rust):

- `src-tauri/src/managers/export.rs`
  - Promote `map_source_range_to_edit_time` from private to
    `pub(crate)` so the new chapters module can reuse it.
  - Add `mod chapters;` and re-export the public `Chapter` type
    + `build_chapters_for_export` + `chapters_to_ffmetadata` +
    `chapters_to_webvtt`.
- `src-tauri/src/managers/export/chapters.rs` (new)
  - `pub struct Chapter`
  - `pub fn build_chapters_for_export(paragraphs, keep_segments)
    -> Vec<Chapter>`
  - `fn derive_title(paragraph_text: &str) -> String`
  - `fn merge_short_chapters(chapters: Vec<Chapter>,
    min_duration_us: i64) -> Vec<Chapter>`
  - `pub fn chapters_to_ffmetadata(chapters: &[Chapter]) -> String`
  - `pub fn chapters_to_webvtt(chapters: &[Chapter]) -> String`
  - Unit tests for AC-001-a, AC-001-c, AC-002-a, AC-002-b.
- `src-tauri/src/commands/waveform/commands.rs`
  - In `export_edited_media` (commands.rs:386) between
    `snap_segments_against_media` (commands.rs:482) and the
    `build_export_args` call (commands.rs:483):
    - Fetch the paragraph list from the editor store / transcript
      pipeline (exact source TBD by task
      `chapter-markers-paragraph-source`).
    - Call `build_chapters_for_export(&paragraphs, &snapped_segments)`.
    - If non-empty, write the ffmetadata temp file and the
      `<basename>.chapters.vtt` sidecar; capture the ffmetadata
      `PathBuf`.
  - Extend the cleanup block at `commands.rs:506` to remove the
    ffmetadata temp regardless of export outcome.
- `src-tauri/src/commands/waveform/mod.rs`
  - Extend `build_export_args` (mod.rs:571) with an optional
    `chapter_metadata_path: Option<&str>` parameter; when `Some`,
    prepend `-f ffmetadata -i <path>` and append
    `-map_metadata <idx>`.
  - Unit-test the new branch with existing
    `build_export_args_*` helpers in
    `src-tauri/src/commands/waveform/tests/part2.rs`.

Evals (shell):

- `scripts/eval/eval-chapter-markers.ps1` — happy-path fixture
  export + ffprobe assertions (covers AC-001-b, AC-004-a).
- `scripts/eval/eval-chapter-markers-stretch.ps1` — stretch
  interaction (covers AC-003-a).

Frontend: none. No i18n changes.

## Single-source-of-truth placement

- **Chapter list.** Authority lives in
  `managers::export::chapters::build_chapters_for_export`. Both
  the ffmetadata writer and the WebVTT writer accept the same
  `&[Chapter]` slice. The ffmpeg invocation and the sidecar
  write pull from the same `Vec<Chapter>` computed once per
  export.
- **Keep-segment model.** Authority remains
  `canonical_keep_segments_for_media`
  (`src-tauri/src/commands/waveform/mod.rs:331`). The chapter
  builder consumes the already-snapped segments returned by
  `snap_segments_against_media` (commands.rs:482) — the same
  slice `build_export_args` gets.
- **Edit-time mapping.** Authority is
  `managers::export::map_source_range_to_edit_time`. Chapters
  reuse it; no parallel implementation.
- **Sidecar naming.** Authority is `Path::with_extension`, same
  helper used by the ASS temp at commands.rs:438. No custom
  string-concatenation path builder.

## Data flow

```
TranscriptParagraphs (from upstream pipeline)
            │
            ▼
canonical_keep_segments_for_media ──► snap_segments_against_media
            │                                   │
            └──────────────┬────────────────────┘
                           ▼
              build_chapters_for_export
                           │
           ┌───────────────┴────────────────┐
           ▼                                ▼
  chapters_to_ffmetadata          chapters_to_webvtt
           │                                │
           ▼                                ▼
  temp .ffmetadata file        <basename>.chapters.vtt
           │
           ▼
  build_export_args ──► ffmpeg ──► output.mp4
           │
           ▼
  -map_metadata <idx>
```

## Migration / compatibility

- No data migration; this is a pure additive export-side feature.
- No settings keys added. No i18n keys added.
- Existing exports without paragraph data keep working unchanged
  (R-004 / AC-004-a).
- No changes to the `transcribe_rs` adapter contract — this
  bundle is a read-only consumer of whatever paragraph data the
  upstream pipeline exposes.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| R1 — No existing paragraph data structure in backend (repo grep for `paragraph\|Paragraph` in `src-tauri/src/` returns zero hits). Seed forbids inventing a clustering heuristic. | Task `chapter-markers-paragraph-source` is the first gate: it MUST (a) locate an existing upstream paragraph signal — e.g. `TranscriptionSegment` boundaries from `transcribe_rs` used at `src-tauri/src/managers/transcription/adapter_normalize.rs:14`, or an existing editor-level grouping — and record the decision in `journal.md`, or (b) surface a blocker and halt the bundle. No new heuristic allowed. | AC-001-a (fixture-driven; decouples chapter-builder tests from upstream reality). |
| R2 — `features/time-stretch-segments/` changes the keep-segment model underneath the chapter builder. | Chapter builder consumes the same `&[(i64, i64)]` slice the rest of the export path consumes, via the shared `map_source_range_to_edit_time`. Any stretch upgrade lands in that helper, so chapters inherit it. | AC-003-a (stretch fixture). |
| R3 — ffmpeg builds without ffmetadata demuxer support on some platforms. | ffmetadata demuxer is in the default ffmpeg build and has been available since FFmpeg 1.0. The repo already shells out to `ffmpeg` unconditionally (see commands.rs:500). Document the requirement in the task context; no code-level mitigation needed. | AC-001-b (fixture verifies ffprobe output; fails loudly on unsupported ffmpeg). |
| R4 — Sidecar filename collision with user-authored `<basename>.chapters.vtt`. | Overwrite behaviour mirrors the existing burn-captions ASS temp path (overwrite without prompting). Document in the task context; no UI to expose. | AC-001-c (grammar test) — does not mitigate collision, but prevents corruption via malformed output. |
| R5 — Players disagree on ffmetadata chapter title interpretation (some expect `title=` UTF-8, some latin-1). | Emit UTF-8, which is the ffmetadata spec and what ffmpeg's demuxer produces. Do not attempt to re-encode. | AC-001-b (ffprobe parity). |
