<#
.SYNOPSIS
    Promote a feature from "defined" to "planned" after passing the
    coverage gate.

.DESCRIPTION
    Runs scripts/feature/check-feature-coverage.ps1 for the given feature slug.
    If it exits 0, updates STATE.md to "planned" and appends a
    timestamped "## Plan complete" entry to journal.md.

    This automates Phase 8 steps 5-6 of
    .github/agents/product-manager.md.

.PARAMETER Slug
    Feature slug.  Resolves to features/<slug>/.

.EXAMPLE
    pwsh scripts/feature/promote-feature.ps1 -Slug notification-center
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Slug
)

$ErrorActionPreference = 'Stop'

$repoRoot   = Split-Path -Parent $PSScriptRoot
$featureDir = Join-Path $repoRoot 'features' $Slug
$stateFile  = Join-Path $featureDir 'STATE.md'
$journalFile = Join-Path $featureDir 'journal.md'

# ── Guard ────────────────────────────────────────────────────────────────
if (-not (Test-Path $featureDir)) {
    Write-Error "Feature directory not found: $featureDir"
    exit 2
}

if (-not (Test-Path $stateFile)) {
    Write-Error "STATE.md not found in $featureDir"
    exit 2
}

$currentState = (Get-Content -Raw $stateFile).Trim()
if ($currentState -ne 'defined') {
    Write-Error "STATE.md is '$currentState', expected 'defined'. Cannot promote."
    exit 1
}

# ── Coverage gate ────────────────────────────────────────────────────────
Write-Host "Running coverage gate for '$Slug'..." -ForegroundColor Cyan

$coverageScript = Join-Path $repoRoot 'scripts' 'check-feature-coverage.ps1'
& pwsh $coverageScript -Feature $Slug

if ($LASTEXITCODE -ne 0) {
    Write-Error "Coverage gate failed (exit $LASTEXITCODE). STATE.md stays at 'defined'."
    exit 1
}

# ── tasks.sql schema gate ────────────────────────────────────────────────
$tasksFile = Join-Path $featureDir 'tasks.sql'
if (Test-Path $tasksFile) {
    Write-Host "Running tasks.sql schema gate for '$Slug'..." -ForegroundColor Cyan
    $tasksScript = Join-Path $repoRoot 'scripts' 'check-feature-tasks.ps1'
    & pwsh $tasksScript -Feature $Slug
    if ($LASTEXITCODE -ne 0) {
        Write-Error "tasks.sql schema gate failed (exit $LASTEXITCODE). STATE.md stays at 'defined'."
        exit 1
    }
}

# ── Promote ──────────────────────────────────────────────────────────────
Set-Content -Path $stateFile -Value 'planned' -NoNewline

$timestamp = Get-Date -Format 'yyyy-MM-ddTHH:mm:ssZ'
$journalEntry = @"

## Plan complete

- Timestamp: $timestamp
- Coverage gate: passed
- State: defined -> planned
"@

Add-Content -Path $journalFile -Value $journalEntry

Write-Host ""
Write-Host "Feature '$Slug' promoted to 'planned'." -ForegroundColor Green
Write-Host "  STATE.md  -> planned"
Write-Host "  journal.md updated with timestamp."
