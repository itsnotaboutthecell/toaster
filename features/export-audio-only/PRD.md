# PRD: Export Audio-Only Presets

## Problem & Goals

Today export is mp4/H.264/AAC only. Podcasters and audio-first users
have no first-class way to export the post-edit audio as mp3, wav,
m4a, or opus. The edit pipeline (keep-segments, seam fades, loudness
filter chain) already produces the audio stream; the gap is in the
muxer/codec selection at
`src-tauri/src/commands/waveform/mod.rs:494-527`.

This bundle adds four audio-only presets and a format Select inside
the Export panel created by Bundle 1 (`export-loudness`).

## Scope

### In scope

- `Settings.export_format` enum.
- Format Select in `ExportSettings.tsx`.
- Codec/muxer mapping helper in `commands/waveform/mod.rs`.
- Bitrate defaults: mp3 192 kbps, m4a 192 kbps, opus 128 kbps; wav
  pcm_s16le.
- Audio-only invocation drops the video stream (`-vn`).

### Out of scope (explicit)

- flac.
- Per-format quality picker.
- Video-without-audio export.
- Re-architecting the Export panel (Bundle 1 owns the scaffold).

## Requirements

### R-001 — Format picker in Export panel

- Description: a Select in `ExportSettings.tsx` offers Video (mp4)
  [default], Audio - mp3, Audio - wav, Audio - m4a, Audio - opus.
  Persisted as `Settings.export_format`.
- Rationale: discoverable, single picker, no duplicate dialog state.
- Acceptance Criteria
  - AC-001-a — In the live app, opening Settings -> Export shows a
    Format Select with exactly five options in the order above; the
    default selected option is "Video (mp4)".
  - AC-001-b — Changing the Format Select and reopening the panel
    shows the new selection persisted.

### R-002 — Audio-only render uses correct codec / muxer / extension

- Description: backend codec/muxer mapping helper produces, for each
  format: container extension, FFmpeg `-c:a` codec, audio-only flag
  (`-vn`), bitrate flag where applicable.
- Acceptance Criteria
  - AC-002-a — `cargo test export_format_codec_map` confirms the
    mapping: mp3 -> (".mp3", "libmp3lame", -vn, "-b:a 192k"); wav ->
    (".wav", "pcm_s16le", -vn, none); m4a -> (".m4a", "aac", -vn,
    "-b:a 192k"); opus -> (".opus", "libopus", -vn, "-b:a 128k").
  - AC-002-b — `cargo test export_format_args_no_video_stream`
    confirms that for any audio-only format the constructed argv
    contains `-vn` and contains no `-c:v` flag.

### R-003 — Round-trip duration parity for all formats

- Description: a single fixture exported to each of the four
  audio-only formats re-decodes to a duration within +-30 ms of the
  post-edit duration. Validates muxer/codec choice does not introduce
  silent padding or truncate the tail.
- Acceptance Criteria
  - AC-003-a — `cargo test audio_only_roundtrip_durations` exports
    `eval/fixtures/toaster_example.mp4` to mp3, wav, m4a, and opus
    in turn, decodes each via `ffprobe`, and asserts each duration
    is within 30 ms of the expected post-edit duration.

### R-004 — Word-timing precision survives audio-only render

- Description: the transcript-precision-eval skill passes after this
  bundle merges. Per-word timing must be preserved (no equal-duration
  synthesis, no midstream drift) when only the audio path runs.
- Acceptance Criteria
  - AC-004-a — `transcript-precision-eval` skill returns pass on
    `eval/fixtures/toaster_example.mp4`.

### R-005 — Loudness path reused; no duplicated audio filter code

- Description: `build_audio_post_filters` (loudness + seam fades) is
  invoked unchanged for audio-only renders. No copy of the filter
  chain.
- Acceptance Criteria
  - AC-005-a — `rg "build_audio_post_filters" src-tauri/src` shows
    exactly one definition site and exactly one call site (the
    existing one); audio-only does not introduce a parallel chain.
  - AC-005-b — `BLUEPRINT.md` "Single-source-of-truth placement"
    section names `build_audio_post_filters` as the audio-filter
    authority, consumed by both video and audio-only paths.

## Edge cases & constraints

- mp4 source with no audio track: audio-only export errors gracefully
  with a user-facing toast; not covered by an AC in this bundle (out
  of scope).
- Extension/format mismatch (e.g. user picks "out.mp4" filename for
  mp3 export): UI auto-suggests the extension matching the format.
- ASCII-only; 800-line cap.

## Data model (if applicable)

- `Settings.export_format: enum { Mp4, Mp3, Wav, M4a, Opus }`,
  serialized as `"mp4" | "mp3" | "wav" | "m4a" | "opus"`. Default
  `Mp4`.

## Non-functional requirements

- AGENTS.md "Single source of truth for dual-path logic" via R-005.
- AGENTS.md "Local-only inference" — no change.
- AGENTS.md "Verified means the live app" — R-001 AC-001-a/b are
  live-app, R-003 AC-003-a is fixture-based.
