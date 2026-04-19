<#
.SYNOPSIS
    Evaluates edit quality by comparing original and edited media files.
.DESCRIPTION
    Compares duration, silence gaps, and leading/trailing silence between
    an original and edited mp4/audio file using ffprobe and ffmpeg silencedetect.
.PARAMETER Original
    Path to the original media file.
.PARAMETER Edited
    Path to the edited media file.
.PARAMETER OutputJson
    Optional path to save the report as JSON.
.EXAMPLE
    .\scripts\eval-edit-quality.ps1 -Original "eval\fixtures\toaster_example.mp4" -Edited "eval\fixtures\toaster_example-edited.mp4"
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$Original,

    [Parameter(Mandatory = $true)]
    [string]$Edited,

    [string]$OutputJson
)

$ErrorActionPreference = "Stop"

# --- Helpers ---

function Test-Prerequisite {
    foreach ($cmd in @("ffmpeg", "ffprobe")) {
        if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
            Write-Host "ERROR: '$cmd' not found in PATH. Install FFmpeg and ensure it is on your PATH." -ForegroundColor Red
            exit 1
        }
    }
}

function Get-MediaDuration {
    param([string]$FilePath)
    $raw = & ffprobe -v error -show_entries format=duration -of csv=p=0 $FilePath 2>&1
    $val = ($raw | Out-String).Trim()
    if ($val -match '^[\d.]+$') {
        return [double]$val
    }
    Write-Host "ERROR: Could not determine duration of '$FilePath'. ffprobe output: $val" -ForegroundColor Red
    exit 1
}

function Get-SilenceGaps {
    <#
    .DESCRIPTION
        Runs ffmpeg silencedetect and returns an array of [PSCustomObject]@{ Start; End; Duration }.
        Silence noise floor: -35dB, minimum duration: 0.1s.
    #>
    param([string]$FilePath)

    $output = & ffmpeg -i $FilePath -af "silencedetect=noise=-35dB:d=0.1" -f null NUL 2>&1 | Out-String

    $gaps = [System.Collections.Generic.List[PSCustomObject]]::new()
    $pendingStart = $null

    foreach ($line in ($output -split "`n")) {
        if ($line -match 'silence_start:\s*([\d.]+)') {
            $pendingStart = [double]$Matches[1]
        }
        elseif ($line -match 'silence_end:\s*([\d.]+)\s*\|\s*silence_duration:\s*([\d.]+)') {
            $silEnd = [double]$Matches[1]
            $silDur = [double]$Matches[2]
            $silStart = if ($null -ne $pendingStart) { $pendingStart } else { $silEnd - $silDur }
            $gaps.Add([PSCustomObject]@{
                Start    = [math]::Round($silStart, 3)
                End      = [math]::Round($silEnd, 3)
                Duration = [math]::Round($silDur, 3)
            })
            $pendingStart = $null
        }
    }

    return , $gaps.ToArray()
}

function Get-LeadingTrailingSilence {
    param(
        [array]$Gaps,
        [double]$FileDuration
    )
    $leading = 0.0
    $trailing = 0.0

    if ($Gaps.Count -gt 0) {
        $first = $Gaps[0]
        if ($first.Start -le 0.05) {
            $leading = $first.Duration
        }
        $last = $Gaps[$Gaps.Count - 1]
        if (($FileDuration - $last.End) -le 0.05) {
            $trailing = $last.Duration
        }
    }
    return @{ Leading = [math]::Round($leading, 3); Trailing = [math]::Round($trailing, 3) }
}

# --- Main ---

Test-Prerequisite

foreach ($p in @(@{N="Original";V=$Original}, @{N="Edited";V=$Edited})) {
    if (-not (Test-Path $p.V)) {
        Write-Host "ERROR: $($p.N) file not found: $($p.V)" -ForegroundColor Red
        exit 1
    }
}

Write-Host "Analyzing files..." -ForegroundColor Cyan

$origDuration = Get-MediaDuration $Original
$editDuration = Get-MediaDuration $Edited
$reduction = if ($origDuration -gt 0) { (1 - $editDuration / $origDuration) * 100 } else { 0 }

$origGaps = Get-SilenceGaps $Original
$editGaps = Get-SilenceGaps $Edited

$origSilenceTotal = ($origGaps | Measure-Object -Property Duration -Sum).Sum
if ($null -eq $origSilenceTotal) { $origSilenceTotal = 0.0 }
$editSilenceTotal = ($editGaps | Measure-Object -Property Duration -Sum).Sum
if ($null -eq $editSilenceTotal) { $editSilenceTotal = 0.0 }

