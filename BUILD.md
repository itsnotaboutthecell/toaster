# Build Instructions

This guide sets up and builds **Toaster** (Tauri + Rust + React).

## Prerequisites

### All platforms

- Rust (stable)
- Node.js (v18+) with npm (Bun optional for utility scripts)
- Tauri prerequisites for your OS
- CMake

### Windows

- Visual Studio 2022 Build Tools (C++ workload)
- LLVM (`winget install LLVM.LLVM`)
- Vulkan SDK (`winget install KhronosGroup.VulkanSDK`)
- Ninja (`winget install Ninja-build.Ninja`)

## Setup

### 1. Clone

```bash
git clone https://github.com/itsnotaboutthecell/toaster.git
cd toaster
```

### 2. Install frontend dependencies

```bash
npm install --ignore-scripts
```

### 3. Windows environment initialization

Run this in the same shell before Cargo/Tauri commands:

```powershell
.\scripts\setup-env.ps1
```

## Development commands

```bash
# full app (frontend + backend)
npm run tauri dev
# or: cargo tauri dev

# production build
npm run tauri build
# or: cargo tauri build

# frontend only
npm run dev
npm run build
```

## Launch protocol

- Default launch command: `npm run tauri dev`.
- Do not stop at process start; monitor startup output for 404/runtime/initialization failures.
- On failure signals, gather logs and perform first-line debugging before reporting status.

### Optional monitored launcher

Use this helper when you want bounded startup observation and captured logs:

```powershell
.\scripts\launch-toaster-monitored.ps1 -ObservationSeconds 120
```

It runs environment setup, starts `npm run tauri dev`, and prints:

- `monitor_summary=...` (detected success/error signal keys + hints)
- `launch_logs_stdout=...` and `launch_logs_stderr=...` (captured logs)
- `launch_status=launched_ok|launched_with_errors|failed_to_launch`

### Automated midstream live validation (no manual playback loop)

Run the backend media-pipeline harness against a real file (defaults to `C:\Users\alexm\Downloads\AddReleaseItem.mp4`):

```powershell
.\scripts\run-live-midstream-validation.ps1
```

Override media path/output directory:

```powershell
.\scripts\run-live-midstream-validation.ps1 -MediaPath "C:\path\to\file.mp4" -OutputDir "C:\temp\toaster-live-validation"
```

Optionally set the local Whisper model file used by the ASR leakage oracle:

```powershell
.\scripts\run-live-midstream-validation.ps1 -AsrModelPath "C:\path\to\ggml-small.bin"
```

The run writes `live-validation-report.json` to the output directory with objective pass/fail metrics (duration parity, boundary parity, seam artifact checks, ASR leakage oracle). The ASR oracle fails explicitly if `TOASTER_LIVE_ASR_MODEL_PATH` is missing or invalid.

### Offline local LLM eval gate (cleanup + precision + ASR oracle)

Run the combined offline rollout gate:

```powershell
.\scripts\run-local-llm-eval-gate.ps1 -MediaPath "C:\path\to\file.mp4" -AsrModelPath "C:\path\to\ggml-small.bin"
```

Optional output directory override:

```powershell
.\scripts\run-local-llm-eval-gate.ps1 -MediaPath "C:\path\to\file.mp4" -AsrModelPath "C:\path\to\ggml-small.bin" -OutputDir "C:\temp\toaster-local-llm-gate"
```

This gate has no silent fallback for required inputs:

- `-MediaPath` is required and must point to an existing media file.
- `-AsrModelPath` is required and must point to an existing local Whisper model file.

The run writes `local-llm-eval-gate-report.json` with machine-readable pass/fail output, explicit criteria for each check (`cleanup_quality`, `precision_safety`, `asr_leakage_oracle`), and failure reasons when the gate fails.

### First Build Timing

The first build after cloning (or after clearing `target/`) takes **2-4 minutes** due to:
- whisper-rs-sys Vulkan/ONNX compilation (~60s)
- Full Rust dependency compilation (~90s)
- Vite bundling (~15s)

Subsequent incremental builds typically take 10-30 seconds.
The launch monitoring script defaults to 120 seconds to accommodate first builds.

## Test and lint

```bash
cd src-tauri && cargo test
cd src-tauri && cargo test test_filter_filler_words -- --nocapture
cd src-tauri && cargo clippy
npm run lint
```

## Windows guardrails

- Use MSVC Rust toolchain target (not GNU)
- Run Cargo commands from `src-tauri\` when working directly with Cargo
- Stop running `toaster-app.exe`/`toaster.exe` before rebuilds to avoid DLL lock/link errors

## Windows code signing

The production build (`cargo tauri build`) produces an unsigned installer by default.
`src-tauri/tauri.conf.json` sets `"signCommand": ""` — an empty string means no signing.

**Unsigned builds will trigger Windows SmartScreen warnings** ("Windows protected your PC")
on first launch, which may deter users.

### What you need to sign

1. **Code signing certificate** — an EV (Extended Validation) certificate removes
   SmartScreen warnings immediately; a standard (OV) certificate builds trust over time.
2. **Set `signCommand`** in `tauri.conf.json` to invoke `signtool`, e.g.:
   ```json
   "signCommand": "signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /f \"%CERT_PATH%\" /p \"%CERT_PASSWORD%\" \"%1\""
   ```
3. **CI environment variables** — expose `CERT_PATH` (path to `.pfx` file) and
   `CERT_PASSWORD` (certificate password) as secrets in your CI pipeline.

For full details see the
[Tauri Windows signing guide](https://v2.tauri.app/distribute/sign/windows/).

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `libclang not found` | LLVM missing | Install LLVM and set `LIBCLANG_PATH` |
| `VULKAN_SDK not set` | Vulkan SDK missing | Install Vulkan SDK and set `VULKAN_SDK` |
| `link.exe not found` | MSVC env not loaded | Run `scripts/setup-env.ps1` in current shell |
| `ort does not provide prebuilt binaries for gnu` | Wrong target | Use `stable-x86_64-pc-windows-msvc` |
