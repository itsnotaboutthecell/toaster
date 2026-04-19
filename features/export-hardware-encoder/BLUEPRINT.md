# Blueprint: Hardware Encoder Selection + CPU Fallback

## Architecture decisions

- **R-001 (detection)**: new module
  `src-tauri/src/managers/export/encoders.rs` (or sibling to the
  waveform commands if managers/export/ doesn't exist yet — decided
  during execution; lean toward creating `managers/export/` so
  encoder + future export concerns get a stable home). Function
  `detect_encoders(ffmpeg_path: &Path) -> EncoderAvailability` runs
  `ffmpeg -encoders` with a 1 s timeout, parses the table, returns
  the struct. Called once from app init, stored on `AppHandle`.
- **R-002 (picker)**: extend
  `src/components/settings/export/ExportSettings.tsx` (Bundle 1's
  panel) with an Encoder Select. Reads `Settings.video_encoder` and
  the cached availability list (returned by a Tauri command
  `get_encoder_availability`).
- **R-003 (fallback)**: new wrapper
  `run_export_with_fallback(args_for_encoder, fallback_args) ->
  Result<...>` in `managers/export/` that runs FFmpeg, classifies
  errors via known stderr signatures, retries with libx264 once,
  returns. Toast emitted from the Tauri command via the existing
  toast event channel.
- **R-004 (parity)**: encoder switching is purely the `-c:v` arg.
  Audio chain and time mapping are unchanged.
- **R-005 (SSOT)**: all codec-name strings live in
  `managers/export/encoders.rs` (a `codec_for(encoder, role) ->
  &'static str` function). Frontend reads enums; never builds a
  codec string.

## Component & module touch-list

| File | Change |
|------|--------|
| `src-tauri/src/managers/export/mod.rs` (new) | Module root. |
| `src-tauri/src/managers/export/encoders.rs` (new) | `detect_encoders`, `EncoderAvailability`, `codec_for`, parser tests. |
| `src-tauri/src/managers/export/run.rs` (new) | `run_export_with_fallback` wrapper + tests. |
| `src-tauri/src/commands/waveform/mod.rs:522` | Replace `-c:v libx264` with `codec_for(resolve_encoder(settings.video_encoder, availability))`. Route through `run_export_with_fallback`. |
| `src-tauri/src/settings/types.rs` | Add `video_encoder: VideoEncoder`. |
| `src-tauri/src/settings/defaults.rs` | Default = `VideoEncoder::Auto`. |
| `src-tauri/src/lib.rs` | Run `detect_encoders` at app init; store on AppHandle; register `get_encoder_availability` command. |
| `src/components/settings/export/ExportSettings.tsx` | Add Encoder Select; query availability via Tauri. |
| `src/i18n/locales/*/translation.json` (20 files) | Add `settings.export.encoder.*` keys + the fallback toast string. Use `i18n-pruning` skill. |

## Single-source-of-truth placement

- **Encoder selection math + codec strings**:
  `src-tauri/src/managers/export/encoders.rs` is the authority.
  Frontend consumers: Encoder Select (reads enum + availability),
  the toast message wiring (reads a kind code, not a string built in
  TS). Backend consumer: `commands/waveform/mod.rs` via
  `codec_for(...)` and `run_export_with_fallback`.
- **Audio path**: unchanged; remains owned by Bundles 1/2.
- **Time mapping / keep-segments**: unchanged; backend authority
  remains `managers/editor/` per AGENTS.md.

## Data flow

```
App init
  -> detect_encoders(ffmpeg_path)  [1 s timeout]
  -> EncoderAvailability cached on AppHandle

User opens Settings -> Export
  -> get_encoder_availability command
  -> Encoder Select shows Auto + CPU + (nvenc|qsv|videotoolbox|vaapi)

User exports
  -> resolve_encoder(settings.video_encoder, availability) -> chosen encoder
  -> codec_for(chosen, Video) -> "-c:v" string
  -> run_export_with_fallback(primary_args, libx264_args)
       -> first attempt
       -> on encoder-init failure: retry with libx264 + emit toast event
       -> on second failure: return error
```

## Migration / compatibility

- New field; default `Auto` preserves current behavior on machines
  without HW encoders (Auto picks libx264 when nothing else is
  detected).

## Sequencing & conflict-avoidance

- **Position**: bundle 3 of 5. Depends on Bundle 1
  (`export-loudness`) for the panel scaffold; sequence after Bundle
  2 (`export-audio-only`) so the format/encoder branches are
  composed in a known order.
- **Files this bundle owns**: new `managers/export/` module; the
  `-c:v` branch in `commands/waveform/mod.rs`; the Encoder Select
  in `ExportSettings.tsx`.
- **Files this bundle agrees not to touch**: anything Bundle 1 owns
  (loudness math, sidebar, Format Select); anything Bundle 2 owns
  (codec/muxer mapping for audio-only; the audio filter chain);
  `tauri.conf.json` (Bundle 4 owns); the post-processing UI tree
  (Bundle 5 owns); the experimental panel (Bundle 5 owns); any
  Handy-era file.
- **Downstream**: Bundle 4 (signing) depends on the export pipeline
  being feature-complete so signed installers ship the full export
  surface.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| `ffmpeg -encoders` parser misses an encoder due to FFmpeg version drift | Parser is a fixture-based test; can pin a snapshot per FFmpeg major | AC-001-a |
| App launch hangs when ffmpeg binary is missing or slow | 1 s timeout in `detect_encoders`; fallback to "no HW detected" | AC-001-c |
| Hardware encoder produces a stream that breaks downstream re-encode | Audio path untouched (AC-004-b); video parity by visual QC; deeper bit-content drift would be caught by `cut-drift-fuzzer` | AC-004-a |
| Fallback retry loop on persistent failure | Explicit one-retry contract + `cargo test hardware_encoder_fallback_no_loop` | AC-003-b |
| TS hand-builds a codec string and drifts from Rust | AC-005-a is a literal grep | AC-005-a |
| `commands/waveform/mod.rs` size cap exceeded | New code lives in `managers/export/`; the file only loses the libx264 literal and gains a function call | (size linter) |
| HEVC/AV1 scope creep into this bundle | PRD out-of-scope is explicit; reviewer enforces | (architectural) |
