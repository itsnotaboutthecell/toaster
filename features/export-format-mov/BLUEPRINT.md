# Blueprint: export format mov

## Architecture decisions

- **R-001 — Enum extension.** Add `Mov` as a new variant of the
  existing `AudioExportFormat` enum in
  `src-tauri/src/commands/waveform/export_format.rs:29`. Follow the
  exact pattern of the existing `Mp4` variant: `#[serde(rename_all =
  "lowercase")]` handles the wire form, `extension()` returns
  `".mov"`, `is_audio_only()` returns `false`, and
  `export_format_codec_map` returns `None`. Rationale: `Mov` is a
  pure container swap on top of the video pipeline; it shares every
  code path with `Mp4` except the muxer flag.
- **R-001 — Muxer flag placement.** In
  `src-tauri/src/commands/waveform/mod.rs:571-729` (`build_export_args`),
  insert `-f mov` and `-pix_fmt yuv420p` tokens just before
  `args.push(output_path.to_string())` at line 727, conditioned on
  `format == AudioExportFormat::Mov`. For `Mp4` the existing
  implicit-from-extension behavior is preserved (no behaviour
  change). Rationale: a single guarded branch keeps the diff
  minimal, and the explicit flag is defensive against the
  extension-swap corner case at
  `src-tauri/src/commands/waveform/commands.rs:464-473`.
- **R-002 — i18n key pattern.** Mirror the existing key shape
  (`src/i18n/locales/en/translation.json:458-461`):
  `settings.export.format.options.mov.label =
    "Video (mov)"`
  `settings.export.format.options.mov.description =
    "Edited video with H.264 video and AAC audio in a mov container.
     Use for Final Cut / Premiere / Resolve import on macOS."`
  Only the English copy is authored here; the other 19 locales use
  an English fallback stub flagged for translation per the existing
  convention.
- **R-002 — Settings round-trip.** No new field is added; the
  existing `AppSettings.export_format` field
  (`src-tauri/src/settings/types.rs:294`) is typed as the enum, so
  adding a variant is sufficient. Default remains `Mp4`
  (`src-tauri/src/settings/defaults.rs:546`).
- **R-003 — Argv parity.** The codec-parity test lives alongside
  existing export-argv tests in
  `src-tauri/tests/export_format_args_no_video_stream.rs` or a new
  sibling file under `src-tauri/tests/`. It uses the existing
  `build_audio_only_export_args_for_tests` shim or a new video-path
  shim that composes `build_export_args` with a synthetic segment
  list.

## Component & module touch-list

| Area | File | Change |
|---|---|---|
| Backend | `src-tauri/src/commands/waveform/export_format.rs:29-58,89-113` | Add `Mov` variant + extension string. |
| Backend | `src-tauri/src/commands/waveform/mod.rs:571-729` | Append `-f mov` + `-pix_fmt yuv420p` in `build_export_args` when `format == Mov`. |
| Backend tests | `src-tauri/src/commands/waveform/export_format.rs` (module `tests`) | Extend `export_format_codec_map_matches_prd_spec`, add `export_format_mov_variant` and `export_format_mov_settings_roundtrip`. |
| Backend tests | `src-tauri/tests/` (new file or extension of an existing sibling) | `export_format_mov_codec_parity` — argv parity assertion. |
| Frontend | `src/components/settings/advanced/ExportGroup.tsx:18` | Extend `EXPORT_FORMATS` from `["mp4", "mp3", "wav", "m4a", "opus"]` to `["mp4", "mov", "mp3", "wav", "m4a", "opus"]`. |
| Frontend bindings | `src/bindings.ts` | Regenerate via specta; do NOT hand-edit beyond the permitted one-line union patch (AGENTS.md rule). |
| i18n | `src/i18n/locales/*/translation.json` (20 files) | Add `settings.export.format.options.mov.{label,description}` keyed at the same nesting depth as `mp4`. |
| No-change | `src-tauri/src/managers/export.rs` (transcript export) | Unrelated enum; do not touch. |
| No-change | `src-tauri/src/commands/waveform/commands.rs:464-473` | Extension-swap already handles arbitrary extensions; no change needed. |

## Single-source-of-truth placement