$origSilencePct = if ($origDuration -gt 0) { $origSilenceTotal / $origDuration * 100 } else { 0 }
$editSilencePct = if ($editDuration -gt 0) { $editSilenceTotal / $editDuration * 100 } else { 0 }

$edgeInfo = Get-LeadingTrailingSilence -Gaps $editGaps -FileDuration $editDuration

# Find worst gap
$worstGap = 0
if ($editGaps.Count -gt 0) {
    $worstGap = ($editGaps | Measure-Object -Property Duration -Maximum).Maximum
}

# --- Console Report ---

Write-Host ""
Write-Host "=== EDIT QUALITY REPORT ===" -ForegroundColor Green
Write-Host ("Original: {0:F2}s | Edited: {1:F2}s | Reduction: {2:F1}%" -f $origDuration, $editDuration, $reduction)
Write-Host ""
Write-Host "Silence Analysis (>100ms gaps at -35dB):" -ForegroundColor Yellow
Write-Host ("  Original: {0} gaps, {1:F1}s total silence ({2:F1}% of file)" -f $origGaps.Count, $origSilenceTotal, $origSilencePct)
Write-Host ("  Edited:   {0} gaps, {1:F1}s total silence ({2:F1}% of file)" -f $editGaps.Count, $editSilenceTotal, $editSilencePct)
Write-Host ""
Write-Host ("Leading silence: {0:F2}s" -f $edgeInfo.Leading)
Write-Host ("Trailing silence: {0:F2}s" -f $edgeInfo.Trailing)

if ($editGaps.Count -gt 0) {
    Write-Host ""
    Write-Host "Remaining silence gaps in edited file:" -ForegroundColor Yellow
    foreach ($g in $editGaps) {
        $ms = [int]($g.Duration * 1000)
        $warn = if ($g.Duration -ge 0.5) { " $(([char]0x26A0))$(([char]0xFE0F))" } else { "" }
        Write-Host ("  {0:F3} - {1:F3}s ({2}ms){3}" -f $g.Start, $g.End, $ms, $warn)
    }
}

Write-Host ""
if ($worstGap -ge 0.8) {
    Write-Host ("VERDICT: $(([char]0x26A0))$(([char]0xFE0F)) Edited file still has {0}ms+ dead air gaps" -f [int]($worstGap * 1000)) -ForegroundColor Red
} elseif ($worstGap -ge 0.5) {
    Write-Host ("VERDICT: $(([char]0x26A0))$(([char]0xFE0F)) Edited file has some notable silence gaps (worst: {0}ms)" -f [int]($worstGap * 1000)) -ForegroundColor Yellow
} elseif ($reduction -lt 5) {
    Write-Host "VERDICT: Minimal editing detected — less than 5% reduction" -ForegroundColor Yellow
} else {
    Write-Host "VERDICT: $(([char]0x2705)) Edit looks clean" -ForegroundColor Green
}

# --- Optional JSON output ---

if ($OutputJson) {
    $report = [ordered]@{
        original = [ordered]@{
            path             = $Original
            duration_s       = [math]::Round($origDuration, 3)
            silence_gaps     = $origGaps.Count
            silence_total_s  = [math]::Round($origSilenceTotal, 3)
            silence_pct      = [math]::Round($origSilencePct, 1)
        }
        edited = [ordered]@{
            path             = $Edited
            duration_s       = [math]::Round($editDuration, 3)
            silence_gaps     = $editGaps.Count
            silence_total_s  = [math]::Round($editSilenceTotal, 3)
            silence_pct      = [math]::Round($editSilencePct, 1)
            leading_silence  = $edgeInfo.Leading
            trailing_silence = $edgeInfo.Trailing
        }
        reduction_pct    = [math]::Round($reduction, 1)
        worst_gap_ms     = [int]($worstGap * 1000)
        gaps_detail      = @($editGaps | ForEach-Object {
            [ordered]@{
                start_s    = $_.Start
                end_s      = $_.End
                duration_ms = [int]($_.Duration * 1000)
            }
        })
    }
    $report | ConvertTo-Json -Depth 4 | Set-Content -Path $OutputJson -Encoding UTF8
    Write-Host ""
    Write-Host "JSON report saved to: $OutputJson" -ForegroundColor Cyan
}
