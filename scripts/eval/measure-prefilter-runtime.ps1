<#
.SYNOPSIS
    R-002 prefilter QC helper — guides operator through an
    A/B comparison of transcription with `vad_prefilter_enabled`
    OFF then ON.

.DESCRIPTION
    The runtime delta gate (G6 in eval-vad.ps1) requires a real
    Whisper model + a real fixture, both of which are too heavy to
    embed in CI. This script is the human-in-the-loop substitute:

      1. Snapshot current settings.json values.
      2. Force `vad_prefilter_enabled = false`.
      3. Launch Toaster monitored, prompt the operator to transcribe
         the fixture, record wall-clock from launch to first
         "Transcription completed in N ms" log line.
      4. Force `vad_prefilter_enabled = true`.
      5. Launch again, prompt for a second transcription of the
         **same** fixture, record wall-clock.
      6. Restore original settings, write a journal under
         `eval/output/vad/runtime-delta/<UTC>/report.md`.

    The journal is the QC artifact PR reviewers cite for AC-002-b/c.
    The expected outcome on a silence-heavy clip (≥ 30 % silence) is
    an inverse runtime ratio < 1.0 (prefilter ON is faster). On a
    pure-speech clip the ratio is ~1.0 ± noise — both are valid
    feature signals, the journal just needs to *record* them.

.PARAMETER Fixture
    Optional human-readable fixture description. Recorded in the
    journal so two runs can be compared. The script never opens or
    reads the fixture — all transcription happens inside the live
    app driven by the operator. Defaults to
    `eval/fixtures/toaster_example.mp4`.

