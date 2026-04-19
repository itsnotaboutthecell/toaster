# Toaster Windows Build Environment Setup
# Run this script before building: . .\scripts\setup-env.ps1

Write-Host "Setting up Toaster build environment..." -ForegroundColor Cyan

# Rust
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Program Files\CMake\bin;$env:PATH"

# LLVM (for bindgen / whisper-rs-sys)
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
if (-not (Test-Path "$env:LIBCLANG_PATH\libclang.dll")) {
    Write-Host "WARNING: LLVM not found. Install with: winget install LLVM.LLVM" -ForegroundColor Yellow
}

# Vulkan SDK (for whisper Vulkan acceleration)
$vulkanDir = Get-ChildItem "C:\VulkanSDK" -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1 -ExpandProperty FullName
if ($vulkanDir) {
    $env:VULKAN_SDK = $vulkanDir
    Write-Host "Vulkan SDK: $vulkanDir" -ForegroundColor Green
} else {
    Write-Host "WARNING: Vulkan SDK not found. Install with: winget install KhronosGroup.VulkanSDK" -ForegroundColor Yellow
}

# CMake generator
$env:CMAKE_GENERATOR = "Ninja"

# Canonical Ninja-hostile-vars list -- declared early so the snapshot
# below can run BEFORE vcvars sources its own copy of these names.
# Keep in sync with docs/build.md "Ninja-hostile vcvars vars".
$script:NinjaHostileVars = @(
    'Platform',                  # MSBuild arch -- CMake reads as default CMAKE_GENERATOR_PLATFORM
    'CMAKE_GENERATOR_PLATFORM',  # -A flag equivalent; Ninja rejects
    'CMAKE_GENERATOR_TOOLSET',   # -T flag equivalent; Ninja rejects
    'CMAKE_GENERATOR_INSTANCE',  # VS install path; Ninja rejects
    'VSINSTALLDIR',              # vcvars-side; cmake-rs promotes to CMAKE_GENERATOR_INSTANCE
    'VCINSTALLDIR',              # vcvars-side; cmake-rs hint for VS install root
    'VCToolsInstallDir',         # vcvars-side; cmake-rs hint for toolset path
    'VisualStudioVersion'        # vcvars-side; cmake-rs hint for VS major version
)

# Snapshot the INHERITED env (parent shell / user injection) BEFORE we
# source vcvars64.bat. vcvars adds its own copies of VSINSTALLDIR /
# VCINSTALLDIR / etc., so any snapshot taken after vcvars cannot
# distinguish "user-injected leak" from "vcvars set it just now".
#
# $wasReSource suppresses the inherited-leak preflight on re-source
# inside an already-configured shell. Without it, devs who source
# setup-env once and then invoke the launcher from the same shell
# would hit a spurious FAIL on every launch.
$wasReSource = ($env:TOASTER_ENV_INITIALIZED -eq '1')
$inheritedHostile = @{}
foreach ($v in $script:NinjaHostileVars) {
    $val = [Environment]::GetEnvironmentVariable($v, 'Process')
    if ($val) { $inheritedHostile[$v] = $val }
}

# Source MSVC environment (Visual Studio Build Tools)
$vcvarsall = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if (Test-Path $vcvarsall) {
    $envOut = cmd /c "`"$vcvarsall`" x64 >nul 2>&1 && set" 2>&1
    foreach ($line in $envOut) {
        if ($line -match "^([^=]+)=(.*)$") {
            [Environment]::SetEnvironmentVariable($matches[1], $matches[2], "Process")
        }
    }
    Write-Host "MSVC environment sourced" -ForegroundColor Green
} else {
    Write-Host "WARNING: VS Build Tools not found. Install C++ workload." -ForegroundColor Yellow
}

# Strip Ninja-hostile vcvars exports.
#
# vcvars64.bat exports a pile of env vars meant to drive MSBuild and the
# Visual Studio generators. CMake (and the cmake-rs crate that whisper-rs-sys
# uses) read several of them and forward as -D flags or generator-instance
# hints. Because we force `CMAKE_GENERATOR=Ninja` above, any of these will
# trip Ninja's "does not support {platform,toolset,instance} specification"
# rejection at the first `project()` call -- 4 minutes deep into a
# whisper-rs-sys build.
#
# We build exclusively with cl.exe + Ninja, never with MSBuild or the VS
# generators, so these vars have no legitimate consumer in this shell.
# Do NOT delete this block without re-reading
# docs/build.md > Build environment gotchas > Ninja-hostile vcvars vars.
foreach ($v in $script:NinjaHostileVars) {
    if (Test-Path "Env:$v") { Remove-Item "Env:$v" -ErrorAction SilentlyContinue }
}

# Bindgen clang include paths
$msvcBase = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC"
$msvcVersion = Get-ChildItem $msvcBase -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1 -ExpandProperty Name
$ucrtBase = "C:\Program Files (x86)\Windows Kits\10\Include"
$ucrtVersion = Get-ChildItem $ucrtBase -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1 -ExpandProperty Name

if ($msvcVersion -and $ucrtVersion) {
    $msvcInclude = "$msvcBase\$msvcVersion\include"
    $ucrtInclude = "$ucrtBase\$ucrtVersion\ucrt"

    # Locate clang's own builtin header directory (contains stdbool.h, stdint.h, etc.).
    # Must come FIRST on the include path: MSVC's <stdbool.h> / <stdint.h> rely on
    # clang builtin headers when libclang parses with --target=*-windows-msvc.
    # Without this, bindgen fails with "'stdbool.h' file not found" and whisper-rs-sys
    # falls back to stale bundled bindings → struct-layout assertion overflow errors.
    $clangBuiltinInclude = $null
    $clangLibDir = "C:\Program Files\LLVM\lib\clang"
    if (Test-Path $clangLibDir) {
        $clangVer = Get-ChildItem $clangLibDir -Directory -ErrorAction SilentlyContinue |
            Sort-Object { [int]($_.Name -replace '\D','') } -Descending |
            Select-Object -First 1 -ExpandProperty Name
        if ($clangVer) {
            $candidate = "$clangLibDir\$clangVer\include"
            if (Test-Path "$candidate\stdbool.h") { $clangBuiltinInclude = $candidate }
        }
    }

    $bindgenArgs = @("--target=x86_64-pc-windows-msvc")
    if ($clangBuiltinInclude) { $bindgenArgs += "-I`"$clangBuiltinInclude`"" }
    $bindgenArgs += "-I`"$msvcInclude`""
    $bindgenArgs += "-I`"$ucrtInclude`""
    $env:BINDGEN_EXTRA_CLANG_ARGS = $bindgenArgs -join " "
    Write-Host "Bindgen includes configured" -ForegroundColor Green
    if (-not $clangBuiltinInclude) {
        Write-Host "  WARNING: clang builtin include dir not found; bindgen may fail" -ForegroundColor Yellow
    }
}

