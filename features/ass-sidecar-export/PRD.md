# PRD: ass sidecar export

## Problem & Goals

Toaster already generates an authoritative ASS document for the
burn-in export path
(`src-tauri/src/managers/captions/ass.rs::blocks_to_ass`, called from
`src-tauri/src/commands/waveform/commands.rs:430-443`). That document
is never offered to the user as a file. Users who want to hand the
generated captions to Aegisub, Subtitle Edit, or an external FFmpeg
pipeline currently have no path to the exact ASS Toaster uses
internally.

Parent: `features/product-map-v1/PRD.md` §6 F11 / §6 SR5 / §10 M2.5.

Goal: a single opt-in checkbox in the export toolbar that, when
enabled, writes the exact same ASS document Toaster would otherwise
feed to `libass` as a `<basename>.ass` sidecar next to the exported
media — with zero duplication of caption layout or ASS serialization
logic.

## Scope

### In scope

- New `AppSettings.export_ass_sidecar_enabled: bool` (default `false`)
  with typed setter command, registered in `lib.rs`.
- New toggle in the editor toolbar, wired to the new setting.
- Sidecar write inside `export_edited_media` that reuses the ASS
  document built for the burn path when both flags are on, and builds
  it once on its own when only sidecar is on.
- New i18n key `editor.saveAssSidecar` in all 20 locales.
- A small refactor extracting "build the ASS document for this
  export" into a single function, consumed at most once per export.
- One Rust unit test asserting single-site generation (AC-003-a).

### Out of scope (explicit)

- SRT / VTT / script sidecars (already shipped).
- Changes to `blocks_to_ass`, `CaptionBlock`, or layout geometry.
- A standalone "export just the ASS, no media" command.
- Settings panel UI beyond the toolbar toggle.
- Conflict-resolution dialog (overwrite silently per user decision).
- Bulk "export all sidecars" flow.

## Requirements

### R-001 — Persist a new `export_ass_sidecar_enabled` boolean

- Description: Add an `export_ass_sidecar_enabled: bool` field to
  `AppSettings` with a `#[serde(default)]` attribute and a `false`
  default in `settings::defaults::default_settings()`. Add a typed
  setter command `change_export_ass_sidecar_setting(app, enabled)`
  that mirrors `change_normalize_audio_setting`
  (`src-tauri/src/commands/app_settings.rs:487-492`) and register it
  in `src-tauri/src/lib.rs` alongside the other `change_*_setting`
  handlers.
- Rationale: Per user decision (REQUEST §Q&A), the checkbox state is
  remembered across app restarts. The typed-handler pattern is the
  convention for all boolean export settings.
- Acceptance Criteria
  - AC-001-a — `AppSettings` contains an `export_ass_sidecar_enabled:
    bool` field defaulting to `false`, the typed setter command
    `change_export_ass_sidecar_setting` writes the new value through
    `settings::write_settings`, and a unit test round-trips the field
    through `serde_json::to_string` / `from_str` showing that `true`
    persists and that a legacy JSON payload missing the field loads as
    `false`.

### R-002 — Checkbox in the export toolbar

- Description: Add a toolbar toggle adjacent to the existing burn
  captions toggle at `src/components/editor/EditorView.tsx:538-548`.
  The toggle reads / writes `export_ass_sidecar_enabled` through the
  typed setter command. Labeled by a new i18n key
  `editor.saveAssSidecar` present in all 20 locales under
  `src/i18n/locales/*/translation.json`. English label: "Also save
  .ass subtitle sidecar".
- Rationale: Two lines of UI, per the product map.
- Acceptance Criteria
  - AC-002-a — The toolbar renders the new toggle next to the burn
    captions toggle whenever the burn captions toggle is rendered
    (same video/has-video gating). Toggling it calls
    `commands.changeExportAssSidecarSetting` and re-reads
    `AppSettings` so the state survives app restart.
  - AC-002-b — `scripts/check-translations.ts` passes with the new
    `editor.saveAssSidecar` key present in every file under
    `src/i18n/locales/*/translation.json`.

### R-003 — Sidecar is byte-identical to the burn-in input