.PARAMETER LaunchDuration
    Per-run launch duration passed to launch-toaster-monitored.ps1.
    Default: `5m` (matches the launcher's own default).

.NOTES
    Mirrors `verify-vad-settings-live.ps1` (AC-006-c) in shape: snapshot
    + journal under `eval/output/vad/<gate>/<UTC>/`. Settings file
    location matches `commands::app_settings::settings_path()`:
    `%APPDATA%\com.toaster.app\settings.json`.
#>

[CmdletBinding()]
param(
    [string]$Fixture = 'eval/fixtures/toaster_example.mp4',
    [string]$LaunchDuration = '5m'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$SettingsPath = Join-Path $env:APPDATA 'com.toaster.app\settings_store.json'
$Launcher = Join-Path $RepoRoot 'scripts\launch-toaster-monitored.ps1'

if (-not (Test-Path $SettingsPath)) {
    throw "Settings file not found: $SettingsPath. Launch the app at least once before running this gate."
}
if (-not (Test-Path $Launcher)) {
    throw "Launcher script not found: $Launcher"
}

$utc = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ')
$outDir = Join-Path $RepoRoot "eval\output\vad\runtime-delta\$utc"
New-Item -ItemType Directory -Path $outDir -Force | Out-Null

# settings_store.json is the Tauri-store wrapper:
#   { "settings": { ...AppSettings... } }
# All edits go through the inner ``.settings`` object.
function Read-Settings { Get-Content -Raw -Path $SettingsPath | ConvertFrom-Json }

function Save-Settings([psobject]$obj) {
    ($obj | ConvertTo-Json -Depth 50) | Set-Content -Path $SettingsPath -Encoding UTF8
}

function Get-Inner([psobject]$root) { $root.settings }

$snapshot = Read-Settings
$inner = Get-Inner $snapshot
# Older snapshots may pre-date the VAD toggles; treat absent as the
# Rust-side default (false for prefilter, true for boundary refine).
$origPrefilter = if ($inner.PSObject.Properties.Match('vad_prefilter_enabled').Count) {
    [bool]$inner.vad_prefilter_enabled
} else { $false }
$origRefine = if ($inner.PSObject.Properties.Match('vad_refine_boundaries').Count) {
    [bool]$inner.vad_refine_boundaries
} else { $true }

function Set-Prefilter([bool]$enabled) {
    $s = Read-Settings
    $inner = Get-Inner $s
    if ($inner.PSObject.Properties.Match('vad_prefilter_enabled').Count) {
        $inner.vad_prefilter_enabled = $enabled
    } else {
        $inner | Add-Member -NotePropertyName 'vad_prefilter_enabled' -NotePropertyValue $enabled -Force
    }
    Save-Settings $s
}

function Invoke-Run([string]$label, [bool]$enabled) {
    Set-Prefilter $enabled
    Write-Host ""
    Write-Host "============================================================"
    Write-Host " $label  (vad_prefilter_enabled = $enabled)"
    Write-Host "============================================================"
    Write-Host "  1. Toaster will launch monitored for $LaunchDuration."
    Write-Host "  2. Open the fixture: $Fixture"
    Write-Host "  3. Transcribe it (model auto-loads on first run)."
    Write-Host "  4. Note the wall-clock from app start to the moment the"
    Write-Host "     transcript appears."
    Write-Host "  5. Close the app cleanly OR let the launcher exit."
    Write-Host ""
    Read-Host "Press <enter> to launch"

    $start = Get-Date
    & $Launcher -Duration $LaunchDuration *>&1 | Tee-Object -FilePath (Join-Path $outDir "$label.log") | Out-Null
    $end = Get-Date
    $elapsed = ($end - $start).TotalSeconds
    Write-Host ("  -> wall-clock: {0:N1} s" -f $elapsed)

    [pscustomobject]@{
        label                  = $label
        vad_prefilter_enabled  = $enabled
        wall_clock_seconds     = [math]::Round($elapsed, 1)
        log                    = "$label.log"
        started_at_utc         = $start.ToUniversalTime().ToString('o')
        ended_at_utc           = $end.ToUniversalTime().ToString('o')
    }
}

try {
    $runOff = Invoke-Run 'A_prefilter_off' $false
    $runOn  = Invoke-Run 'B_prefilter_on'  $true
}
finally {
    # Restore both VAD toggles to their pre-run state. This is
    # deliberately in `finally` so an aborted run doesn't strand the
    # operator with an unexpected toggle position.
    $s = Read-Settings
    $inner = Get-Inner $s
    if ($inner.PSObject.Properties.Match('vad_prefilter_enabled').Count) {
        $inner.vad_prefilter_enabled = $origPrefilter
    } else {
        $inner | Add-Member -NotePropertyName 'vad_prefilter_enabled' -NotePropertyValue $origPrefilter -Force
    }
    if ($inner.PSObject.Properties.Match('vad_refine_boundaries').Count) {
        $inner.vad_refine_boundaries = $origRefine
    } else {
        $inner | Add-Member -NotePropertyName 'vad_refine_boundaries' -NotePropertyValue $origRefine -Force
    }
    Save-Settings $s
    Write-Host ""
    Write-Host "Restored vad_prefilter_enabled=$origPrefilter, vad_refine_boundaries=$origRefine"
}

$ratio = if ($runOff.wall_clock_seconds -gt 0) {
    [math]::Round($runOn.wall_clock_seconds / $runOff.wall_clock_seconds, 3)
} else { $null }

$report = [ordered]@{
    timestamp      = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    feature        = 'reintroduce-silero-vad'
    gate           = 'G6_runtime_delta'
    ac             = 'AC-002-b/c/d'
    fixture        = $Fixture
    runs           = @($runOff, $runOn)
    ratio_on_over_off = $ratio
    note           = 'Operator wall-clock includes app launch + model load. Compare ratio_on_over_off across runs of the same fixture. < 1.0 = prefilter faster, ~1.0 = neutral.'
}

$jsonPath = Join-Path $outDir 'report.json'
$mdPath   = Join-Path $outDir 'report.md'
($report | ConvertTo-Json -Depth 10) | Set-Content -Path $jsonPath -Encoding UTF8

$md = @()
$md += "# R-002 prefilter runtime delta"
$md += ""
$md += "- **Feature:** reintroduce-silero-vad"
$md += "- **Gate:** G6_runtime_delta (AC-002-b/c/d)"
$md += "- **Fixture:** ``$Fixture``"
$md += "- **UTC:** $utc"
$md += ""
$md += "## Runs"
$md += ""
$md += "| label | vad_prefilter_enabled | wall_clock_seconds |"
$md += "|---|---|---|"
foreach ($r in $report.runs) {
    $md += "| $($r.label) | $($r.vad_prefilter_enabled) | $($r.wall_clock_seconds) |"
}
$md += ""
$md += "**ratio_on_over_off:** ``$ratio``"
$md += ""
$md += "Cite this report in the PR body alongside the eval-vad.ps1 G9_prefilter_live_wired pass."
$md -join "`n" | Set-Content -Path $mdPath -Encoding UTF8

Write-Host ""
Write-Host "Report : $mdPath"
Write-Host "JSON   : $jsonPath"
