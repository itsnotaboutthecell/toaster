<#
.SYNOPSIS
    PowerShell wrapper for scripts/eval-verifier/run_parity.py.

.DESCRIPTION
    Runs the LLM-as-a-Verifier Best-of-N parity ranker over the fixtures in
    src-tauri/tests/fixtures/parity/. Emits JSON + markdown under
    eval/output/verifier-parity/<stamp>/ and exits non-zero on failure.

    Default backend is "mock" (deterministic, no network) so this is safe to
    wire into CI alongside eval-multi-backend-parity without adding a hosted
    API dependency.

    To use a real local model, point -Backend openai-compat at a running
    llama.cpp / vLLM server:

        .\scripts\eval-verifier.ps1 -Backend openai-compat -BaseUrl http://127.0.0.1:8080/v1

    To use Gemini for upstream-parity development only:

        $env:GEMINI_API_KEY = "<key>"
        .\scripts\eval-verifier.ps1 -Backend gemini

    NOTE: the gemini backend is CI-only. It MUST NOT be wired into the Tauri
    runtime — AGENTS.md forbids runtime network calls to hosted LLM APIs.
#>

[CmdletBinding()]
param(
    [ValidateSet('mock', 'openai-compat', 'gemini')]
    [string]$Backend = 'mock',
    [string]$BaseUrl = 'http://127.0.0.1:8080/v1',
    [string]$Model = 'local',
    [string]$ApiKey = '',
    [int]$NVerifications = 4,
    [int]$Criteria = 3,
    [int]$MaxWorkers = 8,
    [string]$Fixture = '',
    [switch]$NoExitCode
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$Runner   = Join-Path $PSScriptRoot 'eval-verifier\run_parity.py'

if (-not (Test-Path $Runner)) {
    Write-Host "eval-verifier runner not found at $Runner" -ForegroundColor Red
    exit 2
}

$python = (Get-Command python -ErrorAction SilentlyContinue) ??
          (Get-Command python3 -ErrorAction SilentlyContinue)
if (-not $python) {
    Write-Host "python not found on PATH. Install Python 3.9+ or `python` shim." -ForegroundColor Red
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

Write-Host "[eval-verifier] $($python.Source) $($argList -join ' ')" -ForegroundColor DarkGray
& $python.Source @argList
exit $LASTEXITCODE
