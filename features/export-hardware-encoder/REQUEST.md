# Feature request: Hardware Encoder Selection + CPU Fallback

## 1. Problem & Goals

Closes PRD `product-map-v1` F5. Today video export is libx264-only
(`src-tauri/src/commands/waveform/mod.rs:522`), 3-10x slower than
hardware encoding on long-form material. Modern desktops have NVENC
(NVIDIA), QuickSync (Intel), VideoToolbox (Apple Silicon / Intel
macOS), or VAAPI (Linux discrete + iGPU). Pipeline must detect what
is locally available, let the user pick or auto-select, and gracefully
fall back to libx264 on encoder init failure.

Goal: detect available hardware encoders at startup, expose a picker
in the Export panel, and add a single safe-fallback wrapper so
encoder-init failures retry once with libx264 and surface a
non-blocking toast.

## 2. Desired Outcome & Acceptance Criteria

- At startup, FFmpeg is queried for available encoders; result is
  cached in app state.
- Export Settings has an "Encoder" Select with options: Auto, CPU
  (libx264), and the encoders detected on this machine (NVENC, QSV,
  VideoToolbox, VAAPI as applicable).
- A failed hardware-encode invocation retries automatically with
  libx264, and a non-blocking toast tells the user.
- Pixel-content parity: NVENC / QSV / VideoToolbox / VAAPI outputs
  are visually identical to libx264 within a per-frame PSNR
  threshold (not byte-identical).
- Audio path unchanged: `audio-boundary-eval` skill still passes.
- `cut-drift-fuzzer` agent still passes on a fixture export.

## 3. Scope Boundaries

### In scope

- Encoder detection helper (Rust, parses `ffmpeg -encoders` once at
  startup; cached on `AppHandle`).
- `Settings.video_encoder: VideoEncoder` enum.
- Encoder Select in `ExportSettings.tsx` filtered by detected list.
- Fallback wrapper around the FFmpeg invocation.
- Per-encoder argv mapping (codec name + quality flags equivalent to
  current libx264 quality).

### Out of scope (explicit)

- HEVC variants (h265 hardware encoders) — kept libx264 / h264 only.
- AV1 hardware encoders.
- Hardware decode (only encode is in scope).
- Per-encoder bitrate or rate-control tuning UI.

## 4. References to Existing Code

- `src-tauri/src/commands/waveform/mod.rs:522` — current `-c:v
  libx264` injection point.
- `src/components/settings/export/ExportSettings.tsx` — Bundle 1's
  panel; this bundle adds the Encoder Select.
- `eval/fixtures/toaster_example.mp4` — fuzz fixture.

## 5. Edge Cases & Constraints

- Detection must not block app launch >1 s; if `ffmpeg -encoders`
  fails, the app launches with CPU-only.
- A user with no detected hardware encoders sees only Auto and CPU.
- Fallback must run at most once per export (no infinite retry
  loop).
- Cross-platform: NVENC tested on Windows + Linux, VideoToolbox on
  macOS, VAAPI on Linux, QSV on Windows + Linux.
- ASCII-only changes; 800-line cap; no hosted inference.

## 6. Data Model (optional)

- `Settings.video_encoder: enum { Auto, Cpu, Nvenc, Qsv,
  VideoToolbox, Vaapi }`. Default `Auto`.
- `EncoderAvailability { libx264: true, nvenc: bool, qsv: bool,
  videotoolbox: bool, vaapi: bool }` cached in `AppHandle` state.

## Q&A

Pre-answered:

- Q: Detect at startup or per-export?
  - A: Startup, cached. Per-export adds latency; encoders rarely
    appear/disappear at runtime.
- Q: HW HEVC?
  - A: Out of scope. h264 only.
- Q: Fallback toast UX?
  - A: Non-blocking toast; export still succeeds via CPU.
- Q: Where does the Select live?
  - A: Same Export panel as Bundles 1/2.