- Description: When the export runs and `export_ass_sidecar_enabled`
  is `true`, write the exact ASS document produced by
  `managers::captions::blocks_to_ass` for this export to
  `<output_path with extension replaced by "ass">`. When both the
  sidecar and burn flags are on, the sidecar and the temp file handed
  to FFmpeg's `subtitles=` filter MUST be byte-for-byte identical
  (same string, same trailing-newline policy, same encoding). When
  only the sidecar is on, the ASS document is built once and written
  once — no FFmpeg invocation is added for the sidecar.
- Rationale: Single source of truth (AGENTS.md).
- Acceptance Criteria
  - AC-003-a — A Rust unit/integration test in
    `src-tauri/src/commands/waveform/tests/` builds a canned
    `[CaptionBlock]`, asserts that `blocks_to_ass` is invoked **at
    most once** per export-plan-flag-combination via a helper
    `build_export_ass_doc` (i.e. the test imports the helper and calls
    it; production code MUST route both the burn path and the sidecar
    path through that single helper). The test also fails if a `grep`
    for `blocks_to_ass` inside `src-tauri/src/commands/waveform/`
    yields more than one call site.
  - AC-003-b — Manual fixture run (live app) with both burn and
    sidecar enabled against
    `eval/fixtures/toaster_example.mp4` produces a `<basename>.ass`
    sidecar whose bytes equal the bytes FFmpeg received
    (captured by reading the `*.burn_captions.ass` temp before FFmpeg
    is invoked, or by instrumenting the export to keep the temp
    around for this check).

### R-004 — Orthogonality with the burn flag

- Description: The sidecar flag and the burn flag are independent.
  The four combinations (burn off / sidecar off), (burn off / sidecar
  on), (burn on / sidecar off), (burn on / sidecar on) each behave as
  the combination implies.
- Rationale: User requirement from REQUEST §Scope.
- Acceptance Criteria
  - AC-004-a — Manual fixture run: burn off, sidecar off → no
    `<basename>.ass` file written, output media has no burned
    captions. No stray `.ass` files appear in the target directory
    (including no residual `.burn_captions.ass` temp).
  - AC-004-b — Manual fixture run: burn on, sidecar on → output video
    has burned captions AND a `<basename>.ass` file sits next to it.
    The sidecar contains the same `[Script Info]`, `[V4+ Styles]`, and
    `[Events]` sections that were fed to FFmpeg.
  - AC-004-c — Manual fixture run: burn off, sidecar on, video input →
    no burned captions in output media, but a `<basename>.ass` file
    is written. No FFmpeg `subtitles=` filter is invoked (grep
    export's logged FFmpeg command line).
  - AC-004-d — Manual fixture run: audio-only export format with
    sidecar on → no `<basename>.ass` file written (audio-only gate,
    per REQUEST §5). No error; the toggle is silently ignored when
    the chosen `export_format.is_audio_only()` is true.

## Edge cases & constraints

- Empty caption stream: `blocks_to_ass` returns a valid
  header-only document (`ass.rs:246-252`). Sidecar still writes that
  document — users expect a file out if they ticked the box.
- Input has no video (`has_video == false`): both burn and sidecar are
  suppressed. Matches existing burn gating.
- Write errors (permission denied, disk full) propagate from
  `std::fs::write` back to the frontend as a `Result::Err(String)`,
  matching the existing burn-temp error handling at
  `waveform/commands.rs:439`.
- File conflict: overwrite silently. Covered in REQUEST §Q&A.
- 800-line cap on this file: current length ≪ 800 lines.

## Data model

New field on `AppSettings`:

```rust
#[serde(default)]
pub export_ass_sidecar_enabled: bool,
```

Default: `false`.

No migrations. `#[serde(default)]` handles older settings.json.

## Non-functional requirements

- Local-only: sidecar is `std::fs::write`; no network, no hosted
  inference. Complies with AGENTS.md.
- No new FFmpeg / ffprobe invocation for the sidecar.
- i18n parity across 20 locales.
- No new Rust crate or npm package (dep-hygiene).
- Zero duplication of `blocks_to_ass` call sites within
  `commands/waveform/` (enforced by AC-003-a).
