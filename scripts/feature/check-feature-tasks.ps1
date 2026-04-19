<#
.SYNOPSIS
    Lightweight schema validator for features/<slug>/tasks.sql.

.DESCRIPTION
    The session SQL store schema is:

        todos     (id, title, description, status)
        todo_deps (todo_id, depends_on)

    The PM agent has repeatedly invented columns (estimate_minutes,
    predecessor_id, successor_id) and invalid status values
    (not-started, todo) that only explode at INSERT time.  This script
    greps the SQL for the known bad patterns and rejects them before
    promote-feature.ps1 advances STATE.

.PARAMETER Feature
    Feature slug.  Resolves to features/<slug>/tasks.sql.

.EXAMPLE
    pwsh scripts/feature/check-feature-tasks.ps1 -Feature my-feature
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Feature
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$tasksFile = Join-Path $repoRoot 'features' $Feature 'tasks.sql'

if (-not (Test-Path $tasksFile)) {
    Write-Error "tasks.sql not found: $tasksFile"
    exit 2
}

$content = Get-Content -Raw $tasksFile
$errors = [System.Collections.Generic.List[string]]::new()

$allowedStatuses = @('pending', 'in_progress', 'done', 'blocked')

# 1. todos column list.
$todosInserts = [regex]::Matches($content, '(?ims)INSERT\s+INTO\s+todos\s*\(([^)]*)\)')
foreach ($m in $todosInserts) {
    $cols = ($m.Groups[1].Value -split ',' | ForEach-Object { $_.Trim().ToLower() })
    $expected = @('id', 'title', 'description', 'status')
    if (($cols -join ',') -ne ($expected -join ',')) {
        $errors.Add("INSERT INTO todos columns = ($($cols -join ', ')); expected exactly ($($expected -join ', '))") | Out-Null
    }
}

# 2. todo_deps column list.
$depsInserts = [regex]::Matches($content, '(?ims)INSERT\s+INTO\s+todo_deps\s*\(([^)]*)\)')
foreach ($m in $depsInserts) {
    $cols = ($m.Groups[1].Value -split ',' | ForEach-Object { $_.Trim().ToLower() })
    $expected = @('todo_id', 'depends_on')
    if (($cols -join ',') -ne ($expected -join ',')) {
        $errors.Add("INSERT INTO todo_deps columns = ($($cols -join ', ')); expected exactly ($($expected -join ', '))") | Out-Null
    }
}

# 3. Status literal check. Pull every quoted string in the `values` tail
# of a todos insert and flag unknown status tokens.  Heuristic: any quoted
# string whose value is one of the KNOWN BAD tokens is rejected.
$badStatuses = @('not-started', 'not_started', 'todo', 'open', 'new', 'started')
foreach ($bad in $badStatuses) {
    if ($content -match "'$([regex]::Escape($bad))'") {
        $errors.Add("tasks.sql contains invalid status literal '$bad'; allowed: $($allowedStatuses -join ', ')") | Out-Null
    }
}

# 4. Forbidden column names inside column lists only (prose descriptions
# are allowed to use these words).
$forbiddenColumns = @('estimate_minutes', 'predecessor_id', 'successor_id', 'owner', 'priority')
$allInsertColumnLists = @()
foreach ($m in $todosInserts) { $allInsertColumnLists += $m.Groups[1].Value }
foreach ($m in $depsInserts) { $allInsertColumnLists += $m.Groups[1].Value }
$joinedColumnText = ($allInsertColumnLists -join ' ').ToLower()
foreach ($col in $forbiddenColumns) {
    if ($joinedColumnText -match "\b$([regex]::Escape($col))\b") {
        $errors.Add("tasks.sql column list references forbidden column '$col'; schema is todos(id,title,description,status) + todo_deps(todo_id,depends_on)") | Out-Null
    }
}

# 5. At least one INSERT INTO todos.
if ($todosInserts.Count -lt 1) {
    $errors.Add("tasks.sql has no 'INSERT INTO todos' statement") | Out-Null
}

if ($errors.Count -gt 0) {
    Write-Host "[FAIL] ${Feature}: tasks.sql schema errors" -ForegroundColor Red
    foreach ($e in $errors) { Write-Host "  - $e" -ForegroundColor Red }
    exit 1
}

Write-Host "[OK] ${Feature}: tasks.sql schema valid" -ForegroundColor Green
exit 0
