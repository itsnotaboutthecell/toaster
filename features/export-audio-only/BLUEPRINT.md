# Blueprint: Export Audio-Only Presets

## Architecture decisions

- **R-001 (format picker)**: extend
  `src/components/settings/export/ExportSettings.tsx` (created by
  Bundle 1) with a 5-option Select bound to `Settings.export_format`.
  No new panel. Same `useSettings` hook the loudness Select uses.
- **R-002 (codec/muxer mapping)**: new helper
  `export_format_codec_map(format: ExportFormat) -> CodecSpec` in
  `src-tauri/src/commands/waveform/mod.rs` (or a sibling module if
  the file approaches the 800-line cap; see Risk register). Returns
  the tuple (extension, `-c:a`, `-vn` flag, bitrate flag option).
- **R-003 (round-trip parity)**: new cargo test invokes the export
  pipeline four times against `eval/fixtures/toaster_example.mp4`
  and uses the existing ffprobe wrapper to read durations.
- **R-004 (precision)**: word timing is upstream of muxing; the only
  risk is encoder-introduced silence. Verified via the existing
  `transcript-precision-eval` skill.
- **R-005 (filter reuse)**: the audio filter chain
  (`build_audio_post_filters`,
  `src-tauri/src/commands/waveform/mod.rs:102`) is invoked unchanged
  for both video and audio-only paths. The video path additionally
  adds video-codec args; audio-only drops them.

## Component & module touch-list

| File | Change |
|------|--------|
| `src-tauri/src/settings/types.rs` | Add `export_format: ExportFormat` field. |
| `src-tauri/src/settings/defaults.rs` | Default = `ExportFormat::Mp4`. |
| `src-tauri/src/commands/waveform/mod.rs:494-527` | Branch on `settings.export_format`; route through `export_format_codec_map`; add `-vn` for audio-only; omit `-c:v`. |
| `src-tauri/src/commands/waveform/mod.rs` | Add `export_format_codec_map` + 2 unit tests. (Move to a sibling file if size cap pressures.) |
| `src/components/settings/export/ExportSettings.tsx` | Add Format Select, 5 options, persisted via `useSettings`. |
| `src/i18n/locales/*/translation.json` (20 files) | Add `settings.export.format.label`, `.video_mp4`, `.audio_mp3`, `.audio_wav`, `.audio_m4a`, `.audio_opus`. Use `i18n-pruning` skill. |
| `src/bindings.ts` | Auto-regenerated to include `ExportFormat`. |

## Single-source-of-truth placement

- **Audio filter chain**: `build_audio_post_filters` at
  `src-tauri/src/commands/waveform/mod.rs:102` is the authority.
  Consumers: the video export path and the audio-only export paths.
  No duplicate.
- **Codec/muxer mapping**: `export_format_codec_map` in
  `commands/waveform/mod.rs`. Frontend never builds an FFmpeg arg.
- **Loudness**: still owned by `splice::loudness` per Bundle 1; this
  bundle does not touch the loudness path.

## Data flow

```
User picks format in ExportSettings -> Settings.export_format
  |
  v
Export commit
  -> commands/waveform/mod.rs::build_export_args
       -> common: keep-segments, build_audio_post_filters (loudness, seams)
       -> branch on export_format:
            Mp4   -> video args (libx264 / hardware encoder per Bundle 3) + aac
            Mp3   -> -vn -c:a libmp3lame -b:a 192k
            Wav   -> -vn -c:a pcm_s16le
            M4a   -> -vn -c:a aac        -b:a 192k
            Opus  -> -vn -c:a libopus    -b:a 128k
       -> output extension from export_format_codec_map
```

## Migration / compatibility

- New field; default Mp4 preserves current behavior. No legacy
  migration needed.
- `bindings.ts` regenerates an `ExportFormat` enum.

## Sequencing & conflict-avoidance

- **Position**: bundle 2 of 5. Depends on Bundle 1 (`export-loudness`)
  for the Export panel scaffold. Do not re-design the panel here.
- **Files this bundle owns**: `Settings.export_format` field; the
  codec mapping helper + branch in `commands/waveform/mod.rs`; the
  Format Select inside `ExportSettings.tsx`.
- **Files this bundle agrees not to touch**: anything Bundle 1 owns
  (loudness math, sidebar entry); the encoder branch Bundle 3 will
  own (`-c:v` selection); `tauri.conf.json` (Bundle 4 owns).
- **Downstream**: Bundle 3 adds the encoder picker to the same panel
  and modifies the same `commands/waveform/mod.rs` video branch.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Engineer forks the audio filter chain into an "audio-only" copy (dual-path defect) | AC-005-a grep limits the function to one call site + one definition | AC-005-a |
| `commands/waveform/mod.rs` exceeds 800-line cap after additions | Move codec map and unit tests to a new sibling module if needed; add to allowlist only as last resort | (size linter; reviewer) |
| Encoder pads silence at start/end (m4a/aac is the usual culprit) | AC-003-a duration parity within 30 ms catches it | AC-003-a |
| File extension UI mismatch (user types "out.mp4" for mp3 export) | UX guidance only; auto-suggest extension; no AC required | n/a |
| Loudness regressions creep in via filter-chain edits | This bundle does not edit the loudness path; Bundle 1's AC-005-a (`audio-boundary-eval`) was the gate | (Bundle 1 covers) |
| Word-timing drift on audio-only path | `transcript-precision-eval` skill | AC-004-a |
