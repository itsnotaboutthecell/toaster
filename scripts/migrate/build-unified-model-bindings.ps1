<#
.SYNOPSIS
    Build + bindings check stub for the unified-model-catalog feature.

.DESCRIPTION
    Verifier for AC-006-b. Wraps `npm run build` so the coverage gate
    can bind a single script-path token to the acceptance criterion.

    Planned until the unified-model-catalog feature is executed.
    Exits 2 so `scripts/feature/check-feature-coverage.ps1` can validate the
    script path exists without pretending the build passes before the
    bindings regeneration lands.
#>
[CmdletBinding()]
param()

Write-Host "[STUB] unified-model-catalog bindings build: not yet implemented." -ForegroundColor Yellow
Write-Host "       Once the unified Tauri commands land, replace this stub with" -ForegroundColor Yellow
Write-Host "       an invocation of 'npm run build' that asserts no TS errors." -ForegroundColor Yellow
exit 2
