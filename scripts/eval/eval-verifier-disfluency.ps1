<#
.SYNOPSIS
    PowerShell wrapper for scripts/eval-verifier/run_disfluency.py.

.DESCRIPTION
    Runs the LLM-as-a-Verifier ranker over disfluency fixtures in
    src-tauri/tests/fixtures/disfluency/. Candidates are competing
    "which repetition to keep" decisions. The ranker scores group
    collapse completeness, audio-aware survivor clarity, audio-aware
    cut placement cleanliness, and timing monotonicity — then compares
    the tournament winner to the fixture's expected_winner.

    Default backend is "mock" (deterministic, no network).
#>

[CmdletBinding()]
param(
    [ValidateSet('mock', 'openai-compat', 'gemini')]
    [string]$Backend = 'mock',
    [string]$BaseUrl = 'http://127.0.0.1:8080/v1',
    [string]$Model = 'local',
    [string]$ApiKey = '',
    [int]$NVerifications = 4,
    [int]$Criteria = 5,
    [int]$MaxWorkers = 8,
    [string]$Fixture = '',
    [switch]$NoExitCode
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Runner = Join-Path $PSScriptRoot 'eval-verifier\run_disfluency.py'

if (-not (Test-Path $Runner)) {
    Write-Host "eval-verifier-disfluency runner not found at $Runner" -ForegroundColor Red
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

Write-Host "[eval-verifier-disfluency] $($python.Source) $($argList -join ' ')" -ForegroundColor DarkGray
& $python.Source @argList
exit $LASTEXITCODE
