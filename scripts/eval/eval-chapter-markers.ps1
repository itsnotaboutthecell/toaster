<#
.SYNOPSIS
    Chapter-markers export eval (STUB — planning artifact).

.DESCRIPTION
    Planning-phase stub that satisfies the coverage gate's script-path
    existence check. Real implementation lands with task
    `chapter-markers-eval` per features/chapter-markers/tasks.sql.

    Will render a fixture edit through export_edited_media, then run
    `ffprobe -show_chapters` against the resulting file and the
    sibling `<basename>.chapters.vtt` sidecar, asserting:

      -Mode container  ->  AC-001-b: chapter atoms match backend list
                            within 1 ms per boundary.
      -Mode empty      ->  AC-004-a: no chapter atoms, no sidecar.

.PARAMETER Mode
    container | empty
#>

[CmdletBinding()]
param(
    [ValidateSet('container', 'empty')]
    [string]$Mode = 'container'
)

Write-Host "[stub] eval-chapter-markers.ps1 mode=$Mode" -ForegroundColor Yellow
Write-Host "       Not yet implemented; see features/chapter-markers/tasks.sql task chapter-markers-eval." -ForegroundColor Yellow
exit 2
