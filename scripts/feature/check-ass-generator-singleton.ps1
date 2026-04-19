<#
.SYNOPSIS
    Assert that managers::captions::blocks_to_ass has exactly one call
    site inside src-tauri/src/commands/waveform/.

.DESCRIPTION
    Stub placeholder committed by the ass-sidecar-export planning bundle
    so the coverage gate can resolve the AC-003-a verifier path. The
    real implementation lands with the ass-sidecar-export-refactor task
    (see features/ass-sidecar-export/tasks.sql).

    Exits 2 (not-implemented) to prevent accidental "green" reporting
    before the feature is built.
#>

[CmdletBinding()]
param()

Write-Host "[ass-sidecar-export] check-ass-generator-singleton stub: not yet implemented." -ForegroundColor Yellow
Write-Host "Feature: features/ass-sidecar-export — task: ass-sidecar-export-refactor"
exit 2
