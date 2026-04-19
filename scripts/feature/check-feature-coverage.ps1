<#
.SYNOPSIS
    Coverage gate for a feature planned by .github/agents/product-manager.md.

.DESCRIPTION
    Verifies that every AC-NNN-x in features/<slug>/PRD.md has a matching
    entry in features/<slug>/coverage.json, and that each verifier `kind`
    is one of the allowed values:

        skill        - a name under .github/skills/<name>/SKILL.md
        agent        - a name under .github/agents/<name>.md
        cargo-test   - a `cargo test <name>` command
        script       - a path under scripts/
        manual       - a live-app check (must include 'steps' array)
        doc-section  - a section in a doc file (must include 'sections' array
                       and 'command' that points at a file under features/).
                       Use sparingly: planning-only bundles whose ACs are
                       satisfied by inspecting a documented section rather
                       than by executing code or tests.

    Exits 0 on green, 1 on missing/invalid coverage, 2 on bad input.

    Wired into CI alongside scripts/check-translations.ts.

.PARAMETER Feature
    Feature slug. Resolves to features/<slug>/.

.PARAMETER All
    Validate every feature under features/. Useful in CI.

.EXAMPLE
    pwsh scripts/feature/check-feature-coverage.ps1 -Feature notification-center
    pwsh scripts/feature/check-feature-coverage.ps1 -All
#>

