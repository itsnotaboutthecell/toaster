---
name: build-and-test
description: 'Build and validate Toaster (Tauri + Rust + React). Use for compile/test/lint runs, toolchain issues, and Windows linker/env troubleshooting.'
---

# Build and Test

Use this skill when compiling, linting, testing, or diagnosing build failures in Toaster.

## When to use

- Build the app (`cargo tauri dev` / `cargo tauri build`)
- Run Rust checks/tests or frontend linting
- Diagnose Windows setup issues (`LLVM`, `VULKAN_SDK`, `link.exe`, MSVC target, DLL locks)

## Standard runbook

1. Install frontend dependencies when needed:

```bash
bun install --frozen-lockfile
```

2. On Windows, initialize environment in the same shell before Cargo/Tauri:

```powershell
.\scripts\setup-env.ps1
```

3. Run relevant checks/builds:

```bash
cargo tauri dev
cargo tauri build
cd src-tauri && cargo check
cd src-tauri && cargo test
cd src-tauri && cargo test test_filter_filler_words -- --nocapture
cd src-tauri && cargo clippy
npm run lint
```

## Windows guardrails

- Use MSVC target (`stable-x86_64-pc-windows-msvc`), not GNU.
- Run direct Cargo commands from `src-tauri\`.
- Stop `toaster-app.exe`/`toaster.exe` before rebuilding to avoid link/DLL lock failures.

## Troubleshooting map

| Symptom | Likely cause | Fix |
|---|---|---|
| `libclang not found` | LLVM missing or not on PATH | Install LLVM and set `LIBCLANG_PATH` |
| `VULKAN_SDK not set` | Vulkan SDK missing | Install Vulkan SDK and set `VULKAN_SDK` |
| `link.exe not found` | MSVC env not loaded | Run `scripts/setup-env.ps1` in current shell |
| `ort does not provide prebuilt binaries for gnu` | GNU Rust target selected | Switch to MSVC target |

## Timeline/precision validation reminder

When a change touches transcript timing, keep-segments, playback mapping, or delete/undo behavior, do not mark it complete until replay confirms midstream deletions remain clean and backend timeline mappings stay authoritative.
