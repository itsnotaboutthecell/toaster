<#
.SYNOPSIS
    Chapter-markers time-stretch interaction eval (STUB — planning artifact).

.DESCRIPTION
    Planning-phase stub. Real implementation lands with task
    `chapter-markers-eval` per features/chapter-markers/tasks.sql,
    after features/time-stretch-segments/ defines the stretch model.

    Will render an edit whose keep-segments include a 2x time-stretch
    segment spanning a paragraph boundary, then assert (via
    `ffprobe -show_chapters`) that each chapter's reported start
    matches the analytically computed edit-time start within 1 ms.
    Covers AC-003-a.
#>

[CmdletBinding()]
param()

Write-Host "[stub] eval-chapter-markers-stretch.ps1" -ForegroundColor Yellow
Write-Host "       Not yet implemented; see features/chapter-markers/tasks.sql task chapter-markers-eval." -ForegroundColor Yellow
exit 2
