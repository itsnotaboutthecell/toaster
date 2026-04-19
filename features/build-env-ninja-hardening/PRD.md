# PRD: build env Ninja hardening

## Problem & Goals

The Windows build pipeline routinely fails because `vcvars64.bat`
(sourced inside `scripts/setup-env.ps1`) exports MSBuild-oriented
environment variables that `cmake-rs` and `cmake` auto-promote onto the
configure command line as generator-specific `-D` flags. Those flags
are incompatible with our pinned `CMAKE_GENERATOR=Ninja`, so every
CMake-driven dependency (`whisper-rs-sys`, `ggml`, ffmpeg) fails
configure with a "Ninja does not support … specification" error.

Two such leaks have already been hand-patched (`Platform` and the
implicit VS generator); the latest leak (`VSINSTALLDIR` →
`CMAKE_GENERATOR_INSTANCE`) confirms this is a recurring class of bug
rather than a one-off. The goal is to enumerate the full known-bad
list, neutralize it in one place, fail loudly at env-setup time on any
future regression, and document the pattern.

## Scope

### In scope

- Strip a curated set of vcvars-exported, Ninja-hostile env vars in
  `scripts/setup-env.ps1` immediately after vcvars sourcing.
- Extend the existing preflight in `scripts/setup-env.ps1` so that any
  future leak in the curated set fails loudly when `CMAKE_GENERATOR=Ninja`.
- A new `scripts/check-cmake-ninja-env.ps1` smoke script that runs
  `cmake -G Ninja` against a trivial `CMakeLists.txt` and exits 0/non-0.
- Hook the smoke script into `scripts/launch-toaster-monitored.ps1` as
  a fast preflight before `cargo tauri dev`.
- Document the full Ninja-hostile vcvars list in `docs/build.md`
  alongside the existing "Build environment gotchas" section.

### Out of scope (explicit)

- Switching the build off Ninja.
- Patching `cmake-rs` upstream behaviour.
- macOS / Linux env handling (the leak vector is vcvars-specific).
- Replacing vcvars with a hand-rolled MSVC env exporter.
- Touching `whisper-rs-sys` build.rs or its `cargo:rerun-if-env-changed`
  set (covered separately in `docs/build.md` lines 211-222).

## Requirements

### R-001 — Neutralize all Ninja-hostile vcvars-exported env vars

- Description: After `scripts/setup-env.ps1` finishes, none of the
  curated Ninja-hostile env vars must remain set in the process. The
  curated list, derived from `cmake-rs` source and CMake's documented
  implicit-default behaviour, must include at minimum: `Platform`,
  `CMAKE_GENERATOR_PLATFORM`, `VSINSTALLDIR`, `CMAKE_GENERATOR_INSTANCE`,
  `CMAKE_GENERATOR_TOOLSET`, and `VCToolsInstallDir`-derived siblings
  that `cmake-rs` reads. Each removal must use `Remove-Item Env:<NAME>`
  (not `$env:NAME = ""`), because `cmake-rs` distinguishes unset from
  empty.
- Rationale: One strip block, single source of truth, surgical enough
  not to break cl.exe/link.exe (which need `INCLUDE`, `LIB`, `LIBPATH`,
  `PATH`).
- Acceptance Criteria
  - AC-001-a — After running `. .\scripts\setup-env.ps1` in a fresh
    shell, querying each name in the curated Ninja-hostile list returns
    "not present" (`Test-Path Env:<NAME>` is `$false`), while
    `CMAKE_GENERATOR` is still `Ninja` and `INCLUDE` / `LIB` / `LIBPATH`
    are still populated.
  - AC-001-b — `scripts/check-cmake-ninja-env.ps1` exits 0 against the
    curated trivial `CMakeLists.txt` immediately after `setup-env.ps1`,
    confirming cmake actually accepts the env (not just a pattern
    match).

### R-002 — Preflight fails loudly on any future leak

- Description: Extend the existing preflight at
  `scripts/setup-env.ps1:112-121` to scan the full curated list (not
  just `Platform` / `CMAKE_GENERATOR_PLATFORM`). When `CMAKE_GENERATOR`
  is `Ninja` and any name in the curated list is set, the preflight
  must print a red `[FAIL]` line that names every offender and the var
  value, AND must signal failure to `scripts/launch-toaster-monitored.ps1`
  (exit code, return value, or a documented signal variable — exact
  mechanism is a Blueprint decision).