[CmdletBinding(DefaultParameterSetName = 'Single')]
param(
    [Parameter(ParameterSetName = 'Single', Mandatory = $true)]
    [string]$Feature,

    [Parameter(ParameterSetName = 'All', Mandatory = $true)]
    [switch]$All
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$featuresDir = Join-Path $repoRoot 'features'
$skillsDir = Join-Path $repoRoot '.github\skills'
$agentsDir = Join-Path $repoRoot '.github\agents'
$scriptsDir = Join-Path $repoRoot 'scripts'

$allowedKinds = @('skill', 'agent', 'cargo-test', 'script', 'manual', 'doc-section')

function Get-AcsFromPrd {
    param([string]$PrdPath)
    if (-not (Test-Path $PrdPath)) { return @() }
    $matches = Select-String -Path $PrdPath -Pattern '^\s*-?\s*AC-\d{3}-[a-z]\b' -AllMatches
    $ids = New-Object System.Collections.Generic.HashSet[string]
    foreach ($m in $matches) {
        foreach ($mm in $m.Matches) {
            $id = ($mm.Value -replace '^\s*-?\s*', '').Trim()
            [void]$ids.Add($id)
        }
    }
    return $ids
}

function Test-Verifier {
    param(
        [string]$AcId,
        [psobject]$Entry,
        [System.Collections.Generic.List[string]]$Errors
    )

    if (-not $Entry.PSObject.Properties.Name -contains 'kind') {
        $Errors.Add("${AcId}: missing 'kind'") | Out-Null
        return
    }
    $kind = [string]$Entry.kind
    if ($kind -notin $allowedKinds) {
        $Errors.Add("${AcId}: kind '$kind' not in $($allowedKinds -join ', ')") | Out-Null
        return
    }
    $verifier = [string]$Entry.verifier
    if ([string]::IsNullOrWhiteSpace($verifier)) {
        $Errors.Add("${AcId}: missing 'verifier'") | Out-Null
        return
    }

    switch ($kind) {
        'skill' {
            $p = Join-Path $skillsDir "$verifier\SKILL.md"
            if (-not (Test-Path $p)) {
                $Errors.Add("${AcId}: skill '$verifier' not found at $p") | Out-Null
            }
        }
        'agent' {
            $p = Join-Path $agentsDir "$verifier.md"
            if (-not (Test-Path $p)) {
                $Errors.Add("${AcId}: agent '$verifier' not found at $p") | Out-Null
            }
        }
        'cargo-test' {
            if (-not $Entry.command -or $Entry.command -notmatch 'cargo\s+test') {
                $Errors.Add("${AcId}: cargo-test entry must have a 'command' containing 'cargo test'") | Out-Null
            }
        }
        'script' {
            if (-not $Entry.command) {
                $Errors.Add("${AcId}: script entry missing 'command'") | Out-Null
            }
            else {
                $cmd = [string]$Entry.command
                $scriptName = ($cmd -split '\s+' | Where-Object { $_ -like 'scripts/*' -or $_ -like 'scripts\*' } | Select-Object -First 1)
                if ($scriptName) {
                    $resolved = Join-Path $repoRoot ($scriptName -replace '/', '\')
                    if (-not (Test-Path $resolved)) {
                        $Errors.Add("${AcId}: script '$scriptName' not found") | Out-Null
                    }
                }
                else {
                    $Errors.Add("${AcId}: script entry command must reference a path under scripts/") | Out-Null
                }
            }
        }
        'manual' {
            if (-not $Entry.command) {
                $Errors.Add("${AcId}: manual entry missing 'command' (e.g. launch-toaster-monitored.ps1)") | Out-Null
            }
            if (-not $Entry.steps -or $Entry.steps.Count -lt 1) {
                $Errors.Add("${AcId}: manual entry must include a non-empty 'steps' array") | Out-Null
            }
        }
        'doc-section' {
            if (-not $Entry.command) {
                $Errors.Add("${AcId}: doc-section entry missing 'command' (path to a file under features/)") | Out-Null
            }
            else {
                $cmd = [string]$Entry.command
                $resolved = Join-Path $repoRoot ($cmd -replace '/', '\')
                if (-not (Test-Path $resolved)) {
                    $Errors.Add("${AcId}: doc-section file '$cmd' not found") | Out-Null
                }
                elseif ($cmd -notmatch '^features[\\/]') {
                    $Errors.Add("${AcId}: doc-section command must point at a file under features/") | Out-Null
                }
                else {
                    if (-not $Entry.sections -or $Entry.sections.Count -lt 1) {
                        $Errors.Add("${AcId}: doc-section entry must include a non-empty 'sections' array") | Out-Null
                    }
                    else {
                        $docContent = Get-Content $resolved -Raw
                        foreach ($section in $Entry.sections) {
                            $sectionStr = [string]$section
                            $pattern = '(?m)^#{1,6}\s+' + [regex]::Escape($sectionStr) + '\s*$'
                            if ($docContent -notmatch $pattern) {
                                $Errors.Add("${AcId}: doc-section '$sectionStr' not found as a markdown heading in '$cmd'") | Out-Null
                            }
                        }
                    }
                }
            }
        }
    }
}

function Test-FeatureCoverage {
    param([string]$Slug)

    $featureDir = Join-Path $featuresDir $Slug
    if (-not (Test-Path $featureDir)) {
        Write-Error "Feature directory not found: $featureDir"
        return 2
    }

    $prdPath = Join-Path $featureDir 'PRD.md'
    $covPath = Join-Path $featureDir 'coverage.json'
    $statePath = Join-Path $featureDir 'STATE.md'

    if (-not (Test-Path $prdPath)) {
        Write-Host "[FAIL] ${Slug}: PRD.md missing" -ForegroundColor Red
        return 2
    }
    if (-not (Test-Path $covPath)) {
        Write-Host "[FAIL] ${Slug}: coverage.json missing" -ForegroundColor Red
        return 2
    }

    $acs = Get-AcsFromPrd -PrdPath $prdPath
    if ($acs.Count -eq 0) {
        Write-Warning "${Slug}: PRD.md has no AC-NNN-x entries"
        return 1
    }

    try {
        $cov = Get-Content $covPath -Raw | ConvertFrom-Json
    }
    catch {
        Write-Host "[FAIL] ${Slug}: coverage.json is not valid JSON: $_" -ForegroundColor Red
        return 2
    }

    if (-not $cov.acs) {
        Write-Host "[FAIL] ${Slug}: coverage.json missing top-level 'acs' object" -ForegroundColor Red
        return 2
    }

    $errors = [System.Collections.Generic.List[string]]::new()

    foreach ($ac in $acs) {
        $entry = $cov.acs.$ac
        if (-not $entry) {
            $errors.Add("${ac}: present in PRD.md, missing from coverage.json") | Out-Null
            continue
        }
        Test-Verifier -AcId $ac -Entry $entry -Errors $errors
    }

    foreach ($key in $cov.acs.PSObject.Properties.Name) {
        if (-not $acs.Contains($key)) {
            $errors.Add("${key}: in coverage.json but not in PRD.md (stale)") | Out-Null
        }
    }

    if ($errors.Count -eq 0) {
        $state = if (Test-Path $statePath) { (Get-Content $statePath -Raw).Trim() } else { '<missing>' }
        Write-Host "[OK] $Slug : $($acs.Count) ACs covered (state: $state)" -ForegroundColor Green
        return 0
    }
    else {
        Write-Host "[FAIL] $Slug" -ForegroundColor Red
        foreach ($e in $errors) { Write-Host "  - $e" -ForegroundColor Red }
        return 1
    }
}

if ($All) {
    if (-not (Test-Path $featuresDir)) {
        Write-Host "No features/ directory yet; nothing to check." -ForegroundColor Yellow
        exit 0
    }
    $slugs = Get-ChildItem $featuresDir -Directory | Where-Object { $_.Name -ne '.templates' } | Select-Object -ExpandProperty Name
    if ($slugs.Count -eq 0) {
        Write-Host "features/ is empty; nothing to check." -ForegroundColor Yellow
        exit 0
    }
    $worst = 0
    foreach ($s in $slugs) {
        $rc = Test-FeatureCoverage -Slug $s
        if ($rc -gt $worst) { $worst = $rc }
    }
    exit $worst
}
else {
    exit (Test-FeatureCoverage -Slug $Feature)
}
