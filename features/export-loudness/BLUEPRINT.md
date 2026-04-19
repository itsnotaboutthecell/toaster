# Blueprint: Export Loudness (preflight + loudnorm)

## Architecture decisions

- **R-001 (Export panel)**: new `src/components/settings/export/
  ExportSettings.tsx`, sidebar entry under the existing settings
  pattern (`src/components/settings/CaptionSettings.tsx:1-138`). Own
  panel chosen because Bundles 2 (audio-only formats) and 3 (hardware
  encoder) will add controls; nesting under Models would conflate
  transcription with rendering.
- **R-002 (preflight)**: new Tauri command `loudness_preflight` in
  `src-tauri/src/commands/waveform/commands.rs`. Internally it
  decodes the post-edit PCM (reusing the same keep-segments path the
  exporter walks, so preview/export/preflight share one
  audio-source-of-truth) and calls
  `splice::loudness::measure_loudness`
  (`src-tauri/src/managers/splice/loudness.rs:40`). Returns the DTO.
- **R-003 (target picker + filter authority)**: new helper
  `build_loudnorm_filter(target: LoudnessTarget) -> Option<String>`
  in `src-tauri/src/managers/splice/loudness.rs` (co-located with the
  measurement so both halves of the SSOT live in one file). Replaces
  the inline string at `commands/waveform/mod.rs:121`. Frontend reads
  the enum from settings and renders a 3-option `Select`; never
  builds the filter string.
- **R-004 (migration)**: `migrate_loudness_setting` helper in
  `src-tauri/src/settings/mod.rs`, called from the existing settings
  load path. Legacy boolean field is kept for one release as
  `#[serde(default)]` for downgrade safety, then removed in a
  follow-up bundle.
- **R-005 (boundary-eval)**: filter-chain refactor only changes
  *which* `loudnorm` is emitted, not seam fades / silence chains.
  Verifier is the existing `audio-boundary-eval` skill.
- **R-006 (backend authority)**: DTO exported via `bindings.ts`; the
  React readout component formats the floats with `toFixed(1)` and
  appends units. No arithmetic.

## Component & module touch-list

| File | Change |
|------|--------|
| `src-tauri/src/managers/splice/loudness.rs` | Add `LoudnessTarget` enum, `build_loudnorm_filter`, plus 3 unit tests. |
| `src-tauri/src/commands/waveform/mod.rs:121` | Replace inline `loudnorm=...` with `splice::loudness::build_loudnorm_filter(settings.loudness_target)`. |
| `src-tauri/src/commands/waveform/commands.rs` | Add `loudness_preflight` Tauri command. |
| `src-tauri/src/settings/types.rs:262` | Add `loudness_target: LoudnessTarget` field. |
| `src-tauri/src/settings/defaults.rs` | Default = `LoudnessTarget::Off`. |
| `src-tauri/src/settings/mod.rs` | `migrate_loudness_setting` helper. |
| `src-tauri/src/lib.rs` | Register `loudness_preflight` command. |
| `src/components/settings/export/ExportSettings.tsx` | New panel; loudness target Select. |
| `src/components/settings/export/index.ts` | New barrel. |
| `src/components/dialogs/ExportDialog.tsx` (or current export-dialog file) | Add Preflight section reading `LoudnessPreflight` DTO. |
| `src/i18n/locales/*/translation.json` (20 files) | Add `sidebar.export`, `settings.export.loudness.*`, `dialog.export.preflight.*` keys. Use `i18n-pruning` skill. |
| `src/bindings.ts` | Auto-regenerated to include `loudness_preflight` and `LoudnessPreflight` DTO. |

## Single-source-of-truth placement

- **Loudness math**: authority is
  `src-tauri/src/managers/splice/loudness.rs` for both
  `measure_loudness` (preflight DTO source) and
  `build_loudnorm_filter` (export filter parameters). Frontend
  consumers: `ExportSettings.tsx` (Select) and the export dialog's
  preflight readout. Frontend never re-derives target LUFS or
  re-computes integrated LUFS.

## Data flow

```
User opens Export dialog
  -> click "Run preflight"
  -> invoke loudness_preflight
       -> commands/waveform/commands.rs::loudness_preflight
            -> walk keep-segments, decode PCM
            -> splice::loudness::measure_loudness
            -> return LoudnessPreflight DTO
  -> dialog renders integrated_lufs / true_peak_dbtp / lra

User commits export
  -> commands/waveform/mod.rs::build_audio_post_filters
       -> splice::loudness::build_loudnorm_filter(settings.loudness_target)
            -> Some("loudnorm=I=-16:TP=-1.5:LRA=11") | Some(... -14 ...) | None
  -> filter pushed into FFmpeg argv
```

## Migration / compatibility

- Legacy `normalize_audio_on_export` boolean kept as
  `#[serde(default, skip_serializing)]` for one release.
  `migrate_loudness_setting` runs once on settings load.
- `bindings.ts` regenerated; consumers in TS update to read
  `loudness_target` enum string. Codegen is automatic.

## Sequencing & conflict-avoidance

- **Position**: bundle 1 of 5. Establishes the Export panel scaffold.
- **Files this bundle owns and others must not touch in parallel**:
  `src/components/settings/export/`, `src-tauri/src/managers/splice/
  loudness.rs`, the inline filter at `commands/waveform/mod.rs:121`.
- **Files this bundle agrees not to touch**: anything outside
  `commands/waveform/` and the listed settings files; specifically no
  edits to `src-tauri/src/managers/captions/`, `managers/editor/`,
  `managers/transcription/`.
- **`tauri.conf.json`**: not touched by this bundle. Bundle 4
  (release-code-signing) owns `signCommand` edits later.
- **Downstream**: bundles 2 and 3 add format/encoder dropdowns to the
  same `ExportSettings.tsx` after this bundle merges.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Engineer hand-builds `loudnorm=` string in TS to "save a roundtrip" (recreates dual-path defect) | AC-003-b is a literal grep; PR review enforces | AC-003-b |
| Preflight decodes raw source instead of post-edit audio (numbers drift from export) | Blueprint mandates same keep-segments walk; cargo test compares preflight LUFS to post-export LUFS | AC-002-b, AC-003-c |
| Filter-chain refactor changes seam fade order | `audio-boundary-eval` skill | AC-005-a |
| Settings migration loses user's prior boolean preference | `migrate_loudness_setting` unit test | AC-004-a, AC-004-b |
| Hosted inference sneaks in via a "cloud LUFS" library | AGENTS.md non-negotiable; `dep-hygiene` skill on PR | (architectural; reviewer-enforced) |
| File-size cap exceeded in `commands/waveform/mod.rs` (already large) | Move new helpers into `splice/loudness.rs` not `waveform/mod.rs` | (size linter) |
