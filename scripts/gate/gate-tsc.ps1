<#
.SYNOPSIS
  Exit 0 iff `npx tsc --noEmit` succeeds. Coverage-gate wrapper.
#>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try { npx tsc --noEmit; exit $LASTEXITCODE } finally { Pop-Location }
