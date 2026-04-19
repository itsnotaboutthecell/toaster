<#
.SYNOPSIS
  Exit 0 iff `npm run lint` succeeds. Coverage-gate wrapper.
#>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try { npm run lint; exit $LASTEXITCODE } finally { Pop-Location }
