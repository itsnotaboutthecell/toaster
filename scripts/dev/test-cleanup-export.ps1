#!/usr/bin/env pwsh
# test-cleanup-export.ps1
# Runs the cleanup-cascade integration test and reports the result.

$ErrorActionPreference = "Stop"

# Load build environment if the setup script exists
$setupScript = Join-Path $PSScriptRoot "setup-env.ps1"
if (Test-Path $setupScript) {
    Write-Host "Loading build environment..." -ForegroundColor Cyan
    . $setupScript
}

Push-Location (Join-Path $PSScriptRoot ".." "src-tauri")
try {
    Write-Host "Running cleanup cascade test..." -ForegroundColor Cyan
    cargo test cleanup_cascade_produces -- --nocapture
    if ($LASTEXITCODE -eq 0) {
        Write-Host "`n✅ Cleanup cascade test PASSED" -ForegroundColor Green
    } else {
        Write-Host "`n❌ Cleanup cascade test FAILED (exit code $LASTEXITCODE)" -ForegroundColor Red
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
}
