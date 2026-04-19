<#
.SYNOPSIS
    Multi-backend parity eval (p2-eval-multi-backend-parity).

.DESCRIPTION
    For each parity fixture (src-tauri/tests/fixtures/parity/*.wav), loads
    the oracle word timings and each available backend's cached
    NormalizedTranscriptionResult output, aligns the transcribed words
    against the oracle (text + ordinal position via simple Levenshtein-
    anchored matching), and reports:

      per-backend boundary error vs oracle:
          median, p50, p95, p99   (microseconds; threshold gates)

      cross-backend parity:
          delete-word-N export: seam count, total duration delta
          (two backends must produce the same seam count and <= 20 ms
          duration delta on the same edit)

    Why this exists
    ---------------
    * transcript-precision-eval asserts internal invariants on a single
      backend's word list.
    * audio-boundary-eval asserts per-seam acoustic quality on a single
      splice.
    * This eval is the capstone: it compares each backend against an
      independent oracle and against every other backend, so that
      swapping backends cannot silently degrade word timing or
      silently reshape the export timeline.

    Gate thresholds (mirror AGENTS.md precision gates):

      G1  median boundary error      <= 20 000 us per backend
      G2  p95 boundary error         <= 40 000 us per backend
      G3  cross-backend seam count parity on same edit (==)
      G4  cross-backend duration delta on same edit <= 20 000 us

    Backend availability
    --------------------
    The runner discovers backends by inspecting
    ``src-tauri/tests/fixtures/parity/backend_outputs/<backend>/``.
    A backend is "available for fixture X" iff that directory contains
    ``X.result.json``. Missing backends emit a ``skip`` with reason;
    without ``-StrictMode`` skip exits 0, with it skip promotes to fail.

    Live-adapter wiring (running whisper/parakeet against the .wav from
    this script) is intentionally NOT done here — it belongs in the
    eval-harness-runner agent which has access to the app. This runner
    scores whatever the harness (or a developer) has cached.
#>

[CmdletBinding()]
param(
    [string]$Fixture,
    [switch]$StrictMode,
    [ValidateSet('','equal-duration','pre-speech-padding','authoritative-lie')]
    [string]$ForceRegression = '',
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\eval\output\multi-backend-parity')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot     = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$FixturesRoot = Join-Path $RepoRoot 'src-tauri\tests\fixtures\parity'
$BackendsRoot = Join-Path $FixturesRoot 'backend_outputs'

# Thresholds
$Th = @{
    MedianUs          = 20000
    P95Us             = 40000
    CrossDurationUs   = 20000
}

# --- Oracle + backend loaders ---------------------------------------------
function Get-Fixtures {
    param([string]$Only)
    $wavs = Get-ChildItem -Path $FixturesRoot -Filter '*.wav' | Sort-Object Name
    $out = @()
    foreach ($w in $wavs) {
        $stem = [System.IO.Path]::GetFileNameWithoutExtension($w.Name)
        if ($Only -and $stem -ne $Only) { continue }
        $oraclePath = Join-Path $FixturesRoot "$stem.oracle.json"
        if (-not (Test-Path $oraclePath)) {
            Write-Host "  skip $stem (no oracle.json)" -ForegroundColor Yellow
            continue
        }
        $out += @{ stem = $stem; wav = $w.FullName; oracle = $oraclePath }
    }
    return ,$out
}

function Read-Oracle {
    param([string]$Path)
    $raw = Get-Content $Path -Raw | ConvertFrom-Json
    $list = @()
    foreach ($w in $raw) {
        $list += @{
            text     = [string]$w.word
            start_us = [int64]$w.start_us
            end_us   = [int64]$w.end_us
        }
    }
    return ,$list
}

function Read-BackendResult {
    param([string]$Path, [string]$Regression)
    $raw = Get-Content $Path -Raw | ConvertFrom-Json
    $list = @()
    foreach ($w in $raw.words) {
        $list += @{
            text     = [string]$w.text
            start_us = [int64]$w.start_us
            end_us   = [int64]$w.end_us
        }
    }
    $authoritative = [bool]$raw.word_timestamps_authoritative

    # Negative-test modes: mutate the hypothesis to prove gates fire.
    if ($Regression -eq 'equal-duration' -and $list.Count -gt 0) {
        # Spread total duration evenly across all words — classic whisper
        # char-split synthesis regression. Destroys per-word timing fidelity.
        $firstStart = [int64]$list[0].start_us
        $lastEnd    = [int64]$list[-1].end_us
        $total      = $lastEnd - $firstStart
        $each       = [int64]($total / $list.Count)
        for ($k = 0; $k -lt $list.Count; $k++) {
            $list[$k].start_us = [int64]($firstStart + $k * $each)
            $list[$k].end_us   = [int64]($firstStart + ($k + 1) * $each)
        }
    } elseif ($Regression -eq 'pre-speech-padding' -and $list.Count -gt 0) {
        # Shift every word start 60 ms earlier (padding leaks in).
        for ($k = 0; $k -lt $list.Count; $k++) {
            $list[$k].start_us = [int64]([Math]::Max(0, $list[$k].start_us - 60000))
        }
    } elseif ($Regression -eq 'authoritative-lie') {
        # Claim authoritative but emit proportional timings — lies about
        # word_timestamps_authoritative. The gate catches this because
        # p95 will blow past 40 ms vs oracle.
        $firstStart = [int64]$list[0].start_us
        $lastEnd    = [int64]$list[-1].end_us
        $total      = $lastEnd - $firstStart
        $charsTotal = 0; foreach ($w in $list) { $charsTotal += [Math]::Max(1, $w.text.Length) }
        $cursor = $firstStart
        foreach ($w in $list) {
            $share = [int64]([Math]::Round($total * ([double]([Math]::Max(1, $w.text.Length)) / [double]$charsTotal)))
            $w.start_us = [int64]$cursor
            $w.end_us   = [int64]($cursor + $share)
            $cursor = [int64]($cursor + $share)
        }
        $authoritative = $true
    }

    return @{
        words         = $list
        authoritative = $authoritative
        language      = [string]$raw.language
        regression    = $Regression
    }
}

function Get-AvailableBackends {
    if (-not (Test-Path $BackendsRoot)) { return @() }
    Get-ChildItem -Path $BackendsRoot -Directory | ForEach-Object { $_.Name } | Sort-Object
}

# --- Alignment ------------------------------------------------------------
# Levenshtein-anchored word alignment: align hypothesis words to oracle
# words by text (case-insensitive, punctuation-stripped) using a standard
# edit-distance traceback. Only matched pairs contribute to boundary
# error stats; substitutions/insertions/deletions are counted separately.
function Get-NormalizedText {
    param([string]$S)
    $t = $S.ToLowerInvariant()
    # strip anything non-alphanumeric
    return ($t -replace '[^a-z0-9]', '')
}

function Invoke-WordAlign {
    param([array]$Oracle, [array]$Hyp)
    $m = $Oracle.Count; $n = $Hyp.Count
    $refN = New-Object 'string[]' $m
    $hypN = New-Object 'string[]' $n
    for ($i = 0; $i -lt $m; $i++) { $refN[$i] = Get-NormalizedText $Oracle[$i].text }
    for ($j = 0; $j -lt $n; $j++) { $hypN[$j] = Get-NormalizedText $Hyp[$j].text }

    $d = New-Object 'int[,]' ($m + 1), ($n + 1)
    for ($i = 0; $i -le $m; $i++) { $d.SetValue([int]$i, $i, 0) }
    for ($j = 0; $j -le $n; $j++) { $d.SetValue([int]$j, 0, $j) }
    for ($i = 1; $i -le $m; $i++) {
        for ($j = 1; $j -le $n; $j++) {
            $cost = if ($refN[$i - 1] -eq $hypN[$j - 1]) { 0 } else { 1 }
            $delC = [int]$d.GetValue($i - 1, $j) + 1
            $insC = [int]$d.GetValue($i, $j - 1) + 1
            $subC = [int]$d.GetValue($i - 1, $j - 1) + $cost
            $best = [Math]::Min([Math]::Min($delC, $insC), $subC)
            $d.SetValue([int]$best, $i, $j)
        }
    }

    $pairs = New-Object 'System.Collections.Generic.List[object]'
    $subs = 0; $ins = 0; $dels = 0
    $i = $m; $j = $n
    while ($i -gt 0 -or $j -gt 0) {
        $cur =
            if ($i -gt 0 -and $j -gt 0) { [int]$d.GetValue($i, $j) }
            elseif ($i -gt 0) { [int]$d.GetValue($i, 0) }
            else { [int]$d.GetValue(0, $j) }
        if ($i -gt 0 -and $j -gt 0 -and $refN[$i - 1] -eq $hypN[$j - 1] -and $cur -eq [int]$d.GetValue($i - 1, $j - 1)) {
            [void]$pairs.Add(@{ ref_index = $i - 1; hyp_index = $j - 1; kind = 'match' })
            $i--; $j--
        } elseif ($i -gt 0 -and $j -gt 0 -and $cur -eq [int]$d.GetValue($i - 1, $j - 1) + 1) {
            $subs++; $i--; $j--
        } elseif ($j -gt 0 -and $cur -eq [int]$d.GetValue($i, $j - 1) + 1) {
            $ins++; $j--
        } else {
            $dels++; $i--
        }
    }
    $pairs.Reverse()
    return @{ matches = @($pairs.ToArray()); subs = $subs; ins = $ins; dels = $dels }
}

# --- Stats ----------------------------------------------------------------
function Get-Percentile {
    param([double[]]$Values, [double]$P)
    if ($null -eq $Values -or $Values.Length -eq 0) { return 0.0 }
    $sorted = @($Values | Sort-Object)
    $n = $sorted.Length
    $idx = [Math]::Min($n - 1, [Math]::Max(0, [int][Math]::Ceiling(($P / 100.0) * $n) - 1))
    return [double]$sorted[$idx]
}

function Invoke-BackendEval {
    param([array]$Oracle, [hashtable]$Backend, [string]$BackendName)
    $align = Invoke-WordAlign -Oracle $Oracle -Hyp $Backend.words
    $startErrs = New-Object 'System.Collections.Generic.List[double]'
    $endErrs   = New-Object 'System.Collections.Generic.List[double]'
    $perWord   = @()
    foreach ($m in $align.matches) {
        $r = $Oracle[[int]$m.ref_index]; $h = $Backend.words[[int]$m.hyp_index]
        $rs = [int64]$r.start_us; $re = [int64]$r.end_us
        $hs = [int64]$h.start_us; $he = [int64]$h.end_us
        $sErr = [Math]::Abs([double]($hs - $rs))
        $eErr = [Math]::Abs([double]($he - $re))
        [void]$startErrs.Add($sErr); [void]$endErrs.Add($eErr)
        $perWord += @{
            word            = [string]$r.text
            oracle_start_us = $rs
            oracle_end_us   = $re
            hyp_start_us    = $hs
            hyp_end_us      = $he
            start_err_us    = [int64]$sErr
            end_err_us      = [int64]$eErr
        }
    }
    # Combined errors (start + end together).
    $allErr = @($startErrs) + @($endErrs)
    $allArr = [double[]]$allErr
    $med  = Get-Percentile -Values $allArr -P 50
    $p95  = Get-Percentile -Values $allArr -P 95
    $p99  = Get-Percentile -Values $allArr -P 99
    $g1 = $med -le $Th.MedianUs
    $g2 = $p95 -le $Th.P95Us
    $status = if ($g1 -and $g2) { 'pass' } else { 'fail' }
    return @{
        backend         = $BackendName
        status          = $status
        authoritative   = $Backend.authoritative
        language        = $Backend.language
        matched_words   = $align.matches.Count
        substitutions   = $align.subs
        insertions      = $align.ins
        deletions       = $align.dels
        median_err_us   = [int64]$med
        p95_err_us      = [int64]$p95
        p99_err_us      = [int64]$p99
        threshold_median_us = $Th.MedianUs
        threshold_p95_us    = $Th.P95Us
        per_word        = $perWord
    }
}

# Cross-backend: simulate deleting word index K on each backend's word list
# and compare the resulting keep-segment count + total duration. Seam count
# = number of internal seams in the exported timeline (one less than
# kept-segment count). Duration delta is between backends.
function Invoke-CrossBackendEdit {
    param([hashtable]$A, [hashtable]$B, [int]$DeleteIndex)
    function Get-KeepStats {
        param([array]$Words, [int]$DeleteIndex)
        if ($Words.Count -eq 0) { return @{ seams = 0; duration_us = 0 } }
        # Keep-segments are [first.start_us .. deleted.start_us] +
        # [deleted.end_us .. last.end_us] — i.e. delete one word, splice
        # the neighbors. Seams = number of internal boundaries = kept
        # segments - 1. We treat each maximal contiguous kept range as
        # one segment.
        if ($DeleteIndex -lt 0 -or $DeleteIndex -ge $Words.Count) {
            $dur = [int64]($Words[-1].end_us - $Words[0].start_us)
            return @{ seams = 0; duration_us = $dur }
        }
        $segments = @()
        if ($DeleteIndex -gt 0) {
            $segments += @{ start = [int64]$Words[0].start_us; end = [int64]$Words[$DeleteIndex - 1].end_us }
        }
        if ($DeleteIndex -lt $Words.Count - 1) {
            $segments += @{ start = [int64]$Words[$DeleteIndex + 1].start_us; end = [int64]$Words[-1].end_us }
        }
        $dur = 0L
        foreach ($s in $segments) { $dur += ($s.end - $s.start) }
        $seams = [Math]::Max(0, $segments.Count - 1)
        return @{ seams = $seams; duration_us = [int64]$dur }
    }
    $sa = Get-KeepStats -Words $A.words -DeleteIndex $DeleteIndex
    $sb = Get-KeepStats -Words $B.words -DeleteIndex $DeleteIndex
    $seamOk    = ($sa.seams -eq $sb.seams)
    $durDelta  = [int64][Math]::Abs($sa.duration_us - $sb.duration_us)
    $durOk     = $durDelta -le $Th.CrossDurationUs
    $status    = if ($seamOk -and $durOk) { 'pass' } else { 'fail' }
    return @{
        status             = $status
        delete_index       = $DeleteIndex
        seams_a            = $sa.seams
        seams_b            = $sb.seams
        duration_us_a      = $sa.duration_us
        duration_us_b      = $sb.duration_us
        duration_delta_us  = $durDelta
        threshold_duration_us = $Th.CrossDurationUs
    }
}

# --- Driver ---------------------------------------------------------------
$fixtures = Get-Fixtures -Only $Fixture
if ($fixtures.Count -eq 0) {
    Write-Host "No parity fixtures found under $FixturesRoot" -ForegroundColor Red
    if ($StrictMode) { exit 1 } else { exit 0 }
}

$availableBackends = Get-AvailableBackends
Write-Host ("Fixtures: {0}" -f ($fixtures | ForEach-Object { $_.stem }) -join ', ') -ForegroundColor DarkGray
Write-Host ("Backends discovered under backend_outputs/: {0}" -f ($(if ($availableBackends.Count) { $availableBackends -join ', ' } else { '<none>' }))) -ForegroundColor DarkGray

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$overallStatus = 'pass'
$report = [ordered]@{
    timestamp     = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    thresholds    = $Th
    strict        = [bool]$StrictMode
    fixtures      = @()
}

foreach ($fx in $fixtures) {
    $oracle = Read-Oracle -Path $fx.oracle
    $perBackend = @()
    $loadedBackends = @{}
    foreach ($be in $availableBackends) {
        $rp = Join-Path $BackendsRoot "$be\$($fx.stem).result.json"
        if (-not (Test-Path $rp)) {
            $perBackend += @{
                backend = $be; status = 'skip'; reason = "no $rp"
            }
            continue
        }
        try {
            $br = Read-BackendResult -Path $rp -Regression $ForceRegression
            $loadedBackends[$be] = $br
            $res = Invoke-BackendEval -Oracle $oracle -Backend $br -BackendName $be
            $perBackend += $res
        } catch {
            $perBackend += @{ backend = $be; status = 'error'; reason = $_.Exception.Message; stack = $_.ScriptStackTrace }
        }
    }

    # Cross-backend pairs (delete middle-ish word). Only emit if >= 2
    # backends loaded successfully.
    $crossPairs = @()
    $loadedNames = @($loadedBackends.Keys | Sort-Object)
    if ($loadedNames.Count -ge 2) {
        $midIdx = [int]($oracle.Count / 2)
        for ($i = 0; $i -lt $loadedNames.Count; $i++) {
            for ($j = $i + 1; $j -lt $loadedNames.Count; $j++) {
                $na = $loadedNames[$i]; $nb = $loadedNames[$j]
                $cp = Invoke-CrossBackendEdit -A $loadedBackends[$na] -B $loadedBackends[$nb] -DeleteIndex $midIdx
                $cp['backend_a'] = $na; $cp['backend_b'] = $nb
                $crossPairs += $cp
            }
        }
    }

    $fxFail = @($perBackend | Where-Object { $_.status -eq 'fail' -or $_.status -eq 'error' }).Count
    $fxSkip = @($perBackend | Where-Object { $_.status -eq 'skip' }).Count
    $crossFail = @($crossPairs | Where-Object { $_.status -eq 'fail' }).Count

    # Overall per-fixture status:
    $fxStatus =
        if ($fxFail -gt 0 -or $crossFail -gt 0) { 'fail' }
        elseif ($loadedNames.Count -eq 0) { 'skip' }
        elseif ($StrictMode -and $fxSkip -gt 0) { 'fail' }
        else { 'pass' }

    if ($fxStatus -eq 'fail') { $overallStatus = 'fail' }
    elseif ($fxStatus -eq 'skip' -and $overallStatus -eq 'pass' -and $StrictMode) { $overallStatus = 'fail' }

    $report.fixtures += [ordered]@{
        fixture         = $fx.stem
        status          = $fxStatus
        oracle_words    = $oracle.Count
        backends_scored = $loadedNames
        per_backend     = $perBackend
        cross_backend   = $crossPairs
    }
}

# Emit reports
$runDir = Join-Path $OutputRoot $stamp
[void](New-Item -ItemType Directory -Force -Path $runDir)
$jsonPath = Join-Path $runDir 'report.json'
$mdPath   = Join-Path $runDir 'report.md'
$report | ConvertTo-Json -Depth 12 | Set-Content -Path $jsonPath -Encoding UTF8

$md = @()
$md += "# multi-backend parity eval"
$md += ""
$md += "Status: **$overallStatus**  (strict=$($report.strict))"
$md += ""
$md += "Thresholds: median <= $($Th.MedianUs) us, p95 <= $($Th.P95Us) us, cross-backend duration delta <= $($Th.CrossDurationUs) us."
$md += ""
foreach ($f in $report.fixtures) {
    $md += "## $($f.fixture) — $($f.status)"
    $md += ""
    $md += "Oracle words: $($f.oracle_words). Backends scored: $(if($f.backends_scored.Count){ ($f.backends_scored -join ', ') } else { '<none>' })"
    $md += ""
    $md += "### Per-backend"
    $md += ""
    $md += "| Backend | Status | Authoritative | Matched | Median us | p95 us | p99 us |"
    $md += "| --- | --- | --- | --- | --- | --- | --- |"
    foreach ($b in $f.per_backend) {
        if ($b.status -eq 'pass' -or $b.status -eq 'fail') {
            $md += "| $($b.backend) | $($b.status) | $($b.authoritative) | $($b.matched_words) | $($b.median_err_us) | $($b.p95_err_us) | $($b.p99_err_us) |"
        } else {
            $md += "| $($b.backend) | $($b.status) | - | - | - | - | - |"
        }
    }
    $md += ""
    if ($f.cross_backend.Count -gt 0) {
        $md += "### Cross-backend (delete word)"
        $md += ""
        $md += "| A | B | Status | Seams A | Seams B | Duration Delta us |"
        $md += "| --- | --- | --- | --- | --- | --- |"
        foreach ($c in $f.cross_backend) {
            $md += "| $($c.backend_a) | $($c.backend_b) | $($c.status) | $($c.seams_a) | $($c.seams_b) | $($c.duration_delta_us) |"
        }
        $md += ""
    }
}
($md -join "`n") | Set-Content -Path $mdPath -Encoding UTF8

Write-Host ""
Write-Host "=== multi-backend parity eval ===" -ForegroundColor Cyan
foreach ($f in $report.fixtures) {
    $c = if ($f.status -eq 'pass') { 'Green' } elseif ($f.status -eq 'fail') { 'Red' } else { 'Yellow' }
    Write-Host ("  [{0}] {1}" -f $f.status.ToUpper(), $f.fixture) -ForegroundColor $c
    foreach ($b in $f.per_backend) {
        $bc = if ($b.status -eq 'pass') { 'Green' } elseif ($b.status -eq 'fail') { 'Red' } else { 'DarkYellow' }
        if ($b.status -eq 'pass' -or $b.status -eq 'fail') {
            Write-Host ("      - {0,-12} {1,-4}  median={2,7} us  p95={3,7} us  matched={4}" -f `
                $b.backend, $b.status, $b.median_err_us, $b.p95_err_us, $b.matched_words) -ForegroundColor $bc
        } else {
            Write-Host ("      - {0,-12} {1,-4}  ({2})" -f $b.backend, $b.status, $b.reason) -ForegroundColor $bc
        }
    }
    foreach ($c in $f.cross_backend) {
        $cc = if ($c.status -eq 'pass') { 'Green' } else { 'Red' }
        Write-Host ("      x {0}<->{1}  {2}  seams={3}/{4}  dur_delta={5} us" -f `
            $c.backend_a, $c.backend_b, $c.status, $c.seams_a, $c.seams_b, $c.duration_delta_us) -ForegroundColor $cc
    }
}
Write-Host ""
$overallColor = if ($overallStatus -eq 'pass') { 'Green' } else { 'Red' }
Write-Host ("Overall: {0}" -f $overallStatus.ToUpper()) -ForegroundColor $overallColor
Write-Host ("Report:  {0}" -f $jsonPath) -ForegroundColor DarkGray

if ($overallStatus -ne 'pass') { exit 1 }
exit 0