- Rationale: Catch the next leak in seconds at env-setup time, not in
  the middle of a multi-minute `whisper-rs-sys` configure.
- Acceptance Criteria
  - AC-002-a — In a shell where `setup-env.ps1` has produced a clean
    env, manually setting `$env:VSINSTALLDIR = 'C:\stub'` and
    re-running the preflight produces a red `[FAIL]` line that names
    `VSINSTALLDIR` AND signals non-success to the caller.
  - AC-002-b — `scripts/launch-toaster-monitored.ps1` aborts before
    invoking `cargo tauri dev` when the preflight signals failure, and
    the captured launch output contains the preflight `[FAIL]` line.

### R-003 — Documentation enumerates the full Ninja-hostile list

- Description: Add a subsection to `docs/build.md` "Build environment
  gotchas" (after the existing `Platform=x64` subsection) that names
  every env var in the curated list, the symptom each one produces when
  it leaks ("Generator Ninja does not support … specification, but …
  was specified"), and the one-line fix (`Remove-Item Env:<NAME>` plus
  `scripts/clean-whisper-cache.ps1` to clear the now-stale CMakeCache).
- Rationale: A future contributor hitting a fourth leak should find the
  pattern in docs in under a minute, not re-discover it.
- Acceptance Criteria
  - AC-003-a — `docs/build.md` contains a single subsection that lists
    every env var named in R-001 alongside its specific CMake error
    message and a one-line remediation.
  - AC-003-b — `docs/build.md` Troubleshooting table has a row for the
    `VSINSTALLDIR` / `CMAKE_GENERATOR_INSTANCE` leak that mirrors the
    existing `Platform=x64` row at line 160.

### R-004 — Sub-5-second cmake/Ninja smoke test script

- Description: Create `scripts/check-cmake-ninja-env.ps1` that writes a
  trivial `CMakeLists.txt` (a single `project(toaster_smoke C)` call)
  to a temp dir, runs `cmake -G Ninja -S <tmp> -B <tmp>/build`, asserts
  exit 0, and returns non-zero with a diagnostic message on failure.
  Must complete in under 5 seconds on a clean developer workstation and
  must require neither network nor GPU.
- Rationale: Catches the regression class in seconds, decoupled from
  the multi-minute `whisper-rs-sys` build, so CI and the launcher can
  gate on it cheaply.
- Acceptance Criteria
  - AC-004-a — On a workstation where `setup-env.ps1` has just run
    cleanly, `pwsh scripts/check-cmake-ninja-env.ps1` exits 0 in under
    5 seconds (wall-clock, measured by `Measure-Command`).
  - AC-004-b — On a shell where any single var from the curated R-001
    list is artificially set, `pwsh scripts/check-cmake-ninja-env.ps1`
    exits non-zero and prints a diagnostic naming the offending env
    var.
  - AC-004-c — A `cargo tauri dev` launched via
    `scripts/launch-toaster-monitored.ps1` reaches the React splash
    without any "Ninja does not support … specification" string in the
    captured `launch_logs_stdout` / `launch_logs_stderr`.

## Edge cases & constraints

- `Remove-Item Env:<NAME>` (not assignment to `""`); `cmake-rs`
  distinguishes unset from empty.
- The strip block must remain surgical: removing `INCLUDE`, `LIB`,
  `LIBPATH`, or `PATH` would break cl.exe and link.exe. The curated
  list MUST exclude these.
- The strip and preflight only run inside the Windows code path
  (vcvars-only leak vector). No-op on macOS/Linux.
- The trivial `CMakeLists.txt` used by the smoke script must specify
  language `C` (no `CXX`) to keep configure under one second.
- ASCII only in all source files; no smart quotes.
- The preflight's signaling mechanism (exit code vs. return-value vs.
  a status variable) must be compatible with being dot-sourced (the
  current convention — `. .\scripts\setup-env.ps1`).

## Data model

n/a.

## Non-functional requirements

- Smoke script wall-clock budget: < 5 s on a developer workstation.
- No new Rust or npm dependencies (AGENTS.md "Local-only inference"
  and dep-hygiene posture).
- Bundle files stay under 800 lines (AGENTS.md output discipline).
