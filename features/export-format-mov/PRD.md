# PRD: export format mov

## Problem & Goals

Offer `.mov` as a user-selectable export container so editors who
ingest into DaVinci Resolve / Final Cut / Premiere on macOS do not
have to round-trip through a second tool. Codec stays H.264; only
the muxer and extension differ. Tracks Milestone 2 roadmap item 2.1
of `features/product-map-v1/PRD.md:471`.

## Scope

### In scope

- Backend enum variant `AudioExportFormat::Mov` with extension
  `".mov"` and FFmpeg muxer flag `mov`.
- Explicit `-f mov` muxer flag in `build_export_args` when the
  variant is selected.
- Frontend dropdown option in `ExportGroup.tsx`.
- i18n key `settings.export.format.options.mov.{label,description}`
  in all 20 locales.
- Settings serde round-trip for `"mov"`.
- Regression test asserting the codec portion of the FFmpeg argv is
  identical for Mp4 and Mov (contract for the forthcoming
  `hardware-encoder-fallback` bundle).

### Out of scope (explicit)

- ProRes, HEVC, AV1, VP9, yuv422p10le, or any non-H.264 codec.
- Hardware encoder detection / selection / fallback (owned by
  `features/export-hardware-encoder/`).
- Audio-only-inside-mov advertising (allowed but not surfaced).
- New save-dialog layout, drag-drop export, or batch export.
- webm / mkv / flac containers (separate roadmap items).

## Requirements

### R-001 — Backend enum and FFmpeg muxer

- Description: Add `Mov` variant to `AudioExportFormat`; wire it
  through `extension()`, `is_audio_only()`, and
  `export_format_codec_map` (returns `None`, like `Mp4`). In
  `build_export_args`, when the variant is `Mov`, append `-f mov`
  and `-pix_fmt yuv420p` to the argv before the output path.
- Rationale: Single source of truth lives in the backend; the
  codec map and argv builder are already the authority for every
  other format (see `export_format.rs` header comment).
- Acceptance Criteria
  - AC-001-a — The backend enum has a `Mov` variant that serializes
    to `"mov"`, reports extension `".mov"`, reports
    `is_audio_only() == false`, and `export_format_codec_map` returns
    `None` for it. Verified by a new cargo test
    `export_format_mov_variant`.
  - AC-001-b — Exporting a fixture at each container (`mp4`, `mov`)
    produces a file whose `ffprobe -show_format` exits 0 and whose
    `format_name` contains the expected muxer substring (`mp4` or
    `mov,mp4,m4a,3gp,3g2,mj2`). Live-app / manual verification on
    `eval/fixtures/toaster_example.mp4`.
  - AC-001-c — The export dialog (advanced settings > Export group)
    shows a `Video (mov)` option alongside the existing `Video
    (mp4)` option. Verified live in the monitored app.

### R-002 — i18n parity and settings round-trip

- Description: Add the new translation key to all locale files and
  ensure the settings serde round-trip treats `"mov"` as a valid
  value.
- Rationale: AGENTS.md critical rule — every i18next key must be
  mirrored across all 20 locales; settings files are user-authored
  JSON that survives upgrades.
- Acceptance Criteria
  - AC-002-a — `bun scripts/check-translations.ts` exits 0 after
    `settings.export.format.options.mov.{label,description}` is
    added to all 20 locale files under `src/i18n/locales/*/translation.json`.
  - AC-002-b — Round-trip test: serializing
    `AudioExportFormat::Mov` produces the JSON string `"mov"`, and
    deserializing `"mov"` produces `AudioExportFormat::Mov`.
    Verified by a new cargo test
    `export_format_mov_settings_roundtrip`.

### R-003 — Hardware-encoder contract (cross-bundle)

- Description: Container selection must be orthogonal to codec
  selection. The argv emitted by `build_export_args` for a given
  segment list, audio options, and `has_video=true` must differ
  between `Mp4` and `Mov` only in (a) the `-f` muxer flag and (b)
  the output extension. In particular, the `-c:v` / `-c:a` /
  `-b:a` / filter chain must be byte-identical.
- Rationale: `features/export-hardware-encoder/` will swap
  `-c:v libx264` for a hardware variant. If mov took a divergent
  codec path, that bundle would have to re-implement its encoder
  swap for every container.
- Acceptance Criteria
  - AC-003-a — A cargo test `export_format_mov_codec_parity`
    invokes `build_audio_only_export_args_for_tests` (or an
    equivalent video-path shim) with `AudioExportFormat::Mp4` and
    `AudioExportFormat::Mov` on identical inputs, then asserts the
    resulting argvs differ only in the `-f <mux>` tokens and the
    trailing output path. All `-c:*`, `-b:*`, `-vf`, `-af`,
    `-filter_complex`, and `-map` tokens are byte-identical.

## Edge cases & constraints

- FFmpeg today infers the muxer from extension; adding `-f mov`
  explicitly is defensive. Both the explicit flag and the extension
  must agree (the existing extension-swap at
  `src-tauri/src/commands/waveform/commands.rs:464` ensures this).
- Audio-only + Mov: selecting an audio-only variant
  (`Mp3`/`Wav`/`M4a`/`Opus`) dominates. The UI must only offer
  `Mov` when the effective render has video.
- 800-line cap: current size of `commands/waveform/mod.rs` is 704
  lines; this bundle adds < 20. `export_format.rs` is 189.
- No splice / keep-segment / time-mapping / caption / filler-list
  logic changes.
- No new crates, no new npm packages.

## Data model

```rust
#[derive(Serialize, Deserialize, Debug, Clone, Copy,
         PartialEq, Eq, Type, Default)]
#[serde(rename_all = "lowercase")]
pub enum AudioExportFormat {
    #[default]
    Mp4,
    Mov,      // NEW
    Mp3,
    Wav,
    M4a,
    Opus,
}
```

`AudioExportFormat::Mov.extension() == ".mov"`.
`AudioExportFormat::Mov.is_audio_only() == false`.
`export_format_codec_map(Mov) == None`.

## Non-functional requirements

- No runtime network calls (AGENTS.md critical rule).
- No duplication of dual-path logic; the frontend `EXPORT_FORMATS`
  array in `src/components/settings/advanced/ExportGroup.tsx:18` is
  acknowledged as an existing deviation and is extended in
  lock-step (see BLUEPRINT "Single-source-of-truth placement").
- No change to the caption / forced-alignment / loudness / seam-fade
  subsystems.
- File-size cap respected.

