# PRD: Hardware Encoder Selection + CPU Fallback

## Problem & Goals

Video export is currently locked to libx264
(`src-tauri/src/commands/waveform/mod.rs:522`), which is 3-10x slower
than hardware encoders on long-form material. This bundle adds
detection, selection, and safe-fallback wrappers around FFmpeg's h264
encoders: NVENC, QuickSync (QSV), VideoToolbox, VAAPI. CPU (libx264)
remains the universal fallback.

## Scope

### In scope

- Encoder detection at startup (cached).
- `Settings.video_encoder` enum.
- Encoder Select in the Export panel filtered by detected list.
- Fallback wrapper that retries libx264 once on encoder init
  failure and surfaces a non-blocking toast.

### Out of scope (explicit)

- HEVC / h265 / AV1 hardware encoders.
- Hardware decode.
- Per-encoder bitrate/rate-control UI.
- Encoder benchmarking UI.

## Requirements

### R-001 — Encoder detection at startup

- Description: a Rust helper invokes `ffmpeg -encoders` once at app
  init, parses the output, and caches an `EncoderAvailability`
  struct on `AppHandle` state.
- Acceptance Criteria
  - AC-001-a — `cargo test parse_ffmpeg_encoders_output` confirms
    the parser correctly identifies `h264_nvenc`, `h264_qsv`,
    `h264_videotoolbox`, `h264_vaapi`, and `libx264` from a fixture
    `ffmpeg -encoders` output snapshot bundled with the test.
  - AC-001-b — In the live app, opening Settings -> Export shows
    the Encoder Select populated with Auto + CPU + the encoders
    available on the current machine; on a Windows machine without
    NVIDIA hardware, NVENC is not listed.
  - AC-001-c — App launch is not delayed by more than 1 s due to
    encoder detection; if `ffmpeg -encoders` fails, the app still
    launches and the Select shows only Auto + CPU.

### R-002 — Encoder picker

- Description: `Settings.video_encoder` Select in Export panel.
  Auto means the backend chooses (highest-priority detected encoder:
  NVENC > QSV > VideoToolbox > VAAPI > libx264). CPU means libx264
  unconditionally.
- Acceptance Criteria
  - AC-002-a — In the live app, switching the Select to NVENC and
    exporting a 10 s video fixture produces an mp4 whose
    `ffprobe -hide_banner -show_streams` reports `codec_name=h264`
    and the video stream `encoder` tag references nvenc.
  - AC-002-b — In the live app, switching to CPU and exporting the
    same fixture produces an mp4 whose stream `encoder` tag
    references libx264.

### R-003 — Safe fallback to libx264

- Description: a single wrapper around the FFmpeg invocation
  detects encoder-init failure (non-zero exit + recognized error
  signatures) and retries once with libx264. A non-blocking toast
  ("Hardware encoder failed; used CPU instead") is shown.
- Acceptance Criteria
  - AC-003-a — `cargo test hardware_encoder_fallback` simulates an
    encoder failure (using a stub command path or injected error)
    and confirms the wrapper invokes libx264 on the second attempt
    and returns success.
  - AC-003-b — `cargo test hardware_encoder_fallback_no_loop`
    confirms that if the libx264 fallback also fails, the wrapper
    returns the error without a third attempt (no retry loop).
  - AC-003-c — In the live app, simulating a NVENC failure (e.g.
    by selecting NVENC in a clean VM with no GPU drivers) shows a
    non-blocking toast and the export succeeds via libx264.

### R-004 — Cut-drift-fuzzer + boundary-eval still pass

- Description: switching encoders must not affect time mapping or
  audio seam handling.
- Acceptance Criteria
  - AC-004-a — `cut-drift-fuzzer` agent passes on
    `eval/fixtures/toaster_example.mp4` exported with a hardware
    encoder.
  - AC-004-b — `audio-boundary-eval` skill passes (audio path
    untouched).

### R-005 — SSOT: encoder choice math lives once in Rust

- Description: the Auto-priority order, the codec-name mapping, and
  the fallback decision all live in one Rust module. Frontend reads
  the enum and the cached availability list; never decides.
- Acceptance Criteria
  - AC-005-a — `rg "h264_nvenc|h264_qsv|h264_videotoolbox|h264_vaapi"
    src` returns zero matches; the strings exist only in Rust.
  - AC-005-b — `BLUEPRINT.md` "Single-source-of-truth placement"
    section names the Rust authority module and lists the frontend
    as a passive consumer.

## Edge cases & constraints

- A user with detected NVENC but a driver-mismatch at export time
  hits R-003 fallback; toast informs but does not block.
- macOS Apple Silicon: VideoToolbox is the only HW encoder; NVENC,
  QSV, VAAPI absent from Select.
- Linux without VAAPI drivers: only Auto + CPU.
- Detection runs once per app session.
- ASCII-only; 800-line cap; no hosted inference.

## Data model (if applicable)

- `Settings.video_encoder: enum { Auto, Cpu, Nvenc, Qsv,
  VideoToolbox, Vaapi }` serialized lowercase. Default `Auto`.
- `EncoderAvailability { nvenc: bool, qsv: bool, videotoolbox: bool,
  vaapi: bool }` (libx264 always true, omitted from struct).

## Non-functional requirements

- AGENTS.md "Single source of truth for dual-path logic" via R-005.
- AGENTS.md "Local-only inference" — encoder detection is local.
- AGENTS.md "Verified means the live app, not `cargo check`" — R-002
  and R-003 include live-app ACs.
