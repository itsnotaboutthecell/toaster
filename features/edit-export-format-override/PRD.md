# PRD: Edit Export Format Override

## Problem & Goals

The Editor's "Export Edited Media" action cannot override the final-export format per project. Default is correctly held in `settings.export_format` (`src-tauri/src/settings/types.rs:294`, default `Mp4` at `defaults.rs:546`) and honored by the backend (`commands/waveform/commands.rs:455`), but the UI buries it in `Settings -> Advanced -> Export`, forcing users to round-trip through global settings for a per-project choice.

Goal: expose a per-invocation format picker in the Editor's Export button flow. Backend remains the single authority for format-to-FFmpeg mapping; frontend passes only the chosen `AudioExportFormat`. `settings.export_format` stays the default when the user does not override.

## Scope

### In scope

- Editor Export button: format picker control colocated with the button; state is component-local (ephemeral).
- `export_edited_media` Tauri command: new optional `format_override: Option<AudioExportFormat>` parameter.
- Backend helper `allowed_formats_for_source(ext: &str) -> Vec<AudioExportFormat>` so frontend can show a media-type-aware list without duplicating policy.
- Save-dialog suggested filename extension derived from the chosen override (or `settings.export_format` if no override).
- i18n keys mirrored across all 20 locales under `editor.exportFormat.*`.
- `src/bindings.ts` regenerated via specta.

### Out of scope (explicit)

- Persisting the override in the `.toaster` project file.
- Adding new `AudioExportFormat` variants.
- Preview playback, keep-segments, time-mapping, captions, ASR, filler, cleanup.
- Any hosted / network inference.
- Changing `Settings -> Advanced -> Export` content or layout.

## Requirements

### R-001 - Per-project override on Editor Export button

- Description: The Editor's Export Edited Media action presents a format picker next to the Export button. Changing the picker selects a per-invocation `AudioExportFormat` override.
- Rationale: eliminates the round-trip through Advanced settings and the settings-vs-filename inconsistency.
- Acceptance Criteria
  - AC-001-a - A format picker control is rendered adjacent to the Export button at `src/components/editor/EditorView.tsx` when media is loaded; options come from the backend helper `allowed_formats_for_source` and are not hard-coded on the frontend.
  - AC-001-b - When the user selects a format in the picker and clicks Export, the Tauri command `export_edited_media` is invoked with `format_override = Some(<chosen>)`; the backend uses the override instead of `settings.export_format`.
  - AC-001-c - When the user never interacts with the picker, `export_edited_media` is invoked with `format_override = None` and the backend uses `settings.export_format`; there is no behavior change versus today.

### R-002 - Default authority stays in Settings -> Advanced -> Export

- Description: `settings.export_format` remains the global default and the fallback when no override is set.
- Rationale: single source of truth for the default; no behavior change for users who never open the picker.
- Acceptance Criteria
  - AC-002-a - `Settings -> Advanced -> Export` continues to bind to `settings.export_format`; its UI surface at `src/components/settings/advanced/ExportGroup.tsx` is unmodified except for any imports required by shared types.
  - AC-002-b - With no override set, changing `settings.export_format` in Advanced still changes the format used by the Editor's Export button.

### R-003 - Backend is the single authority for format-to-FFmpeg mapping

- Description: All format-to-extension, format-to-codec, and format-to-video-stream decisions live in backend (`AudioExportFormat::extension`, `export_format_codec_map`, `build_export_args`). Frontend passes the enum only.
- Rationale: dual-path rule in AGENTS.md - preview and export share a single backend authority; frontend must not reinterpret.
- Acceptance Criteria
  - AC-003-a - Frontend code contains no mapping from `AudioExportFormat` to codec, FFmpeg argv, or video-stream policy. The only frontend mapping allowed is `AudioExportFormat -> localized display label` via i18n keys.
  - AC-003-b - The Tauri command signature change is regenerated into `src/bindings.ts` via specta; `bindings.ts` is not hand-edited.

### R-004 - Source-media-type-aware format list

- Description: The picker shows only formats that make sense for the active project's source media type.
- Rationale: avoid offering `Mp4` for an audio-only source; avoid surfacing unexpected video-stripping when the user just wants a different container.
- Acceptance Criteria
  - AC-004-a - For a video source (extension in `{mp4, mkv, mov, avi, webm, flv}`), `allowed_formats_for_source` returns `[Mp4, Mp3, Wav, M4a, Opus]` in that order.
  - AC-004-b - For an audio-only source (extension not in the video set), `allowed_formats_for_source` returns `[Mp3, Wav, M4a, Opus]` (no `Mp4`).
  - AC-004-c - Selecting an audio-only override on a video source still produces a valid audio-only file; existing backend behavior `effective_has_video = has_video && !export_format.is_audio_only()` (`commands/waveform/commands.rs:457`) continues to hold.

### R-005 - Save-dialog filename extension matches the chosen format

- Description: The save-dialog's suggested filename uses the extension of the chosen override (or `settings.export_format` when no override).
- Rationale: today the dialog uses the source file's extension (`EditorView.tsx:315`), producing a filename that disagrees with the actual output format.
- Acceptance Criteria
  - AC-005-a - When the picker is unchanged, the save-dialog suggested filename ends in `AudioExportFormat::extension()` for `settings.export_format` (e.g. `.mp4` for default).
  - AC-005-b - When the picker is changed, the save-dialog suggested filename ends in `AudioExportFormat::extension()` for the chosen override.

### R-006 - i18n hygiene

- Description: Every user-visible string introduced by this feature is an i18next key mirrored across all 20 locale files.
- Rationale: AGENTS.md non-negotiable; `scripts/check-translations.ts` is the CI gate.
- Acceptance Criteria
  - AC-006-a - `scripts/check-translations.ts` exits 0 after the feature lands. New keys (all under `editor.exportFormat.*`): `editor.exportFormat.label`, `editor.exportFormat.tooltip`, `editor.exportFormat.formatMp4`, `editor.exportFormat.formatMp3`, `editor.exportFormat.formatWav`, `editor.exportFormat.formatM4a`, `editor.exportFormat.formatOpus`.

### R-007 - Scope boundaries honored

- Description: No change outside the files enumerated in BLUEPRINT.md. Precision / boundary evals stay green.
- Rationale: the sprint's stated hard scope; prevents dual-path regressions in preview / keep-segments.
- Acceptance Criteria
  - AC-007-a - The precision regression `cargo test -p toaster-lib precision_eval` passes after the feature lands.
  - AC-007-b - No file touched by this feature exceeds 800 lines.
