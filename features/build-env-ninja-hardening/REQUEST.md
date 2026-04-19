# Feature request: build env Ninja hardening

## 1. Problem & Goals

`cargo tauri dev` on Windows fails repeatedly during `whisper-rs-sys`
configure because `vcvars64.bat` (sourced by `scripts/setup-env.ps1:28-39`)
exports a pile of MSBuild-oriented environment variables that the
`cmake-rs` crate auto-promotes onto the cmake command line as `-D`
flags. Those flags are mutually exclusive with `CMAKE_GENERATOR=Ninja`
which we force in `scripts/setup-env.ps1:25`.

We have already paid this tax twice:

1. `Visual Studio 18 2026` generator was being auto-picked → fixed by
   forcing `CMAKE_GENERATOR=Ninja`.
2. `Platform=x64` leaked → CMake injected `CMAKE_GENERATOR_PLATFORM=x64`
   → "Generator Ninja does not support platform specification, but
   platform x64 was specified". Fixed at `scripts/setup-env.ps1:41-51`
   with a `Remove-Item Env:Platform` and a preflight at lines 112-121.

Now we hit it a third time:

```
CMake Error at CMakeLists.txt:2 (project):
  Generator
    Ninja
  does not support instance specification, but instance
    C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools
  was specified.
```

Root cause: `VSINSTALLDIR` (set by vcvars) → `cmake-rs` auto-promotes to
`-DCMAKE_GENERATOR_INSTANCE=...` → Ninja rejects it.

The pattern is structural: vcvars sets a wide pile of MSBuild variables
and several of them cmake/cmake-rs read as implicit defaults for
generator-specific flags. The current setup-env.ps1 strips one
(`Platform`); we need to enumerate and strip ALL of them, and to ensure
the next leak fails at env-setup time (in seconds) rather than four
minutes deep into a `cargo build`.

Goal: end the whack-a-mole. Make Ninja-incompatible env leaks impossible
to merge.

## 2. Desired Outcome & Acceptance Criteria

- After sourcing `scripts/setup-env.ps1`, no Ninja-hostile env var is
  set alongside `CMAKE_GENERATOR=Ninja`.
- A < 5 s smoke check confirms `cmake -G Ninja` succeeds against a
  trivial `CMakeLists.txt`, so regressions surface at env-setup time
  instead of mid-`cargo build`.
- The full known-bad-vars list is documented in `docs/build.md`
  alongside the existing "Build environment gotchas" section so future
  contributors can find it without code-spelunking.
- A live-app `cargo tauri dev` on a clean `target/` reaches the React
  splash without hitting any "Ninja does not support … specification"
  error.

## 3. Scope Boundaries

### In scope

- `scripts/setup-env.ps1` strip block (extending lines 41-51 pattern).
- `scripts/setup-env.ps1` preflight block (extending lines 112-121).
- `docs/build.md` "Build environment gotchas" section.
- A new `scripts/check-cmake-ninja-env.ps1` smoke script.
- Wiring the smoke script into `scripts/launch-toaster-monitored.ps1`
  as a fast-fail preflight before `cargo tauri dev` starts.

### Out of scope (explicit)

- Switching off Ninja. Ninja stays.
- Patching `cmake-rs` upstream. We treat its env-promotion behaviour as
  fixed and quarantine the inputs.
- macOS / Linux. The leaks are vcvars-specific, so the strip + preflight
  only run inside the Windows code path of setup-env.ps1.
- Replacing vcvars with a hand-rolled MSVC env exporter. Too risky for
  this slice.
- Reworking the launcher's monitoring loop.

## 4. References to Existing Code

- `scripts/setup-env.ps1:25` — `$env:CMAKE_GENERATOR = "Ninja"`.
- `scripts/setup-env.ps1:28-39` — vcvars64.bat sourcing block.
- `scripts/setup-env.ps1:41-51` — existing `Platform` strip (the
  pattern to extend).
- `scripts/setup-env.ps1:112-121` — existing preflight check (the
  pattern to extend).
- `docs/build.md:160` — troubleshooting row for the `Platform` leak.
- `docs/build.md:162-209` — "Build environment gotchas" section.
- `scripts/launch-toaster-monitored.ps1` — sources setup-env.ps1 then
  launches `cargo tauri dev`; the new smoke script hooks in here.
- `AGENTS.md:120-125` — Windows requirements (setup-env.ps1 is
  mandatory).
- `AGENTS.md:127-134` — cargo runtime expectations (a 4-minute deep
  cmake failure burns the whole budget; preflights matter).
- `AGENTS.md:108-118` — Launch protocol (monitored launcher only).

## 5. Edge Cases & Constraints

- `cmake-rs` distinguishes "unset" from "set-to-empty". The strip MUST
  use `Remove-Item Env:<NAME>`; assigning `$env:NAME = ""` does not
  remove it and `cmake-rs` may still pick it up.
- vcvars sets `VSINSTALLDIR` with a trailing backslash on some VS
  versions. The preflight check must be presence-based, not value-based.
- The strip must run AFTER vcvars sourcing and BEFORE any cargo / cmake
  invocation. Same position constraint as the existing `Platform` strip.
- Some vcvars-set vars (e.g. `INCLUDE`, `LIB`, `LIBPATH`, `PATH`) ARE
  required by cl.exe / link.exe. The strip list must be surgical.
- The preflight must not regress the existing "scream loudly" UX: it
  prints `[FAIL]` in red and lists the offenders. Whether it also exits
  non-zero is a Blueprint decision.
- The smoke script must finish in < 5 seconds on a developer machine and
  must not require network or GPU.
- ASCII only in all artifacts (AGENTS.md output discipline).

## 6. Data Model

n/a — pure environment-variable hygiene, no persistent data.

## Q&A

(Phase 5 was skipped: the request fully specifies the four R-IDs, the
strip-list discovery method, the script path, and the documentation
surface. There were no scope-changing ambiguities to resolve.)
