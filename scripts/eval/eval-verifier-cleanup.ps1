<#
.SYNOPSIS
    PowerShell wrapper for scripts/eval-verifier/run_cleanup.py.

.DESCRIPTION
    Runs the LLM-as-a-Verifier Best-of-N ranker over cleanup fixtures in
    src-tauri/tests/fixtures/cleanup/. Each fixture ships a set of
    candidate cleanup outputs (kept_indices); the ranker uses filler
    recall, content preservation, timing monotonicity, and audio-aware
    "deleted region audibility" to pick a winner and compare it against
    the fixture's expected_winner.

    Default backend is "mock" (deterministic, no network). See
    eval-verifier.ps1 for notes on wiring a real local model.
#>

[CmdletBinding()]
param(
    [ValidateSet('mock', 'openai-compat', 'gemini')]
    [string]$Backend = 'mock',
    [string]$BaseUrl = 'http://127.0.0.1:8080/v1',
    [string]$Model = 'local',
    [string]$ApiKey = '',
    [int]$NVerifications = 4,
    [int]$Criteria = 4,
    [int]$MaxWorkers = 8,
    [string]$Fixture = '',
    [switch]$NoExitCode
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Runner = Join-Path $PSScriptRoot 'eval-verifier\run_cleanup.py'

if (-not (Test-Path $Runner)) {
    Write-Host "eval-verifier-cleanup runner not found at $Runner" -ForegroundColor Red
    exit 2
}

$python = (Get-Command python -ErrorAction SilentlyContinue) ??
          (Get-Command python3 -ErrorAction SilentlyContinue)
if (-not $python) {
    Write-Host "python not found on PATH. Install Python 3.9+ or 'python' shim." -ForegroundColor Red
    exit 2
}

$argList = @(
    $Runner,
    '--backend', $Backend,
    '--n-verifications', $NVerifications,
    '--criteria', $Criteria,
    '--max-workers', $MaxWorkers
)
if ($Backend -eq 'openai-compat') {
    $argList += @('--base-url', $BaseUrl, '--model', $Model)
    if (-not [string]::IsNullOrWhiteSpace($ApiKey)) {
        $argList += @('--api-key', $ApiKey)
    }
}
if (-not [string]::IsNullOrWhiteSpace($Fixture)) {
    $argList += @('--fixture', $Fixture)
}
if ($NoExitCode) {
    $argList += '--no-exit-code'
}

Write-Host "[eval-verifier-cleanup] $($python.Source) $($argList -join ' ')" -ForegroundColor DarkGray
& $python.Source @argList
exit $LASTEXITCODE
