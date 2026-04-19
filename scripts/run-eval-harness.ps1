<#
.SYNOPSIS
    Runs the Toaster precision / midstream / export evals in a deterministic
    order and emits a single JSON report at the path specified by -OutputJson.

.DESCRIPTION
    Wraps the three eval entry points referenced by the eval-harness-runner
    agent (see .github/agents/eval-harness-runner.md):

      1. Rust precision eval        -> cargo test precision_eval
      2. Audio-boundary eval         -> scripts/eval-audio-boundary.ps1
      3. Export parity eval          -> scripts/eval-edit-quality.ps1

    Evals that require running app state or fixtures that are not yet
    available are reported as status="skip" with a reason, NOT silently
    omitted. Exit code is non-zero iff overall != "pass".

.PARAMETER OutputJson
    Path to write the JSON report. Defaults to .eval-output/eval-harness-report.json.

.PARAMETER SkipAudioBoundary
    Skip the audio-boundary check (useful when boundary fixtures are missing).

.PARAMETER SkipExportParity
    Skip the export parity check (useful when fixtures / baseline missing).

.NOTES
    Tracked as p5-eval-runner-agent-wire.
#>

[CmdletBinding()]
param(
    [string]$OutputJson = (Join-Path $PSScriptRoot '..\.eval-output\eval-harness-report.json'),
    [switch]$SkipAudioBoundary,
    [switch]$SkipExportParity
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

function New-EvalEntry {
    param(
        [string]$Name,
        [string]$Command,
        [string]$Status,
        [double]$DurationS,
        [hashtable]$Details = @{},
        [string]$Notes = ''
    )
    [ordered]@{
        name       = $Name
        command    = $Command
        status     = $Status
        duration_s = [math]::Round($DurationS, 3)
        details    = $Details
        notes      = $Notes
    }
}

$outDir = Split-Path -Parent $OutputJson
if (-not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
}

$evals = @()

# --- 1. Rust precision eval -----------------------------------------------
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$precisionStatus = 'error'
$precisionDetails = @{ passed = 0; failed = 0; filtered = 0 }
$precisionNotes = ''
try {
    Push-Location (Join-Path $RepoRoot 'src-tauri')
    $output = cargo test -p toaster --lib precision_eval -- --nocapture 2>&1 | Out-String
    $resultLine = ($output -split "`n") | Where-Object { $_ -match 'test result:' } | Select-Object -Last 1
    if ($resultLine -match 'test result: ok\. (\d+) passed; (\d+) failed; \d+ ignored; \d+ measured; (\d+) filtered') {
        $precisionDetails.passed = [int]$Matches[1]
        $precisionDetails.failed = [int]$Matches[2]
        $precisionDetails.filtered = [int]$Matches[3]
        $precisionStatus = if ($precisionDetails.failed -eq 0 -and $precisionDetails.passed -gt 0) { 'pass' } else { 'fail' }
    } else {
        $precisionNotes = "Unparseable cargo test output"
    }
} catch {
    $precisionNotes = $_.Exception.Message
} finally {
    Pop-Location
    $sw.Stop()
}
$evals += New-EvalEntry `
    -Name 'precision' `
    -Command 'cargo test -p toaster --lib precision_eval' `
    -Status $precisionStatus `
    -DurationS ($sw.Elapsed.TotalSeconds) `
    -Details $precisionDetails `
    -Notes $precisionNotes

# --- 2. Audio-boundary eval -----------------------------------------------
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$boundaryScript = Join-Path $RepoRoot 'scripts\eval-audio-boundary.ps1'
$boundaryFixturesRoot = Join-Path $RepoRoot 'src-tauri\tests\fixtures\boundary'
$boundaryDetails = @{}
if ($SkipAudioBoundary.IsPresent) {
    $boundaryStatus = 'skip'
    $boundaryNotes = '-SkipAudioBoundary flag set'
} elseif (-not (Test-Path $boundaryScript)) {
    $boundaryStatus = 'skip'
    $boundaryNotes = 'eval-audio-boundary.ps1 not present'
} elseif (-not (Test-Path $boundaryFixturesRoot)) {
    $boundaryStatus = 'skip'
    $boundaryNotes = 'boundary fixtures not present; run scripts/generate-boundary-fixtures.ps1'
} else {
    try {
        & pwsh -NoProfile -File $boundaryScript | Out-Null
        $boundaryStatus = if ($LASTEXITCODE -eq 0) { 'pass' } else { 'fail' }
        $boundaryNotes = ''
    } catch {
        $boundaryStatus = 'error'
        $boundaryNotes = $_.Exception.Message
    }
}
$sw.Stop()
$evals += New-EvalEntry `
    -Name 'audio_boundary' `
    -Command 'scripts/eval-audio-boundary.ps1' `
    -Status $boundaryStatus `
    -DurationS ($sw.Elapsed.TotalSeconds) `
    -Details $boundaryDetails `
    -Notes $boundaryNotes

# --- 3. Export parity ------------------------------------------------------
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$exportScript = Join-Path $RepoRoot 'scripts\eval-edit-quality.ps1'
$baselinePath = Join-Path $RepoRoot 'tests\fixtures\edit-quality.baseline.json'
$exportOut = Join-Path $outDir 'edit-quality.json'
$exportDetails = @{}
if ($SkipExportParity.IsPresent) {
    $exportStatus = 'skip'
    $exportNotes = '-SkipExportParity flag set'
} elseif (-not (Test-Path $exportScript)) {
    $exportStatus = 'skip'
    $exportNotes = 'eval-edit-quality.ps1 not present'
} elseif (-not (Test-Path (Join-Path $RepoRoot 'eval\fixtures\toaster_example.mp4'))) {
    $exportStatus = 'skip'
    $exportNotes = 'fixture eval/fixtures/toaster_example.mp4 missing'
} else {
    try {
        & pwsh -NoProfile -File $exportScript `
            -Original (Join-Path $RepoRoot 'eval\fixtures\toaster_example.mp4') `
            -Edited   (Join-Path $RepoRoot 'eval\fixtures\toaster_example-edited.mp4') `
            -OutputJson $exportOut | Out-Null
        if ($LASTEXITCODE -ne 0) {
            $exportStatus = 'error'
            $exportNotes = "eval-edit-quality.ps1 exited $LASTEXITCODE"
        } elseif (-not (Test-Path $baselinePath)) {
            $exportStatus = 'skip'
            $exportNotes = "baseline tests/fixtures/edit-quality.baseline.json missing; see p5-eval-export-parity"
            if (Test-Path $exportOut) { $exportDetails.output_generated = $true }
        } else {
            $current  = Get-Content $exportOut     -Raw | ConvertFrom-Json
            $baseline = Get-Content $baselinePath  -Raw | ConvertFrom-Json
            $exportDetails.duration_delta_s        = [math]::Round(($current.edited.duration_s    - $baseline.edited.duration_s), 3)
            $exportDetails.silence_gaps_delta      = ($current.edited.silence_gaps                - $baseline.edited.silence_gaps)
            $exportDetails.leading_silence_delta_s = [math]::Round(($current.edited.leading_silence  - $baseline.edited.leading_silence), 3)
            $exportDetails.trailing_silence_delta_s= [math]::Round(($current.edited.trailing_silence - $baseline.edited.trailing_silence), 3)
            $tol = 0.050
            $regressed = ([math]::Abs($exportDetails.duration_delta_s) -gt $tol) `
                         -or ($exportDetails.silence_gaps_delta -ne 0) `
                         -or ([math]::Abs($exportDetails.leading_silence_delta_s)  -gt $tol) `
                         -or ([math]::Abs($exportDetails.trailing_silence_delta_s) -gt $tol)
            $exportStatus = if ($regressed) { 'fail' } else { 'pass' }
            $exportNotes  = ''
        }
    } catch {
        $exportStatus = 'error'
        $exportNotes = $_.Exception.Message
    }
}
$sw.Stop()
$evals += New-EvalEntry `
    -Name 'export_parity' `
    -Command 'scripts/eval-edit-quality.ps1' `
    -Status $exportStatus `
    -DurationS ($sw.Elapsed.TotalSeconds) `
    -Details $exportDetails `
    -Notes $exportNotes

# --- Overall ---------------------------------------------------------------
$hasError = $evals | Where-Object { $_.status -eq 'error' }
$hasFail  = $evals | Where-Object { $_.status -eq 'fail' }
if ($hasError)    { $overall = 'error' }
elseif ($hasFail) { $overall = 'fail'  }
else              { $overall = 'pass'  }

$report = [ordered]@{
    timestamp   = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    commit      = (& git rev-parse HEAD 2>$null)
    environment = [ordered]@{
        os   = 'windows'
        rust = (& rustc --version 2>$null)
        node = (& node --version 2>$null)
    }
    evals       = $evals
    overall     = $overall
}

$report | ConvertTo-Json -Depth 10 | Set-Content -Path $OutputJson -Encoding UTF8
Write-Host ("Eval harness report: {0}  (overall={1})" -f $OutputJson, $overall)
$evals | ForEach-Object {
    Write-Host ("  - {0,-14} {1,-5}  {2}" -f $_.name, $_.status, $_.notes)
}

if ($overall -ne 'pass') { exit 1 }
exit 0
