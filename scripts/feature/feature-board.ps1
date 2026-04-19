<#
.SYNOPSIS
    Terminal Kanban view of all features under features/<slug>/STATE.md.

.DESCRIPTION
    Reads features/*/STATE.md and prints a 6-lane board:

        defined | planned | executing | reviewing | shipped | archived

    Mirrors the afkode Kanban concept (defined -> planned -> executing ->
    reviewing -> shipped) without requiring a GUI.

    State transitions are owned by the relevant skill/agent:
      - product-manager: defined -> planned
      - executing-plans: planned -> executing -> reviewing
      - finishing-a-development-branch: reviewing -> shipped
      - manual / archive policy: any -> archived

.EXAMPLE
    pwsh scripts/feature/feature-board.ps1
#>

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$featuresDir = Join-Path $repoRoot 'features'

$lanes = [ordered]@{
    'defined'   = [System.Collections.Generic.List[string]]::new()
    'planned'   = [System.Collections.Generic.List[string]]::new()
    'executing' = [System.Collections.Generic.List[string]]::new()
    'reviewing' = [System.Collections.Generic.List[string]]::new()
    'shipped'   = [System.Collections.Generic.List[string]]::new()
    'archived'  = [System.Collections.Generic.List[string]]::new()
}

$laneColors = @{
    'defined'   = 'Gray'
    'planned'   = 'Cyan'
    'executing' = 'Yellow'
    'reviewing' = 'Magenta'
    'shipped'   = 'Green'
    'archived'  = 'DarkGray'
}

if (-not (Test-Path $featuresDir)) {
    Write-Host "No features/ directory yet." -ForegroundColor Yellow
    exit 0
}

$slugs = Get-ChildItem $featuresDir -Directory | Sort-Object Name
if ($slugs.Count -eq 0) {
    Write-Host "features/ is empty." -ForegroundColor Yellow
    exit 0
}

foreach ($d in $slugs) {
    $statePath = Join-Path $d.FullName 'STATE.md'
    $state = if (Test-Path $statePath) { (Get-Content $statePath -Raw).Trim().ToLower() } else { 'defined' }
    if (-not $lanes.Contains($state)) {
        Write-Warning "$($d.Name): unknown state '$state', placing under 'defined'"
        $state = 'defined'
    }
    $lanes[$state].Add($d.Name) | Out-Null
}

Write-Host ""
Write-Host "Toaster feature board" -ForegroundColor White
Write-Host ("=" * 60) -ForegroundColor DarkGray

foreach ($lane in $lanes.Keys) {
    $items = $lanes[$lane]
    $color = $laneColors[$lane]
    $header = "[{0,-9}] ({1})" -f $lane, $items.Count
    Write-Host $header -ForegroundColor $color
    if ($items.Count -eq 0) {
        Write-Host "    (empty)" -ForegroundColor DarkGray
    }
    else {
        foreach ($s in $items) {
            $prdPath = Join-Path $featuresDir "$s\PRD.md"
            $tag = if (Test-Path $prdPath) {
                $first = (Get-Content $prdPath -TotalCount 1) -replace '^#\s*PRD:\s*', ''
                "  $s  -- $first"
            }
            else {
                "  $s"
            }
            Write-Host $tag
        }
    }
}
Write-Host ""
