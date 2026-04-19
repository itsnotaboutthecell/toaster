# clean-whisper-cache.ps1
# Nukes the cached whisper-rs-sys build artifacts so the next cargo run
# re-runs build.rs from scratch.
#
# WHY THIS EXISTS:
#   whisper-rs-sys's build.rs declares only `rerun-if-env-changed` for
#   BINDGEN_EXTRA_CLANG_ARGS* and VULKAN_SDK — NOT CMAKE_GENERATOR or
#   Platform. So if a single build runs under a bad environment (e.g.
#   MSBuild's `Platform=x64` leaking into a Ninja CMake configure), the
#   failed CMakeCache is cached forever and every subsequent build keeps
#   failing the same way. Cargo also has a separate fingerprint dir that
#   has to be cleaned alongside the build dir.
#
#   Different cargo subcommands (check vs test, with different feature
#   flags) hash to different `whisper-rs-sys-<hash>` directories, so we
#   wildcard them all.
#
# WHEN TO RUN:
#   - After fixing scripts\setup-env.ps1 and still seeing
#     "Generator Ninja does not support platform specification".
#   - After changing CMAKE_GENERATOR, Vulkan SDK location, or any other
#     env var that whisper-rs-sys's build.rs does not advertise.
#   - When you want a clean baseline before a release build.
#
# Safe to run anytime; worst case is a longer next build.

[CmdletBinding()]
param(
    [string]$TargetDir = (Join-Path $PSScriptRoot "..\src-tauri\target")
)

$ErrorActionPreference = "Stop"

$resolvedTarget = Resolve-Path -Path $TargetDir -ErrorAction SilentlyContinue
if (-not $resolvedTarget) {
    Write-Host "Target dir does not exist: $TargetDir (nothing to clean)" -ForegroundColor Yellow
    exit 0
}

$paths = @(
    "debug\build\whisper-rs-sys-*",
    "debug\.fingerprint\whisper-rs-sys-*",
    "release\build\whisper-rs-sys-*",
    "release\.fingerprint\whisper-rs-sys-*"
)

$removed = 0
foreach ($pattern in $paths) {
    $full = Join-Path $resolvedTarget $pattern
    Get-ChildItem -Path $full -Directory -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host "  removing $($_.FullName)" -ForegroundColor DarkGray
        Remove-Item -Recurse -Force $_.FullName
        $removed++
    }
}

if ($removed -eq 0) {
    Write-Host "No whisper-rs-sys cache directories found under $resolvedTarget" -ForegroundColor Green
} else {
    Write-Host "Removed $removed whisper-rs-sys cache director(y/ies). Next build will re-run cmake from scratch." -ForegroundColor Green
}