# Verify toolchain
$checks = @(
    @{ Name = "rustc"; Cmd = { rustc --version 2>$null } },
    @{ Name = "cargo"; Cmd = { cargo --version 2>$null } },
    @{ Name = "cl.exe"; Cmd = { Get-Command cl.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source } },
    @{ Name = "ninja"; Cmd = { Get-Command ninja -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source } },
    @{ Name = "cmake"; Cmd = { cmake --version 2>$null | Select-Object -First 1 } }
)

Write-Host "`nToolchain:" -ForegroundColor Cyan
foreach ($check in $checks) {
    $result = & $check.Cmd
    if ($result) {
        Write-Host "  [OK] $($check.Name): $result" -ForegroundColor Green
    } else {
        Write-Host "  [!!] $($check.Name): NOT FOUND" -ForegroundColor Red
    }
}

Write-Host "`nEnvironment ready. Run 'cargo tauri dev' to start." -ForegroundColor Cyan

# Preflight: catch future regressions of the vcvars leak class on day zero.
# Two failure modes:
#   1. Inherited leak -- the parent shell (or a prior run of this script)
#      had a Ninja-hostile var set when we started. Strip neutralised it,
#      but we still want to scream so the user fixes their profile or
#      stops re-sourcing setup-env in a polluted shell. Captured into
#      $inheritedHostile above, before strip ran.
#   2. Post-strip leak -- something between strip and here re-exported a
#      tracked var (a future bug in this script, or a sourced helper).
#      Caught by re-scanning the env now.
#
# Sets $global:ToasterEnvPreflightOk so launch-toaster-monitored.ps1 can
# refuse to invoke `cargo tauri dev` on a corrupted env.
$global:ToasterEnvPreflightOk = $true
if ($env:CMAKE_GENERATOR -eq 'Ninja') {
    if ($inheritedHostile.Count -gt 0 -and -not $wasReSource) {
        Write-Host "`n[FAIL] Build env: Ninja-hostile vars inherited from parent shell:" -ForegroundColor Red
        foreach ($kv in $inheritedHostile.GetEnumerator()) {
            Write-Host "       $($kv.Key)=$($kv.Value)" -ForegroundColor Red
        }
        Write-Host "       Strip cleaned them, but fix your shell profile or open a fresh window." -ForegroundColor Red
        Write-Host "       See docs/build.md > Build environment gotchas > Ninja-hostile vcvars vars." -ForegroundColor Red
        $global:ToasterEnvPreflightOk = $false
    }
    $leaked = @()
    foreach ($v in $script:NinjaHostileVars) {
        $val = [Environment]::GetEnvironmentVariable($v, 'Process')
        if ($val) { $leaked += "$v=$val" }
    }
    if ($leaked.Count -gt 0) {
        Write-Host "`n[FAIL] Build env corrupted post-strip: tracked vars re-appeared:" -ForegroundColor Red
        foreach ($l in $leaked) { Write-Host "       $l" -ForegroundColor Red }
        Write-Host "       CMake will reject these. See docs/build.md > Build environment gotchas." -ForegroundColor Red
        $global:ToasterEnvPreflightOk = $false
    }
}

# Sentinel: marks this shell as having completed setup-env at least once.
# Read by the inherited-leak preflight above on re-source to suppress the
# FAIL that would otherwise fire on every legitimate re-source / launcher
# invocation from an already-configured shell.
if ($global:ToasterEnvPreflightOk) {
    $env:TOASTER_ENV_INITIALIZED = '1'
}
