# Blueprint: build env Ninja hardening

## Architecture decisions

- **R-001 strip block: extend the existing pattern at
  `scripts/setup-env.ps1:41-51`.** That comment already captures the
  template (root cause, why it is safe to strip, do-not-delete warning).
  Add a sibling block immediately below it for the new vars, sharing
  the same comment cadence. Single source of truth: there is exactly
  one place in the repo that knows about Ninja-hostile vcvars vars, and
  it is `setup-env.ps1`. Do NOT duplicate the list into the launcher,
  CI, or `docs/build.md` as code — `docs/build.md` only describes the
  list; the canonical enumeration lives in setup-env.ps1.

- **Removal mechanism: `Remove-Item Env:<NAME> -ErrorAction
  SilentlyContinue` per var.** Same idiom as line 51 today. Do not
  assign `$env:NAME = ""`: `cmake-rs` checks presence, not emptiness,
  and an empty value still triggers `-DCMAKE_GENERATOR_INSTANCE=` on
  the cmake command line.

- **Curated list (initial): `Platform`, `CMAKE_GENERATOR_PLATFORM`
  (already covered today), plus `VSINSTALLDIR`,
  `CMAKE_GENERATOR_INSTANCE`, `CMAKE_GENERATOR_TOOLSET`,
  `VCToolsInstallDir`, `VCINSTALLDIR`, `VisualStudioVersion`.**
  Implementer must verify by reading
  `cmake-rs` (the version pinned in `src-tauri/Cargo.lock`) source for
  every `env::var` lookup that maps onto a `-D` flag, and adding any
  hit not already in the list. The list is allowed to grow during
  implementation; it must not shrink without explicit justification in
  the journal.

- **R-002 preflight: extend `scripts/setup-env.ps1:112-121` to iterate
  the curated list.** Replace the hard-coded `$env:Platform -or
  $env:CMAKE_GENERATOR_PLATFORM` check with a `foreach` over the same
  list constant used by the strip block. Reuse one PowerShell `$script:`
  array so the strip and the preflight cannot drift.

- **R-002 failure signaling: write a status variable
  (`$global:ToasterEnvPreflightOk = $false`) AND return a non-zero exit
  code from the preflight function.** The setup script is dot-sourced
  (`. .\scripts\setup-env.ps1`), so a bare `exit 1` would kill the
  user's shell. Instead: set the global, print `[FAIL]`, and have
  `launch-toaster-monitored.ps1` consult `$global:ToasterEnvPreflightOk`
  before invoking `cargo tauri dev`. This keeps the existing dot-source
  contract intact while still aborting the launcher path.

- **R-004 smoke script location: `scripts/gate/check-cmake-ninja-env.ps1`.**
  Same convention as `scripts/check-translations.ts` and
  `scripts/feature/check-feature-coverage.ps1` (`check-*` = fast gate).

- **R-004 smoke script implementation: `New-TemporaryFile`-style temp
  dir, write `cmake_minimum_required(VERSION 3.13)` +
  `project(toaster_smoke C)`, `cmake -G Ninja -S <tmp> -B <tmp>/build`,
  capture exit code and the first matching "Ninja does not support" or
  "instance specification" line, clean up the temp dir in a `finally`.**
  Language `C` only — `CXX` adds ~3 s to configure.

- **R-004 launcher hook:
  `scripts/launch-toaster-monitored.ps1` calls
  `pwsh scripts/gate/check-cmake-ninja-env.ps1` AFTER sourcing
  `setup-env.ps1` and BEFORE invoking `cargo tauri dev`.** On non-zero
  exit, the launcher prints the diagnostic, sets
  `launch_status=failed_to_launch`, and returns without ever starting
  cargo.

- **R-003 documentation placement: append a single
  "Ninja-hostile vcvars vars" subsection to
  `docs/build.md` "Build environment gotchas" right after the existing
  "Platform=x64 from vcvars vs CMAKE_GENERATOR=Ninja" subsection.** Add
  a new troubleshooting table row that mirrors the existing
  `Platform=x64` row, pointing at the same `clean-whisper-cache.ps1`
  remediation. Per `canonical-instructions` skill: do not duplicate the
  curated list anywhere except as prose in `docs/build.md`; AGENTS.md
  remains canonical for "use setup-env.ps1".

