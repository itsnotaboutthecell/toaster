# Feature request: Edit Export Format Override

## 1. Problem & Goals

Users can set a default final-export container/format under `Settings -> Advanced -> Export` (`settings.export_format: AudioExportFormat`), but cannot override it per edit-project. The Editor's "Export Edited Media" button (`src/components/editor/EditorView.tsx:431`, handler at `:312`) does not expose a per-invocation picker, and the save-dialog suggests a filename extension derived from the source file (`EditorView.tsx:315`) that can disagree with `settings.export_format`. The backend (`src-tauri/src/commands/waveform/commands.rs:455`) currently reads only the global setting, then silently rewrites the extension (`:464-473`).

Goal: let users pick a final-export format per edit-project directly in the Export button flow, while keeping the Advanced-settings value as the default. Backend remains the single authority for format selection; frontend only passes the chosen enum.

## 2. Desired Outcome & Acceptance Criteria

- An Editor-side control (near the Export button) lets the user pick a format for the current export. Options are filtered to match the active project's source media type (video project does not show audio-only formats unless the user opts in; audio project does not show video containers).
- When the user does not change the control, the export uses `settings.export_format` (current behavior preserved).
- When the user changes the control, `export_edited_media` runs with the chosen format; `settings.export_format` is not mutated.
- The save-dialog suggested filename extension matches the chosen format, not the source extension.
- All new strings land as i18next keys mirrored across all 20 locales.
- No preview playback, keep-segments, time-mapping, captions, ASR, or network-call code is touched.

## 3. Scope Boundaries

### In scope

- Editor Export button UX: format picker, state, save-dialog extension.
- Tauri command `export_edited_media` signature: new optional override parameter.
- Backend wiring: when override is `Some`, use it; when `None`, fall back to `settings.export_format` (no behavior change).
- Media-type-aware format list helper (backend function, consumed by frontend).
- i18n keys for all new user-visible strings in all 20 locales.
- `src/bindings.ts` regeneration via specta.

### Out of scope (explicit)

- Preview playback path, waveform rendering, keep-segments / time-mapping.
- ASR, captions, filler detection, cleanup / LLM.
- Persisting the per-project override to disk (project-save format). Override is ephemeral for the current session's export action.
- New export formats. Set stays `Mp4`, `Mp3`, `Wav`, `M4a`, `Opus`.
- Changing `Settings -> Advanced -> Export` surface (other than verifying it still works).
- Any hosted inference / network call.

## 4. References to Existing Code

- `src-tauri/src/commands/waveform/export_format.rs:29-165` — `AudioExportFormat` enum, `is_audio_only`, `extension`, `export_format_codec_map`. Authority for format metadata.
- `src-tauri/src/commands/waveform/commands.rs:386-510` — `export_edited_media` Tauri command; current single consumer of `settings.export_format` for edit export.
- `src-tauri/src/commands/waveform/mod.rs:580, 751` — `build_export_args` signature carries `format: AudioExportFormat`. Extend call sites; do not change signature.
- `src-tauri/src/settings/types.rs:289-294` — `Settings::export_format` field (default surface).
- `src-tauri/src/settings/defaults.rs:546` — default value `Mp4`.
- `src/components/editor/EditorView.tsx:312-336` — `handleExportEditedMedia` (call site to extend).
- `src/components/editor/EditorView.tsx:431-439` — Export button (UX anchor).
- `src/components/settings/advanced/ExportGroup.tsx:10-110` — pattern to mirror for the format picker (list + change-handler + label).
- `src/bindings.ts:767, 1088` — specta-generated command signature and enum.
- `src/i18n/locales/*/translation.json` — 20 locale files; `editor.*` namespace for export-related strings.
- `scripts/check-translations.ts` — i18n gate.

## 5. Edge Cases & Constraints

- Video-source project with audio-only override: user explicitly chose to strip video. Existing backend code (`commands.rs:457` `effective_has_video = has_video && !export_format.is_audio_only()`) already handles this; reuse, do not duplicate.
- Audio-source project with `Mp4` override: `Mp4` in current enum represents the "keep source video + AAC audio" path. For an audio-only source, selecting `Mp4` has no meaning; it must not appear in the picker.
- Save-dialog extension must update when the user changes the override after opening the picker but before clicking Export.
- If the save-dialog returns a path whose extension disagrees with the chosen override (user typed a different extension manually), backend's existing extension-rewrite at `commands.rs:464-473` is authoritative; no frontend override.
- i18n: every user-visible string (picker label, per-format display name if different from enum value, tooltip, any error toast) must be an i18next key. No inline English fallbacks.
- Specta: the override enum reuses `AudioExportFormat`. Adding an `Option<AudioExportFormat>` parameter to the command requires `pnpm tauri specta` (or equivalent) regen of `src/bindings.ts`; do not hand-edit.
- 800-line cap applies to any new `.ts`/`.tsx`/`.rs` file.
- The override is scoped to a single Export invocation; closing and re-opening the project resets to `settings.export_format`. Persisting to the `.toaster` project file is explicitly out of scope.

## 6. Data Model (optional)

No new enum. Reuse `AudioExportFormat` (`src-tauri/src/commands/waveform/export_format.rs:29`).

New backend helper (signature sketch):

```
pub fn allowed_formats_for_source(ext: &str) -> Vec<AudioExportFormat>;
// Video source (mp4/mkv/mov/avi/webm/flv) -> [Mp4, Mp3, Wav, M4a, Opus]
// Audio source (m4a/mp3/wav/...)          -> [Mp3, Wav, M4a, Opus]   (no Mp4)
```

Updated command signature (sketch; exact naming during implementation):

```
pub async fn export_edited_media(
    app: AppHandle,
    store: State<'_, EditorStore>,
    input_path: String,
    output_path: String,
    burn_captions: Option<bool>,
    format_override: Option<AudioExportFormat>, // NEW
) -> Result<String, String>;
```

Resolution rule: `let export_format = format_override.unwrap_or(settings.export_format);`

## Q&A

No clarifying round required. The user's one-liner plus PLAN-phase Q1 pins all six elements:

- **Placement**: Editor's Export button flow (not Advanced).
- **Default authority**: `settings.export_format` stays the default.
- **Backend single source of truth**: backend owns format selection; frontend passes the enum only.
- **Video vs audio**: source-media-type-aware list.
- **i18n**: all strings keyed across 20 locales.
- **bindings.ts**: specta regen, not hand-edit.

Any remaining ambiguity (e.g. exact picker widget shape, whether to persist override in project file) is resolved by the explicit "Out of scope" list in REQUEST.md section 3.