- **Authoritative enum**: `AudioExportFormat` in
  `src-tauri/src/commands/waveform/export_format.rs`. All container
  / extension / codec-spec questions are answered here.
- **Authoritative argv builder**: `build_export_args` in
  `src-tauri/src/commands/waveform/mod.rs:571`. The mov-specific
  `-f mov` and `-pix_fmt yuv420p` flags live here, not in any
  caller.
- **Frontend consumer**: `ExportGroup.tsx:18` (`EXPORT_FORMATS`).
  This file **duplicates** the backend variant list because specta
  emits the union as a TS string literal type but does not emit an
  array of values. That is an acknowledged deviation from the
  dual-path rule. This bundle does NOT fix it (out of scope per the
  REQUEST), but the deviation is captured here so
  `superpowers:code-reviewer` can catch drift on future additions.
- **i18n consumer**: `ExportGroup.tsx:112` formats the label via
  `t("settings.export.format.options.${value}.label")`. Adding
  `"mov"` to `EXPORT_FORMATS` automatically requires a matching
  key in every locale; `bun scripts/check-translations.ts` is the
  gate.
- **Settings persistence**: `AppSettings.export_format` in
  `src-tauri/src/settings/types.rs:294`. The `serde(rename_all =
  "lowercase")` attribute on the enum is the only source of truth
  for the on-disk spelling.

## Data flow

1. User selects `Video (mov)` in Advanced Settings > Export.
2. `ExportGroup.tsx:102` calls `updateSetting("export_format",
   "mov")`.
3. Setting is persisted via Tauri command to `settings.json`.
4. Next export invocation: `export_edited_media`
   (`commands/waveform/commands.rs:386`) reads
   `settings.export_format` (line 455), computes effective
   `has_video`, extension-swaps the output path to `.mov` (line
   464-473), and calls `build_export_args`.
5. `build_export_args` emits the libx264 + AAC argv as before, plus
   `-f mov -pix_fmt yuv420p`, and writes to the `.mov` path.
6. FFmpeg 7 muxes an H.264 + AAC stream inside a QuickTime
   container. Codec layer is untouched — the same bytes that would
   have landed in an `.mp4` land in a `.mov`.

No change to: keep-segment selection, time-mapping, caption
generation, filler detection, loudness, seam-fade, silence filter.

## Migration / compatibility

- **Settings files.** Existing `settings.json` with
  `"export_format": "mp4"` continues to work (default unchanged).
  New `"mov"` values deserialize cleanly after this bundle lands.
  No migration code needed.
- **Downgrade.** A user who manually picks `"mov"`, persists, then
  rolls back to a build without this bundle will see serde fail on
  the enum. Documented in the journal as an accepted risk (opt-in
  forward-only feature).
- **Bindings regeneration.** `src/bindings.ts` is specta-generated;
  the backend enum change must be followed by `cargo build` /
  `specta` emit step. Ad-hoc one-line union edits are permitted
  per AGENTS.md only as a temporary patch; the build must produce
  the final bindings.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| FFmpeg ambiguity between `-f mov` implicit and explicit causes a muxer mismatch | Explicit `-f mov` + extension swap both enforced; ffprobe fixture test on real export. | AC-001-b |
| Frontend `EXPORT_FORMATS` drifts from backend enum | Dual-path deviation called out in Blueprint; i18n gate catches missing key. | AC-002-a + AC-001-c |
| Codec path for mov diverges from mp4, breaking `hardware-encoder-fallback` later | Codec-parity cargo test enforces argv byte-equality modulo `-f` and extension. | AC-003-a |
| i18n key added only to English; CI gate fails | Task `mov-i18n-20-locales` is blocked by the translation-check script in its own AC. | AC-002-a |
| Settings round-trip breaks on old settings.json | Default remains `Mp4`; new variant is purely additive. | AC-002-b |
| 800-line cap breached in `commands/waveform/mod.rs` | Current 704 lines + < 20 added; monitored by `bun run check:file-sizes`. | (session-level gate) |
| Audio-only-inside-mov surprises a user | UI gates mov option behind `has_video`; back-end allows but does not advertise. | AC-001-c (manual UI check) |