## Component & module touch-list

| File | Change kind | Notes |
|------|-------------|-------|
| `scripts/setup-env.ps1` | Edit (strip + preflight) | Extend lines 41-51 and lines 112-121; introduce one `$script:NinjaHostileVars` constant. |
| `scripts/gate/check-cmake-ninja-env.ps1` | New | New gate script per R-004. |
| `scripts/launch-toaster-monitored.ps1` | Edit (hook) | Insert smoke-script call between setup sourcing and cargo invocation; respect `$global:ToasterEnvPreflightOk`. |
| `docs/build.md` | Edit (docs) | New subsection + new troubleshooting row. |
| `.github/workflows/*.yml` (Windows job, if present) | Optional edit | Add a CI step calling the smoke script; deferred to implementation if a Windows runner exists. |

## Single-source-of-truth placement

- **Curated Ninja-hostile var list:** `scripts/setup-env.ps1`
  (`$script:NinjaHostileVars`). Nothing else duplicates it.
- **CMake generator pin (`Ninja`):** `scripts/setup-env.ps1:25`. The
  smoke script reads `$env:CMAKE_GENERATOR`; the launcher does not
  override it.
- **Preflight failure status:** `$global:ToasterEnvPreflightOk` set by
  setup-env.ps1; consumed by `launch-toaster-monitored.ps1`.

## Data flow

```
[ developer shell ]
    |
    | . .\scripts\setup-env.ps1
    v
[ setup-env.ps1 ]
    + sources vcvars64.bat (env now polluted)
    + Remove-Item Env:<each name in $NinjaHostileVars>
    + sets CMAKE_GENERATOR=Ninja, BINDGEN_EXTRA_CLANG_ARGS, etc.
    + preflight: foreach var in $NinjaHostileVars => if set, [FAIL] +
      $global:ToasterEnvPreflightOk = $false
    |
    v
[ launch-toaster-monitored.ps1 ]
    + if $global:ToasterEnvPreflightOk -eq $false => abort, status=failed_to_launch
    + else: pwsh scripts/gate/check-cmake-ninja-env.ps1
       - exit 0 => proceed to cargo tauri dev
       - exit !=0 => abort, status=failed_to_launch, surface diagnostic
    |
    v
[ cargo tauri dev ]
    + whisper-rs-sys configure now succeeds (no -DCMAKE_GENERATOR_*
      flags injected)
```

## Migration / compatibility

- No state, no on-disk schema, no user-facing behaviour change. The
  feature is invisible on a healthy machine.
- Existing developers who already have `target/` populated with a
  failed `whisper-rs-sys-<hash>` directory must run
  `scripts/dev/clean-whisper-cache.ps1` once after pulling. Call that out
  in the journal entry promoting STATE.
- Dot-source contract for `setup-env.ps1` is preserved (no `exit` in
  preflight; status via `$global:` variable instead).

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Strip list too aggressive — removes a var cl.exe needs (e.g. `INCLUDE`) | Curated list excludes compiler/linker path vars; document "do not add `INCLUDE`/`LIB`/`LIBPATH`/`PATH`" inline | AC-004-c (live `cargo tauri dev` reaches splash) |
| Strip list incomplete — a fourth leak emerges in a future `cmake-rs` version | Smoke script runs on every monitored launch and fails fast | AC-004-a, AC-004-c |
| Preflight uses `exit 1` and kills the dot-sourcing user shell | Use `$global:ToasterEnvPreflightOk` instead | AC-002-b |
| Smoke script depends on network / GPU and slows the launcher | C-only `project()`, no `find_package`, temp dir, `< 5 s` budget | AC-004-a |
| List drift between strip block and preflight | Both iterate the same `$script:NinjaHostileVars` constant | AC-002-a |
| Docs go stale relative to the curated list | Docs describe the class, not the literal list; the list lives only in setup-env.ps1 | AC-003-a (manual review against setup-env.ps1) |
| New leak appears only under `cargo test` (different `whisper-rs-sys-<hash>`) | docs/build.md:211-222 already documents this; smoke script reproduces independently of cargo | AC-004-b |
