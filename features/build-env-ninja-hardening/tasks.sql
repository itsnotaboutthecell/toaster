-- Task graph for build-env-ninja-hardening.
-- Ingest into the session SQL store with the `sql` tool.

INSERT INTO todos (id, title, description, status) VALUES
  ('build-env-ninja-hardening-enumerate',
   'Enumerate Ninja-hostile vcvars vars from cmake-rs source',
   'Read the cmake-rs version pinned in src-tauri/Cargo.lock and list every env::var lookup it promotes onto the cmake command line as a -D flag. Cross-check against CMake docs for implicit defaults (Platform, CMAKE_GENERATOR_PLATFORM, VSINSTALLDIR, CMAKE_GENERATOR_INSTANCE, CMAKE_GENERATOR_TOOLSET, VCToolsInstallDir, VCINSTALLDIR, VisualStudioVersion). Output: a finalized $script:NinjaHostileVars array, written to features/build-env-ninja-hardening/journal.md. Verifier: AC-001-a per coverage.json.',
   'pending'),

  ('build-env-ninja-hardening-strip',
   'Extend setup-env.ps1 strip block (R-001)',
   'Add a $script:NinjaHostileVars constant in scripts/setup-env.ps1 and replace the single Remove-Item Env:Platform line at lines 41-51 with a foreach loop over that constant using Remove-Item Env:$_ -ErrorAction SilentlyContinue. Preserve the existing comment block; extend it to mention the broader class. Verifier: AC-001-a, AC-001-b per coverage.json.',
   'pending'),

  ('build-env-ninja-hardening-preflight',
   'Extend setup-env.ps1 preflight + status signal (R-002)',
   'Replace the hard-coded if-block at scripts/setup-env.ps1:112-121 with a foreach over $script:NinjaHostileVars when $env:CMAKE_GENERATOR -eq "Ninja". On any hit, print red [FAIL] naming each offender, and set $global:ToasterEnvPreflightOk = $false. On clean env, set $global:ToasterEnvPreflightOk = $true. Do not call exit (script is dot-sourced). Verifier: AC-002-a per coverage.json.',
   'pending'),

  ('build-env-ninja-hardening-smoke',
   'Create scripts/check-cmake-ninja-env.ps1 smoke script (R-004)',
   'New file scripts/check-cmake-ninja-env.ps1. Writes a trivial CMakeLists.txt (cmake_minimum_required(VERSION 3.13) + project(toaster_smoke C)) to a temp dir, runs cmake -G Ninja -S <tmp> -B <tmp>/build, captures exit code, surfaces any "Ninja does not support" / "instance specification" line on failure. Cleans up in a finally block. Wall-clock budget < 5s. Verifier: AC-004-a, AC-004-b per coverage.json.',
   'pending'),

  ('build-env-ninja-hardening-launcher-hook',
   'Hook smoke script + preflight status into launch-toaster-monitored.ps1',
   'After dot-sourcing setup-env.ps1 and before invoking cargo tauri dev, check $global:ToasterEnvPreflightOk and run pwsh scripts/check-cmake-ninja-env.ps1. On either failure, set launch_status=failed_to_launch, surface the diagnostic in launch_logs_stderr, and return without starting cargo. Verifier: AC-002-b, AC-004-c per coverage.json.',
   'pending'),

  ('build-env-ninja-hardening-docs',
   'Document Ninja-hostile vcvars list in docs/build.md (R-003)',
   'Append a "Ninja-hostile vcvars vars" subsection to docs/build.md "Build environment gotchas" immediately after the existing "Platform=x64 from vcvars vs CMAKE_GENERATOR=Ninja" subsection. List every var class with the symptom it produces and the one-line remediation. Add a troubleshooting table row mirroring the existing Platform=x64 row at line 160 for the VSINSTALLDIR / CMAKE_GENERATOR_INSTANCE case. Do NOT duplicate the literal list — describe the class only; the canonical list lives in scripts/setup-env.ps1. Invoke the canonical-instructions skill before editing. Verifier: AC-003-a, AC-003-b per coverage.json.',
   'pending'),

  ('build-env-ninja-hardening-qc',
   'QC: run smoke + monitored launch + coverage gate',
   'Run (1) pwsh scripts/verify-build-env-ninja-hardening.ps1 -All; expect exit 0 with all 8 automated ACs PASS (AC-001-a, AC-001-b, AC-002-a, AC-002-b, AC-003-a, AC-003-b, AC-004-a, AC-004-b). Note: AC-002-a tests preflight from a fresh shell (inject VSINSTALLDIR=C:\\stub, then source setup-env once, expect [FAIL] + $global:ToasterEnvPreflightOk=$false) — NOT a re-source from an already-configured shell, which is suppressed by the TOASTER_ENV_INITIALIZED sentinel. (2) AC-004-c live-app gate (manual per AGENTS.md "Verified means the live app"): pwsh scripts/launch-toaster-monitored.ps1 -ObservationSeconds 300; expect launch_status=launched_ok and zero "does not support" / "instance specification" matches in captured logs. (3) pwsh scripts/check-feature-coverage.ps1 -Feature build-env-ninja-hardening; expect exit 0. Append results to journal.md.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('build-env-ninja-hardening-strip',          'build-env-ninja-hardening-enumerate'),
  ('build-env-ninja-hardening-preflight',      'build-env-ninja-hardening-strip'),
  ('build-env-ninja-hardening-smoke',          'build-env-ninja-hardening-enumerate'),
  ('build-env-ninja-hardening-launcher-hook',  'build-env-ninja-hardening-preflight'),
  ('build-env-ninja-hardening-launcher-hook',  'build-env-ninja-hardening-smoke'),
  ('build-env-ninja-hardening-docs',           'build-env-ninja-hardening-enumerate'),
  ('build-env-ninja-hardening-qc',             'build-env-ninja-hardening-launcher-hook'),
  ('build-env-ninja-hardening-qc',             'build-env-ninja-hardening-docs');
