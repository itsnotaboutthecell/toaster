# Feature request: Export Audio-Only Presets

## 1. Problem & Goals

Closes PRD `product-map-v1` F4c. Today export is mp4/H.264/AAC only
(`src-tauri/src/commands/waveform/mod.rs:494-527`). Podcasters and
audio-first users round-trip through Audacity to get mp3 / wav / m4a /
opus. The edit/keep-segment pipeline already produces the post-edit
audio stream we'd need; we just need to swap muxer + codec and drop
the video stream when the user picks an audio-only preset.

Goal: add audio-only export presets (mp3, wav, m4a, opus) selectable
from a format dropdown in the Export panel created by
`export-loudness`. Same edit pipeline; same loudness authority; just a
different muxer/codec output.

## 2. Desired Outcome & Acceptance Criteria

- A format Select in the Export panel offers: Video (mp4) [default],
  Audio - mp3, Audio - wav, Audio - m4a, Audio - opus.
- For audio-only formats, FFmpeg is invoked without a video stream
  (`-vn`) and with the appropriate codec/container.
- Per-word timing is preserved across audio-only render
  (`transcript-precision-eval` skill passes).
- A `cargo test` round-trip exports `eval/fixtures/toaster_example.mp4`
  to each of the four formats and re-decodes each to a duration within
  +-30 ms of the post-edit duration.

## 3. Scope Boundaries

### In scope

- New `export_format: ExportFormat` setting.
- Format Select in `ExportSettings.tsx` (panel from Bundle 1).
- Codec/muxer mapping helper in
  `src-tauri/src/commands/waveform/mod.rs`.
- Bitrate defaults: mp3 192 kbps, m4a 192 kbps, opus 128 kbps; wav
  pcm_s16le (no bitrate).
- All current edit logic (keep-segments, seam fades, loudness)
  continues to apply.

### Out of scope (explicit)

- flac (not in PRD F4c list as scoped here).
- Per-channel layout control.
- Video-only or video-without-audio export.
- Re-encode quality presets (single sensible default per format).

## 4. References to Existing Code

- `src-tauri/src/commands/waveform/mod.rs:494-527` — current
  audio/video codec selection (`aac`, `libx264`).
- `src-tauri/src/commands/waveform/mod.rs:102` —
  `build_audio_post_filters` (loudness + seam path; reused as-is).
- `src/components/settings/export/ExportSettings.tsx` — created by
  Bundle 1, extended here.
- `eval/fixtures/toaster_example.mp4` — round-trip fixture.

## 5. Edge Cases & Constraints

- An audio-only export from a video source must drop video cleanly
  (`-vn`) and not re-encode video.
- mp3 has no native chapter/cuepoint metadata; we do not add cues.
- m4a (aac in mp4 container, audio-only) must validate as an audio
  file in players that distinguish audio.mp4 vs video.mp4.
- ASCII-only changes; 800-line cap per file.
- No hosted inference.

## 6. Data Model (optional)

- `Settings.export_format: enum { Mp4, Mp3, Wav, M4a, Opus }`,
  serialized lowercase. Default `Mp4`.

## Q&A

Pre-answered:

- Q: Which formats?
  - A: mp3 (libmp3lame), wav (pcm_s16le), m4a (aac), opus (libopus).
- Q: Bitrate UI?
  - A: No. Sensible defaults per format documented in BLUEPRINT.
- Q: Where does the picker live?
  - A: Same Export panel as Bundle 1; do not redesign the panel.
- Q: Does loudness apply to audio-only?
  - A: Yes; `build_audio_post_filters` is reused unchanged.
