<# .SYNOPSIS
    Pre-build hook: kill stale processes that hold DLL locks.
    Called by Copilot hooks before build commands (mingw32-make, cmake).
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'SilentlyContinue'

$input_json = [Console]::In.ReadToEnd()

$staleProcesses = @('toaster-app', 'mingw32-make', 'cc1plus', 'cc1')
$killed = @()

foreach ($name in $staleProcesses) {
    $procs = Get-Process -Name $name -ErrorAction SilentlyContinue
    foreach ($proc in $procs) {
        try {
            $proc.Kill()
            $killed += $name
        } catch {
            # Process may have already exited
        }
    }
}

if ($killed.Count -gt 0) {
    $uniqueKilled = $killed | Sort-Object -Unique
    Write-Host "Killed stale processes: $($uniqueKilled -join ', ')"
} else {
    Write-Host "No stale processes found."
}

exit 0
