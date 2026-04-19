<#
.SYNOPSIS
  Exit 0 iff the supplied ripgrep pattern is ABSENT from the given path.
  Used by coverage.json entries that assert "this symbol/key is no longer
  referenced".

.PARAMETER Pattern
  Regex pattern, passed to ripgrep.

.PARAMETER Path
  File or directory to search.

.PARAMETER Glob
  Optional ripgrep glob filter (e.g. *.tsx).
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$Pattern,
  [Parameter(Mandatory)][string]$Path,
  [string]$Glob
)
$ErrorActionPreference = 'Stop'
$args = @('-n', '--no-heading', $Pattern, $Path)
if ($Glob) { $args = @('-n', '--no-heading', '-g', $Glob, $Pattern, $Path) }
& rg @args
$rc = $LASTEXITCODE
if ($rc -eq 1) { exit 0 }
if ($rc -eq 0) {
  Write-Error "Pattern '$Pattern' still present under '$Path'."
  exit 1
}
Write-Error "ripgrep failed with exit $rc."
exit $rc
