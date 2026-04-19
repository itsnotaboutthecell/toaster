<#
.SYNOPSIS
    Settings UI consistency audit runner.

.DESCRIPTION
    Drives the Playwright audit spec (tests/settingsUIAudit.spec.ts) across
    every settings route at desktop + mobile viewports, collates the
    emitted violations, and writes a machine-readable audit.json plus a
    human-readable audit.md into features/settings-ui-consistency-audit/
    audit-report/ (or -OutputDir). Exits 0 when the critical count is
    zero, 1 otherwise.

    Implements AC-001-c (runtime gate), AC-006-a/b/c (report artefacts +
    artifact-size cap + exit codes) for the settings-ui-consistency-audit
    bundle.

.PARAMETER Ac
    Optional AC ID. Echoed in logs so CI routing is visible; the full
    spec is always run.

.PARAMETER OutputDir
    Output directory. Defaults to
    features/settings-ui-consistency-audit/audit-report.

.PARAMETER SkipDevServerCheck
    Skip the http://localhost:1420 preflight probe. Use in CI where the
    dev server is guaranteed started by the job runner.

.EXAMPLE
    pwsh scripts/migrate/audit-settings-ui.ps1
    pwsh scripts/migrate/audit-settings-ui.ps1 -Ac AC-004-a -SkipDevServerCheck
