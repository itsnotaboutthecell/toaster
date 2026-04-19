<#
.SYNOPSIS
  Exit 0 iff `cargo check -p toaster --lib` succeeds from src-tauri/.
  Coverage-gate wrapper. Dot-source setup-env.ps1 in the same session for Windows.
#>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Push-Location (Join-Path $root 'src-tauri')
try { cargo check -p toaster --lib; exit $LASTEXITCODE } finally { Pop-Location }
