<#
.SYNOPSIS
    Scaffold a new feature planning bundle under features/<slug>/.

.DESCRIPTION
    Creates the directory structure and starter files for a PM-planned
    feature.  Copies templates from features/.templates/ and initialises
    STATE.md to "defined".

    Optionally creates a git worktree with branch feat/<slug> so work
    is isolated from the main tree.

    Designed to automate Phase 1 of .github/agents/product-manager.md so
    the agent (or a human) does not have to create files by hand.

.PARAMETER Slug
    Feature slug (kebab-case, max 40 chars).  Becomes features/<slug>/.

.PARAMETER Worktree
    Create a git worktree with branch feat/<slug>.  If the branch already
    exists, appends a numeric suffix (feat/<slug>-2, -3, ...).

.PARAMETER Force
    Overwrite an existing feature directory.  Without this flag, the script
    exits with an error if the directory already exists.

.EXAMPLE
    pwsh scripts/feature/scaffold-feature.ps1 -Slug notification-center
    pwsh scripts/feature/scaffold-feature.ps1 -Slug caption-export-v2 -Worktree
    pwsh scripts/feature/scaffold-feature.ps1 -Slug caption-export-v2 -Force
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[a-z0-9][a-z0-9-]{0,38}[a-z0-9]$')]
    [string]$Slug,

    [switch]$Worktree,

    [switch]$Force
)

$ErrorActionPreference = 'Stop'

$repoRoot    = Split-Path -Parent $PSScriptRoot
$templateDir = Join-Path $repoRoot 'features' '.templates'
$featureDir  = Join-Path $repoRoot 'features' $Slug
$tasksDir    = Join-Path $featureDir 'tasks'

# ── Guard ────────────────────────────────────────────────────────────────
if ((Test-Path $featureDir) -and -not $Force) {
    Write-Error "Feature directory already exists: $featureDir. Use -Force to overwrite."
    exit 1
}

if (-not (Test-Path $templateDir)) {
    Write-Error "Template directory not found: $templateDir"
    exit 2
}

# ── Create structure ─────────────────────────────────────────────────────
New-Item -ItemType Directory -Path $tasksDir -Force | Out-Null

# Copy and stamp templates
$templateFiles = @('REQUEST.md', 'PRD.md', 'BLUEPRINT.md', 'CATEGORIES.md', 'coverage.json', 'tasks.sql')
foreach ($file in $templateFiles) {
    $src = Join-Path $templateDir $file
    if (-not (Test-Path $src)) {
        Write-Warning "Template not found, skipping: $src"
        continue
    }
    $content = Get-Content -Raw $src
    $content = $content -replace '\{\{SLUG\}\}', $Slug
    $content = $content -replace '\{\{TITLE\}\}', ($Slug -replace '-', ' ')
    Set-Content -Path (Join-Path $featureDir $file) -Value $content -NoNewline
}

# STATE.md — always starts at "defined"
Set-Content -Path (Join-Path $featureDir 'STATE.md') -Value 'defined' -NoNewline

# journal.md — empty starter
Set-Content -Path (Join-Path $featureDir 'journal.md') -Value '' -NoNewline

# ── Worktree ─────────────────────────────────────────────────────────────
if ($Worktree) {
    $branch = "feat/$Slug"
    $suffix = 1

    # Find a branch name that doesn't collide
    $existingBranches = git branch --list --all 2>&1 |
        ForEach-Object { $_.Trim().TrimStart('* ').Replace('remotes/origin/', '') }

    while ($existingBranches -contains $branch) {
        $suffix++
        $branch = "feat/$Slug-$suffix"
    }

    $worktreePath = Join-Path $repoRoot '.worktrees' $Slug
    git worktree add -b $branch $worktreePath 2>&1 | Out-Null

    if ($LASTEXITCODE -ne 0) {
        Write-Warning "Failed to create worktree at $worktreePath (branch $branch)."
    } else {
        Write-Host "  Worktree       -> $worktreePath (branch: $branch)" -ForegroundColor Cyan
    }
}

# ── Summary ──────────────────────────────────────────────────────────────
Write-Host "Feature scaffolded: $featureDir" -ForegroundColor Green
Write-Host ""
Write-Host "  STATE.md        -> defined"
Write-Host "  REQUEST.md      -> fill in the six-element request"
Write-Host "  PRD.md          -> fill in requirements and ACs"
Write-Host "  BLUEPRINT.md    -> fill in architecture decisions"
Write-Host "  coverage.json   -> map every AC to a verifier"
Write-Host "  tasks.sql       -> task graph for execution"
Write-Host "  tasks/          -> per-task context briefings"
Write-Host "  journal.md      -> operational journal"
Write-Host ""
Write-Host "Next: edit REQUEST.md, then run the PM agent phases 2-8."