#>
[CmdletBinding()]
param(
    [string] $Ac,
    [string] $OutputDir,
    [switch] $SkipDevServerCheck
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

if (-not (Test-Path (Join-Path $repoRoot '.git')) -or -not (Test-Path (Join-Path $repoRoot 'features'))) {
    Write-Error "Expected to run from repo root containing .git/ and features/. Got: $repoRoot"
    exit 2
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot 'features\settings-ui-consistency-audit\audit-report'
}
$OutputDir = [System.IO.Path]::GetFullPath($OutputDir)

if ($Ac) { Write-Host "[audit] AC filter (informational): $Ac" -ForegroundColor Cyan }

if (-not $SkipDevServerCheck) {
    try {
        $null = Invoke-WebRequest -Uri 'http://localhost:1420' -UseBasicParsing -TimeoutSec 5
    }
    catch {
        Write-Error @"
Dev server not reachable at http://localhost:1420.
Start it first via one of:
  .\scripts\launch-toaster-monitored.ps1 -ObservationSeconds 300
  bun run dev
Then re-run this script (or pass -SkipDevServerCheck in CI).
"@
        exit 2
    }
}

# Clean output dir contents (preserve the dir itself).
if (Test-Path $OutputDir) {
    Get-ChildItem -Path $OutputDir -Force | Remove-Item -Recurse -Force
}
else {
    New-Item -Path $OutputDir -ItemType Directory -Force | Out-Null
}

$tempDir = Join-Path $env:TEMP ("settings-ui-audit-" + [Guid]::NewGuid().ToString('N'))
New-Item -Path $tempDir -ItemType Directory -Force | Out-Null
$env:AUDIT_OUTPUT_DIR = $tempDir

Write-Host "[audit] Output dir: $OutputDir" -ForegroundColor Cyan
Write-Host "[audit] Staging dir: $tempDir" -ForegroundColor Cyan

$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
& bunx playwright test tests/settingsUIAudit.spec.ts --reporter=list --project=chromium
$playwrightExit = $LASTEXITCODE
$stopwatch.Stop()
$runtimeSec = [math]::Round($stopwatch.Elapsed.TotalSeconds, 1)
if ($runtimeSec -gt 120) {
    Write-Warning "[audit] Runtime ${runtimeSec}s exceeded 120s CI budget (AC-001-c)."
}

$rawPath = Join-Path $tempDir 'raw.json'
if (-not (Test-Path $rawPath)) {
    Write-Error "[audit] Playwright did not emit raw.json at $rawPath (exit=$playwrightExit)."
    exit 1
}

$raw = Get-Content $rawPath -Raw | ConvertFrom-Json
$violations = @($raw.violations)

$severityOrder = @{ 'critical' = 0; 'major' = 1; 'minor' = 2 }
$sorted = $violations | Sort-Object `
    @{Expression = { $_.page } }, `
    @{Expression = { $severityOrder[$_.severity] } }, `
    @{Expression = { $_.rule } }, `
    @{Expression = { $_.selector } }

$critical = ($sorted | Where-Object { $_.severity -eq 'critical' }).Count
$major = ($sorted | Where-Object { $_.severity -eq 'major' }).Count
$minor = ($sorted | Where-Object { $_.severity -eq 'minor' }).Count

$report = [ordered]@{
    schemaVersion = 1
    generatedAt   = $raw.generatedAt
    runtimeSec    = $runtimeSec
    summary       = [ordered]@{
        total    = $sorted.Count
        critical = $critical
        major    = $major
        minor    = $minor
    }
    violations    = $sorted
}
$reportJson = $report | ConvertTo-Json -Depth 12
Set-Content -Path (Join-Path $OutputDir 'audit.json') -Value $reportJson -Encoding UTF8

# Copy screenshots.
$srcShots = Join-Path $tempDir 'screenshots'
$dstShots = Join-Path $OutputDir 'screenshots'
if (Test-Path $srcShots) {
    New-Item -Path $dstShots -ItemType Directory -Force | Out-Null
    Copy-Item -Path (Join-Path $srcShots '*') -Destination $dstShots -Recurse -Force
}

# Build Markdown report.
$pages = $sorted | Group-Object page
$severities = @('critical', 'major', 'minor')

$md = New-Object System.Text.StringBuilder
[void]$md.AppendLine('# Settings UI consistency audit')
[void]$md.AppendLine('')
[void]$md.AppendLine("Generated: $($raw.generatedAt)  ")
[void]$md.AppendLine("Runtime: ${runtimeSec}s  ")
[void]$md.AppendLine("Totals: critical=$critical major=$major minor=$minor (total=$($sorted.Count))")
[void]$md.AppendLine('')
[void]$md.AppendLine('## Summary by page × severity')
[void]$md.AppendLine('')
[void]$md.AppendLine('| Page | Critical | Major | Minor |')
[void]$md.AppendLine('|------|---------:|------:|------:|')
foreach ($g in $pages) {
    $c = ($g.Group | Where-Object severity -EQ 'critical').Count
    $m = ($g.Group | Where-Object severity -EQ 'major').Count
    $n = ($g.Group | Where-Object severity -EQ 'minor').Count
    [void]$md.AppendLine("| $($g.Name) | $c | $m | $n |")
}
[void]$md.AppendLine('')

foreach ($g in $pages) {
    [void]$md.AppendLine("## Page: $($g.Name)")
    [void]$md.AppendLine('')
    foreach ($sev in $severities) {
        $group = @($g.Group | Where-Object severity -EQ $sev)
        if ($group.Count -eq 0) { continue }
        [void]$md.AppendLine("### $sev ($($group.Count))")
        [void]$md.AppendLine('')
        foreach ($v in $group) {
            [void]$md.AppendLine("- rule: ``$($v.rule)`` viewport: ``$($v.viewport)``")
            [void]$md.AppendLine('  ```json')
            $detail = [ordered]@{
                selector  = $v.selector
                expected  = $v.expected
                actual    = $v.actual
                fileHint  = $v.fileHint
            }
            [void]$md.AppendLine(($detail | ConvertTo-Json -Depth 8))
            [void]$md.AppendLine('  ```')
            if ($v.screenshotPath) {
                [void]$md.AppendLine("  ![$($v.rule)]($($v.screenshotPath))")
            }
            [void]$md.AppendLine('')
        }
    }
}

Set-Content -Path (Join-Path $OutputDir 'audit.md') -Value $md.ToString() -Encoding UTF8

# Artifact size cap.
$sizeBytes = (Get-ChildItem -Path $OutputDir -Recurse -File | Measure-Object -Property Length -Sum).Sum
if ($null -eq $sizeBytes) { $sizeBytes = 0 }
if ($sizeBytes -gt 50MB) {
    Write-Error "[audit] Artefacts exceed 50 MB cap (actual: $([math]::Round($sizeBytes/1MB,1)) MB)."
    exit 1
}

Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue

$summaryColor = if ($critical -eq 0) { 'Green' } else { 'Red' }
Write-Host "SUMMARY: critical=$critical major=$major minor=$minor runtime=${runtimeSec}" -ForegroundColor $summaryColor

if ($critical -eq 0) { exit 0 } else { exit 1 }
