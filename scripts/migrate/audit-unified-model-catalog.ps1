<#
.SYNOPSIS
    Audit stub for the unified-model-catalog feature.

.DESCRIPTION
    Verifier for AC-002-b (managers/llm/catalog.rs + managers/llm/download.rs
    deleted), AC-004-a (no `fn download` under managers/llm/), AC-005-a
    (single ModelDownloadProgress struct definition), and AC-008-a
    (local-models/LlmModelCatalog.tsx deleted or <= 40 lines).

    Planned until the unified-model-catalog feature is executed.
    Exits 2 so `scripts/feature/check-feature-coverage.ps1` can validate the
    script path exists without pretending the audit is green.

.PARAMETER Check
    Which acceptance criterion slice to run. One of:
        deleted     - AC-002-b
        no-llm-dl   - AC-004-a
        single-ev   - AC-005-a
        shim-ui     - AC-008-a
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('deleted', 'no-llm-dl', 'single-ev', 'shim-ui')]
    [string]$Check
)

$repoRoot = Split-Path -Parent $PSScriptRoot
$srcRoot = Join-Path $repoRoot 'src-tauri\src'

switch ($Check) {
    'deleted' {
        # AC-002-b: managers/llm/catalog.rs + managers/llm/download.rs deleted.
        $catalog = Join-Path $srcRoot 'managers\llm\catalog.rs'
        $download = Join-Path $srcRoot 'managers\llm\download.rs'
        $missing = @()
        if (Test-Path $catalog) { $missing += $catalog }
        if (Test-Path $download) { $missing += $download }
        if ($missing.Count -gt 0) {
            Write-Host "[FAIL] expected these files to be deleted:" -ForegroundColor Red
            $missing | ForEach-Object { Write-Host "   still present: $_" }
            exit 1
        }

        # Also verify no remaining references to the deleted modules
        # (other than in this audit script itself).
        $files = Get-ChildItem -Path $srcRoot -Recurse -Filter *.rs -File
        $refs = $files |
            Select-String -Pattern 'managers::llm::(catalog|download)\b|LlmCatalogEntry\b|crate::managers::llm::catalog\b|crate::managers::llm::download\b' |
            Where-Object { $_.Line -notmatch '^\s*(//|\*)' }
        if (@($refs).Count -gt 0) {
            Write-Host "[FAIL] leftover references to deleted llm::catalog/llm::download:" -ForegroundColor Red
            $refs | ForEach-Object { Write-Host "   $_" }
            exit 1
        }

        # Also verify managers/llm/mod.rs no longer declares the removed
        # submodules.
        $modRs = Join-Path $srcRoot 'managers\llm\mod.rs'
        if (Test-Path $modRs) {
            $modContent = Get-Content -Raw $modRs
            if ($modContent -match '(?m)^\s*pub\s+mod\s+(catalog|download)\s*;') {
                Write-Host "[FAIL] managers/llm/mod.rs still declares a removed submodule (pub mod catalog/download)" -ForegroundColor Red
                exit 1
            }
        }

        Write-Host "[PASS] managers/llm/catalog.rs and managers/llm/download.rs deleted with no leftover references" -ForegroundColor Green
        exit 0
    }
    'single-ev' {
        # AC-005-a: exactly one `struct ModelDownloadProgress` under src-tauri/src/
        # AND it must contain a `category` field.
        $files = Get-ChildItem -Path $srcRoot -Recurse -Filter *.rs -File
        $matches = $files | Select-String -Pattern 'struct\s+ModelDownloadProgress'
        $count = @($matches).Count
        if ($count -ne 1) {
            Write-Host "[FAIL] expected exactly 1 struct ModelDownloadProgress, found $count" -ForegroundColor Red
            $matches | ForEach-Object { Write-Host "   $_" }
            exit 1
        }
        $file = $matches[0].Path
        $content = Get-Content -Raw $file
        if ($content -notmatch '(?s)struct\s+ModelDownloadProgress\s*\{[^}]*\bcategory\s*:') {
            Write-Host "[FAIL] ModelDownloadProgress in $file is missing a 'category' field" -ForegroundColor Red
            exit 1
        }
        # Also ensure LlmDownloadProgress is gone (indicates collapse, not
        # coexistence).
        $llmStructs = $files | Select-String -Pattern 'struct\s+LlmDownloadProgress'
        if (@($llmStructs).Count -gt 0) {
            Write-Host "[FAIL] legacy LlmDownloadProgress struct still present:" -ForegroundColor Red
            $llmStructs | ForEach-Object { Write-Host "   $_" }
            exit 1
        }
        Write-Host "[PASS] single ModelDownloadProgress struct with category field ($file)" -ForegroundColor Green
        exit 0
    }
    default {
        Write-Host "[STUB] unified-model-catalog audit ($Check): not yet implemented." -ForegroundColor Yellow
        exit 2
    }
}
