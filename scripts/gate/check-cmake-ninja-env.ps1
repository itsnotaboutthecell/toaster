# Toaster CMake/Ninja environment smoke test.
#
# Runs `cmake -G Ninja` on a trivial CMakeLists.txt in a fresh temp dir.
# Catches the vcvars-leak class of bug (Platform / CMAKE_GENERATOR_PLATFORM
# / CMAKE_GENERATOR_TOOLSET / CMAKE_GENERATOR_INSTANCE escaping into the
# shell) in <5s, instead of 4 minutes deep into a whisper-rs-sys build.
#
# Also wipes stale CMakeCache.txt entries from prior VS-generator builds
# under src-tauri/target/debug/build/whisper-rs-sys-* -- a separate failure
# mode where CMAKE_GENERATOR_INSTANCE/PLATFORM/TOOLSET are baked into the
# cache as INTERNAL values from a previous failed configure and conflict
# with the current Ninja generator.
#
# Exit codes:
#   0 = environment is sound; safe to invoke `cargo tauri dev`
#   1 = environment is corrupted or stale cache detected and not cleaned

[CmdletBinding()]
param(
    [switch]$WipeStaleCaches
)

$ErrorActionPreference = 'Stop'
$failures = @()

# 0. Live env scan: any Ninja-hostile var still set is an instant fail.
#    Trivial cmake projects ignore CMAKE_GENERATOR_INSTANCE / Platform /
#    CMAKE_GENERATOR_TOOLSET, so we cannot rely on the cmake invocation
#    below to surface those leaks. Scan the env first so the smoke script
#    is a true environment gate.
$ninjaHostileVars = @(
    'Platform',
    'CMAKE_GENERATOR_PLATFORM',
    'CMAKE_GENERATOR_TOOLSET',
    'CMAKE_GENERATOR_INSTANCE',
    'VSINSTALLDIR',
    'VCINSTALLDIR',
    'VCToolsInstallDir',
    'VisualStudioVersion'
)
$leaked = @()
foreach ($v in $ninjaHostileVars) {
    $val = [Environment]::GetEnvironmentVariable($v, 'Process')
    if ($val) { $leaked += "$v=$val" }
}
if ($leaked.Count -gt 0 -and $env:CMAKE_GENERATOR -eq 'Ninja') {
    $failures += "Ninja-hostile env vars leaked into CMake-Ninja shell:"
    foreach ($l in $leaked) { $failures += "    $l" }
    $failures += "Re-source scripts/setup-env.ps1 in a fresh PowerShell window."
}

# 1. Smoke: configure trivial project with -G Ninja.
$tmp = Join-Path $env:TEMP "toaster-cmake-ninja-smoke-$(Get-Random)"
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
try {
    @"
cmake_minimum_required(VERSION 3.10)
project(toaster_cmake_smoke C)
"@ | Out-File -FilePath (Join-Path $tmp "CMakeLists.txt") -Encoding ascii

    Push-Location $tmp
    try {
        $out = cmake -G Ninja . 2>&1
        if ($LASTEXITCODE -ne 0) {
            $failures += "cmake -G Ninja failed (exit $LASTEXITCODE):"
            $failures += ($out | Select-String "Error|error|specification" | Select-Object -First 6 | ForEach-Object { "    $_" })
        }
    } finally {
        Pop-Location
    }
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

# 2. Stale-cache scan: look for whisper-rs-sys CMakeCache.txt files with
#    INTERNAL values that conflict with the current Ninja generator.
$repoRoot = Split-Path -Parent $PSScriptRoot
$caches = Get-ChildItem -Path (Join-Path $repoRoot "src-tauri\target\debug\build") `
    -Filter "CMakeCache.txt" -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match "whisper-rs-sys" }

$staleCaches = @()
foreach ($c in $caches) {
    $content = Get-Content $c.FullName -ErrorAction SilentlyContinue
    $hasStale = $content | Select-String -Pattern "^CMAKE_GENERATOR_(INSTANCE|PLATFORM|TOOLSET):INTERNAL=." -Quiet
    if ($hasStale) { $staleCaches += $c.FullName }
}

if ($staleCaches.Count -gt 0) {
    if ($WipeStaleCaches) {
        foreach ($c in $staleCaches) {
            $buildDir = Split-Path $c -Parent
            Remove-Item $buildDir -Recurse -Force -ErrorAction SilentlyContinue
            Write-Host "[smoke] Wiped stale cache: $buildDir" -ForegroundColor Yellow
        }
    } else {
        $failures += "Stale CMakeCache.txt with INTERNAL CMAKE_GENERATOR_{INSTANCE,PLATFORM,TOOLSET} from prior VS-generator build:"
        foreach ($c in $staleCaches) { $failures += "    $c" }
        $failures += "Re-run with -WipeStaleCaches, or run: cargo clean -p whisper-rs-sys --manifest-path src-tauri\Cargo.toml"
    }
}

if ($failures.Count -gt 0) {
    Write-Host "[smoke] FAIL" -ForegroundColor Red
    foreach ($f in $failures) { Write-Host "  $f" -ForegroundColor Red }
    Write-Host "  See docs/build.md > Build environment gotchas > Ninja-hostile vcvars vars." -ForegroundColor Red
    exit 1
}

Write-Host "[smoke] OK: cmake -G Ninja configures cleanly; no stale whisper-rs-sys caches." -ForegroundColor Green
exit 0
