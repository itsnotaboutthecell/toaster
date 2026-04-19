# Feature request: poster-frame-export

## 1. Problem & Goals

Toaster today exports mp4/mov files with no embedded cover image, so
file browsers and video players fall back to whatever poster frame the
encoder happens to land on (often a black frame near the first
keyframe). `features/product-map-v1/PRD.md:376` (NH4) and
`features/product-map-v1/PRD.md:414` (F10) both flag this as a
Launch-Ready polish item that separates a v1.0 release from a
tech-preview.

Goal: let the user nominate one frame from the *edited* timeline as
the video's poster frame, and embed it in mp4/mov exports via FFmpeg's
attached-picture mechanism.

## 2. Desired Outcome & Acceptance Criteria

- Right-click a word in the transcript editor -> context menu entry
  "Set as poster frame" writes an edit-time timestamp (ms) onto the
  current project. Exactly one poster frame per project;
  re-selecting replaces the previous value.
- mp4/mov export with a selected poster frame produces a file whose
  `ffprobe -show_streams` output includes an attached-picture stream
  carrying a PNG extracted from the edited timeline at the stored
  timestamp.
- mp4/mov export with no selection produces no attached picture
  (matches today's behavior).
- webm export ignores the poster frame silently.
- The project file round-trips the new field; projects saved before
  this feature load cleanly via `#[serde(default)]`.
- The temporary PNG used for attachment is written to the OS temp
  directory and removed after export (success or failure).

## 3. Scope Boundaries

### In scope

- New `Option<u64>` field on the project for the poster-frame
  timestamp (edit-time milliseconds).
- One new i18n-keyed context-menu label ("Set as poster frame").
- Rust-side frame extraction via FFmpeg from the rendered edited
  output at the stored timestamp, followed by attachment into the
  final mux.
- mp4 and mov containers only.

### Out of scope (explicit)

- Thumbnail hover-preview of the selected frame anywhere in the UI.
- A dedicated poster-frame picker view (the transcript context menu
  is the whole selection UI for v1).
- webm poster-frame support (player-dependent; low ROI).
- Auto-suggested poster frames (scene detection, face detection).
- OS-specific guarantees about file-explorer thumbnail display.
- Any network-based image sourcing (local-only inference rule).

## 4. References to Existing Code

- `src-tauri/src/managers/project.rs:19-47` -- `ToasterProject` and
  `ProjectSettings` serde structs; pattern for `#[serde(default)]`
  backward-compat already used for `caption_profiles`
  (`src-tauri/src/managers/project.rs:41-46`).
- `src-tauri/src/commands/waveform/commands.rs:386-535` --
  `export_edited_media`, the single FFmpeg mux entry point; temp-ASS
  cleanup at `src-tauri/src/commands/waveform/commands.rs:505-508`
  is the pattern for "delete temp artifact on success or failure".
- `src-tauri/src/commands/waveform/mod.rs` -- `build_export_args`
  builds the FFmpeg command line; poster-frame extraction and
  attachment arguments must be injected here (and only here) so
  preview vs export stays in one place.
- `src-tauri/src/commands/waveform/export_format.rs:47-51` -- format
  dispatch (`.mp4`, `.mov`, audio-only variants); webm is not a
  first-class video export format today, so the AC for "webm ignored
  silently" is protective rather than currently reachable.
- `src/components/editor/TranscriptEditor.tsx:194-201,360-366` --
  existing word-level `onContextMenu` wiring; the new menu entry
  plugs into the same context-menu component the file already
  renders.
- `src/i18n/locales/*/translation.json` -- i18n parity target
  (`bun scripts/check-translations.ts`).

## 5. Edge Cases & Constraints

- Timestamp is **edit-time** (ms relative to the rendered edited
  timeline). The PNG is extracted from the exported video's own
  coordinate system, not from the source file, so time-stretched or
  deleted segments upstream do not require remapping.
- If the stored timestamp is beyond the edited duration (e.g., user
  deleted the tail after choosing the poster frame), the extraction
  must clamp to the last available frame and still produce a valid
  attachment.
- Temp PNG path is `std::env::temp_dir().join("toaster_poster_<uuid>.png")`
  and is removed in both success and failure branches, matching the
  existing ASS cleanup pattern.
- mp4/mov only: on other containers the poster frame is silently
  ignored; no warning banner, no error.
- Backward-compat: projects without `poster_frame_ms` load cleanly
  via `#[serde(default)]`; on save the field is written as `null`
  when unset so the on-disk schema is self-describing.
- 800-line file cap applies to all touched `.rs` / `.ts` / `.tsx`.
- i18n parity: one new key must be present in all 20 locales
  (`bun scripts/check-translations.ts`).
- No new runtime network calls.

## 6. Data Model

`ProjectSettings` gains:

```rust
/// Edit-time timestamp (ms into the rendered edited timeline) of the
/// frame to embed as the poster/cover image on mp4/mov export. `None`
/// means "no poster frame" and matches pre-1.2.0 projects.
#[serde(default)]
pub poster_frame_ms: Option<u64>,
```

Bump `PROJECT_VERSION` to `1.2.0`. v1.1.0 and v1.0.0 projects load
cleanly via `#[serde(default)]` and are rewritten on next save.

## Q&A

Treated as already answered by the seed request:

- Q: How does the user pick a frame?
  A: Right-click on any frame in the editor timeline -> "Set as
     poster frame" context menu entry. Exactly one poster frame per
     project; re-selecting replaces the previous one.
- Q: Where is the frame stored?
  A: As an edit-time timestamp (ms) on the project. The PNG is
     extracted at export time from the rendered edited output, not
     from the source file.
- Q: How is it embedded?
  A: For mp4/mov, FFmpeg `-attach <frame.png> -metadata:s:t
     mimetype=image/png`. For webm, ignored.
- Q: Default when unset?
  A: No poster frame in the output (matches today).
- Q: Do we guarantee file-browser thumbnail behavior?
  A: No. File browsers that honor embedded cover images will show
     it; this bundle does not promise behavior on any specific OS.
