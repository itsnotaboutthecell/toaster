# PRD: Export Loudness (preflight + loudnorm)

## Problem & Goals

Toaster ships an FFmpeg-based export pipeline that gates loudness
normalization on a single boolean (`normalize_audio_on_export`,
`src-tauri/src/settings/types.rs:262`) and unconditionally injects
`loudnorm=I=-16:TP=-1.5:LRA=11`
(`src-tauri/src/commands/waveform/mod.rs:121`) when that boolean is
true. Users have:

- no readout of source loudness (LUFS / dBTP / LRA),
- no choice of target,
- no warning when the source is already clipping or far off target.

The deterministic EBU R128 measurement infra
(`src-tauri/src/managers/splice/loudness.rs:25-40`) is fully
implemented but unused at the export surface.

Goal: surface a preflight readout in the export dialog and a
target-LUFS picker; route both the readout and the `loudnorm` filter
parameters through the same Rust authority so frontend and backend
cannot drift.

## Scope

### In scope

- New `ExportSettings` panel + sidebar entry (`sidebar.export`).
- `loudness_preflight` Tauri command (analysis-only) that invokes
  `splice::loudness::measure_loudness` over the post-edit audio.
- Export dialog preflight section: integrated LUFS, true-peak dBTP,
  and LRA, plus an off-target warning at >12 LU.
- Target picker: Off / Podcast (-16 LUFS) / Streaming (-14 LUFS).
- Migration: existing `normalize_audio_on_export = true` becomes
  `loudness_target = "podcast_-16"`.

### Out of scope (explicit)

- Live `ebur128` panel in the editor (PRD `product-map-v1` F12).
- Per-segment loudness control.
- Replacing `loudnorm` with offline gain (open question Q2 in
  `product-map-v1`).
- Any hosted-inference call.

## Requirements

### R-001 — Export Settings panel scaffold

- Description: a new top-level Settings panel "Export" hosts all
  export-pipeline knobs. This bundle establishes the panel; Bundles 2
  (audio-only formats) and 3 (hardware encoders) extend it.
- Rationale: keeps Models / Editor panels focused; the Export pile-on
  is foreseeable.
- Acceptance Criteria
  - AC-001-a — In the live app, Settings has an "Export" entry in the
    sidebar (i18n key `sidebar.export`); clicking it opens an
    ExportSettings panel.
  - AC-001-b — `BLUEPRINT.md` documents the panel placement decision
    in the "Sequencing & conflict-avoidance" section.

### R-002 — Loudness preflight (analysis-only)

- Description: before commit, the export dialog can run an
  analysis-only EBU R128 pass and render integrated LUFS, true-peak
  dBTP, and LRA. Source: post-edit audio (after keep-segments).
- Rationale: users need to know what the export will sound like.
- Acceptance Criteria
  - AC-002-a — A new Tauri command `loudness_preflight` exists and
    returns `{ integrated_lufs, true_peak_dbtp, lra, target_lufs? }`
    by delegating to `splice::loudness::measure_loudness`. Verified by
    a `cargo test` round-trip on `eval/fixtures/toaster_example.mp4`.
  - AC-002-b — In the live app, opening the export dialog and
    clicking "Run preflight" shows three numeric readouts (LUFS,
    dBTP, LRA) within 5 s on a 60 s fixture.
  - AC-002-c — In the live app, when the measured integrated LUFS is
    >12 LU off the selected target, a non-blocking warning appears in
    the dialog. Export remains enabled.

### R-003 — Target picker + loudnorm parameter authority

- Description: a `loudness_target` setting takes one of `"off"`,
  `"podcast_-16"`, `"streaming_-14"`. The `loudnorm=...` filter string
  is constructed by a Rust helper that reads `loudness_target` and
  emits the matching `I=` value. The frontend never builds the filter
  string nor re-derives parameters.
- Rationale: AGENTS.md "Single source of truth for dual-path logic".
  Two places computing target LUFS is the defect class to prevent.
- Acceptance Criteria
  - AC-003-a — `cargo test` confirms a Rust unit
    `build_loudnorm_filter` returns `"loudnorm=I=-16:TP=-1.5:LRA=11"`
    for `podcast_-16`, `"loudnorm=I=-14:TP=-1.5:LRA=11"` for
    `streaming_-14`, and `None` for `off`.
  - AC-003-b — `rg "loudnorm=" src/` returns zero matches: no string
    is built in TS.
  - AC-003-c — In the live app, switching the target in Export
    Settings to Streaming and exporting a 60 s fixture produces a
    file whose post-export integrated LUFS is within 1.0 LU of -14.

### R-004 — Migration: legacy boolean -> enum

- Description: on first launch after upgrade, a stored
  `normalize_audio_on_export = true` becomes
  `loudness_target = "podcast_-16"`; `false` becomes `"off"`. The
  legacy field is then ignored (kept for one release for downgrade
  safety).
- Acceptance Criteria
  - AC-004-a — `cargo test` confirms a `migrate_loudness_setting`
    helper maps `(true, _) -> "podcast_-16"`, `(false, _) -> "off"`,
    `(absent, present) -> present`.
  - AC-004-b — In the live app, after upgrading a settings file with
    `normalize_audio_on_export: true`, the Export panel shows
    "Podcast (-16 LUFS)" selected on first open.

### R-005 — Boundary-eval still passes

- Description: the filter-chain refactor must not regress seam
  handling.
- Acceptance Criteria
  - AC-005-a — `audio-boundary-eval` skill returns pass on
    `eval/fixtures/toaster_example.mp4` after this bundle merges.

### R-006 — Backend authority for loudness math

- Description: no LUFS / dBTP / LRA arithmetic in TS. Frontend reads
  numbers from the Rust DTO and renders them.
- Acceptance Criteria
  - AC-006-a — `rg "ebur128|integrated_lufs|truePeak|true_peak|LRA"
    src/` returns matches only inside type definitions / display
    formatters that consume the DTO; no arithmetic operator is
    applied to those fields outside formatting.
  - AC-006-b — `BLUEPRINT.md` "Single-source-of-truth placement"
    section names `splice::loudness` as the authority and the new
    React readout component as the consumer.

## Edge cases & constraints

- Preflight on a 30+ minute file may exceed 30 s; UI must show a
  spinner and remain cancellable. AC covered by R-002 timing on the
  60 s fixture (extrapolation only).
- A source already at the target LUFS still goes through `loudnorm`
  (current behavior); that is intentional — `loudnorm` also gates
  true-peak and LRA.
- ASCII-only source changes.
- 800-line cap per `.rs` / `.tsx`.

## Data model (if applicable)

- `Settings.loudness_target: enum { Off, Podcast_-16, Streaming_-14 }`
  serialized as `"off" | "podcast_-16" | "streaming_-14"`.
- DTO `LoudnessPreflight { integrated_lufs: f64, true_peak_dbtp: f64,
  lra: f64, target_lufs: Option<f64> }` exported via `bindings.ts`.

## Non-functional requirements

- AGENTS.md "Verified means the live app, not `cargo check`": every
  R-NNN has at least one live-app or fixture-based AC.
- AGENTS.md "Local-only inference": preflight runs in-process via the
  `ebur128` crate; no network call.
- AGENTS.md "Single source of truth for dual-path logic": R-003 and
  R-006 enforce this by source-tree assertion plus cargo test.
