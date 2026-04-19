# Blueprint: Edit Export Format Override

## Architecture decisions

- **R-001, R-004** — Add a Rust helper `allowed_formats_for_source(ext: &str) -> Vec<AudioExportFormat>` in `src-tauri/src/commands/waveform/export_format.rs` (alongside `export_format_codec_map` at line 89). Expose it as a `#[tauri::command]` (e.g. `list_allowed_export_formats`) so frontend consumes it via specta — no frontend duplication of the video-extension set. Follows the pattern already used by `export_format_codec_map` (pure function + unit tests in `tests/part2.rs`).
- **R-001, R-003** — Extend `export_edited_media` (`commands/waveform/commands.rs:386`) with an optional `format_override: Option<AudioExportFormat>` parameter. Resolution at the existing `let export_format = settings.export_format;` site (line 455) becomes `let export_format = format_override.unwrap_or(settings.export_format);`. This keeps the existing `build_export_args` call (line 483) and `effective_has_video` computation (line 457) untouched — override is indistinguishable from a settings change from that point downward.
- **R-002** — `Settings -> Advanced -> Export` (`src/components/settings/advanced/ExportGroup.tsx`) is read-only with respect to this feature. No structural edits; the component continues to bind to `settings.export_format`.
- **R-005** — `EditorView.tsx` extension derivation at `:315` changes from `mediaInfo.extension || ...` to a function that reads `(override ?? settings.export_format).extension()` — but since `AudioExportFormat::extension()` is Rust-side, a tiny TS mirror `formatToExtension(fmt: AudioExportFormat): string` lives either in a generated constant or as a lookup table. To avoid dual-path drift, prefer adding a second Tauri command `export_format_extension(format: AudioExportFormat) -> String` OR — simpler — export a `const FORMAT_EXTENSIONS: Record<AudioExportFormat, string>` from the same module that defines `EXPORT_FORMATS` in `ExportGroup.tsx:19`, sourced by a backend JSON fixture consumed through the allowed-formats command payload (shape: `{ format, extension }`). Final choice during implementation; blueprint requirement is **no hand-maintained duplicate extension map on frontend**.
- **R-006** — New i18n keys land under `editor.exportFormat.*`. Per-format labels are keyed (not string-interpolated from the enum) so locales can render "MP4 (video)" etc.
- **R-007** — Every new `.ts/.tsx/.rs` file is introduced at < 400 lines to leave headroom under the 800-line cap (AGENTS.md).

## Component & module touch-list

### Modified

- `src-tauri/src/commands/waveform/export_format.rs` — add `allowed_formats_for_source` and its unit tests.
- `src-tauri/src/commands/waveform/commands.rs` — add `format_override` parameter to `export_edited_media`; resolution line at :455.
- `src-tauri/src/commands/waveform/mod.rs` — re-export `allowed_formats_for_source` and register the new Tauri command.
- `src-tauri/src/lib.rs:294` area — add new command(s) to `tauri::Builder::invoke_handler`.
- `src/components/editor/EditorView.tsx` — add format picker state + control, wire `format_override` into `commands.exportEditedMedia`, derive save-dialog extension from picker.
- `src/i18n/locales/<all 20>/translation.json` — add the 7 new keys under `editor.exportFormat`.
- `src/bindings.ts` — specta-regenerated (do not hand-edit).

### New files

- `src/components/editor/ExportFormatPicker.tsx` — small presentational component (< 150 lines) rendering a select bound to an `AudioExportFormat` list. Follows the pattern at `src/components/settings/advanced/ExportGroup.tsx:44-110`.
- `src-tauri/src/commands/waveform/tests/export_format_override.rs` (or extend `tests/part2.rs`) — unit tests for `allowed_formats_for_source` and for the override resolution rule in `export_edited_media` (via a thin seam if the command is hard to unit-test directly).

### Untouched (asserted)

- `src-tauri/src/managers/splice/*` (seam boundaries, preview authority).
- `src-tauri/src/managers/editor/*` (keep-segments, time mapping).
- `src-tauri/src/managers/captions/*`.
- `src-tauri/src/managers/transcription/*`.

## Single-source-of-truth placement

- **Format metadata (codec, extension, video/audio split)**: `src-tauri/src/commands/waveform/export_format.rs`. Frontend never re-derives.
- **Format list by source type**: `allowed_formats_for_source` in the same file, exposed as a Tauri command. Frontend consumes; never duplicates the video-extension set.
- **Override resolution rule**: `export_edited_media` in `commands/waveform/commands.rs:455` — one line, one authority.
- **Default**: `settings.export_format` — unchanged.
- **Preview**: N/A — preview path never consumes export format; this feature adds zero dual-path surface.

## Data flow

```
User opens editor with project P (source: video.mp4)
  v
EditorView mounts, calls commands.listAllowedExportFormats(ext="mp4")
  v
Backend returns [Mp4, Mp3, Wav, M4a, Opus]
  v
ExportFormatPicker renders options; default selection = settings.export_format
  v
User selects Mp3 (override)
  v
User clicks "Export Edited Media"
  v
EditorView opens save dialog with suggested name "...-edited.mp3"
  v
User confirms path
  v
commands.exportEditedMedia(inputPath, outputPath, burnCaptions, format_override=Some(Mp3))
  v
Backend: export_format = Some(Mp3).unwrap_or(settings.export_format)  // = Mp3
  v
Backend: effective_has_video = true && !Mp3.is_audio_only()           // = false
  v
build_export_args(..., format=Mp3) -> FFmpeg runs -> output.mp3
```

## Migration / compatibility

- Tauri command signature change is a breaking ABI change. Handled by specta regen of `src/bindings.ts`. All callers of `commands.exportEditedMedia` are under `src/components/editor/EditorView.tsx` (single call site at :330) and are updated in the same PR.
- No persistent data migration: override is ephemeral session state.
- `.toaster` project files unchanged.
- Settings schema unchanged (`settings.export_format` kept as-is).

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Frontend drifts a duplicate extension map and disagrees with backend | Forbid hand-maintained TS extension map; source from backend payload or specta-generated const | AC-003-a |
| Override silently ignored by backend (plumbing bug) | Backend unit test with both `Some` and `None` paths | AC-001-b, AC-001-c |
| `Mp4` offered for audio-only source, then produces an empty video stream | `allowed_formats_for_source` filters server-side; unit-tested | AC-004-a, AC-004-b |
| Save-dialog filename extension diverges from chosen format | Derive save-dialog `extensions[0]` from `(override ?? settings.export_format).extension()` | AC-005-a, AC-005-b |
| New locale keys missing from 1-of-20 files | `scripts/check-translations.ts` in CI | AC-006-a |
| Preview / keep-segments regression sneaks in via shared module edits | Precision eval re-run as gate | AC-007-a |
| A touched file crosses 800 lines | Split `ExportFormatPicker.tsx` as its own component; keep `EditorView.tsx` edit surgical | AC-007-b |
| `bindings.ts` hand-edited instead of regenerated | PR review + CI specta check | AC-003-b |
