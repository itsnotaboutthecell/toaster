<#
.SYNOPSIS
Launches Toaster in dev mode with startup monitoring.

.DESCRIPTION
Runs .\scripts\setup-env.ps1 in the current shell context, starts `npm run tauri dev`,
monitors startup output for success/failure signatures, prints concise diagnosis hints,
and emits a compact final status summary after the observation window.

.PARAMETER ObservationSeconds
Minimum startup observation window in seconds. Default is 20.

.PARAMETER SetupScriptPath
Optional path override for setup-env.ps1 (useful for diagnostics/testing).

.EXAMPLE
.\scripts\launch-toaster-monitored.ps1

.EXAMPLE
.\scripts\launch-toaster-monitored.ps1 -ObservationSeconds 30
#>
[CmdletBinding()]
param(
    [ValidateRange(5, 300)]
    [int]$ObservationSeconds = 300,
    [ValidateRange(1, 30)]
    [int]$DrainSeconds = 3,
    [string]$SetupScriptPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$setupScript = if ([string]::IsNullOrWhiteSpace($SetupScriptPath)) {
    Join-Path $PSScriptRoot "setup-env.ps1"
}
else {
    $SetupScriptPath
}

function Get-LaunchStatus {
    param(
        [bool]$SawSuccessSignal,
        [bool]$SawErrorSignal
    )

    if (-not $SawSuccessSignal) {
        return "failed_to_launch"
    }

    if ($SawErrorSignal) {
        return "launched_with_errors"
    }

    return "launched_ok"
}

function Stop-ProcessTree {
    param(
        [Parameter(Mandatory = $true)]
        [int]$RootPid,
        [System.Collections.Generic.HashSet[int]]$Visited = $(New-Object 'System.Collections.Generic.HashSet[int]'),
        [Nullable[datetime]]$NotBeforeUtc = $null
    )

    $stoppedCount = 0

    if (-not $Visited.Add($RootPid)) {
        return 0
    }

    $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $RootPid" -ErrorAction SilentlyContinue)

    foreach ($child in $children) {
        if ($null -ne $NotBeforeUtc -and $child.CreationDate) {
            $childCreatedUtc = $null
            try {
                $childCreatedUtc = [System.Management.ManagementDateTimeConverter]::ToDateTime($child.CreationDate).ToUniversalTime()
            }
            catch {
                $childCreatedUtc = $null
            }

            if ($null -ne $childCreatedUtc -and $childCreatedUtc -lt $NotBeforeUtc) {
                continue
            }
        }
        $stoppedCount += Stop-ProcessTree -RootPid ([int]$child.ProcessId) -Visited $Visited -NotBeforeUtc $NotBeforeUtc
    }

    $rootProcess = Get-CimInstance Win32_Process -Filter "ProcessId = $RootPid" -ErrorAction SilentlyContinue
    $canStopRoot = $true
    if ($rootProcess -and $null -ne $NotBeforeUtc -and $rootProcess.CreationDate) {
        $rootCreatedUtc = $null
        try {
            $rootCreatedUtc = [System.Management.ManagementDateTimeConverter]::ToDateTime($rootProcess.CreationDate).ToUniversalTime()
        }
        catch {
            $rootCreatedUtc = $null
        }

        if ($null -ne $rootCreatedUtc -and $rootCreatedUtc -lt $NotBeforeUtc) {
            $canStopRoot = $false
        }
    }

    if ($canStopRoot -and (Get-Process -Id $RootPid -ErrorAction SilentlyContinue)) {
        try {
            Stop-Process -Id $RootPid -Force -ErrorAction SilentlyContinue
            $stoppedCount += 1
        }
        catch {
            Write-Host ("[monitor] Warning: failed to stop pid {0}: {1}" -f $RootPid, $_.Exception.Message) -ForegroundColor Yellow
        }
    }

    return $stoppedCount
}

function Stop-LaunchOrphans {
    param(
        [Parameter(Mandatory = $true)]
        [datetime]$NotBeforeUtc,
        [Parameter(Mandatory = $true)]
        [string]$RepoRoot,
        [Parameter(Mandatory = $true)]
        [int]$RootPid
    )

    $stoppedCount = 0
    $candidateNames = @("node.exe", "npm.exe", "npm.cmd", "cargo.exe", "toaster.exe")
    $candidates = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object {
            $_.ProcessId -ne $RootPid -and
            $_.Name -in $candidateNames -and
            $_.CommandLine -and
            $_.CommandLine -match [regex]::Escape($RepoRoot)
        })

    foreach ($candidate in $candidates) {
        $createdUtc = $null
        if ($candidate.CreationDate) {
            try {
                $createdUtc = [System.Management.ManagementDateTimeConverter]::ToDateTime($candidate.CreationDate).ToUniversalTime()
            }
            catch {
                $createdUtc = $null
            }
        }

        if ($null -ne $createdUtc -and $createdUtc -lt $NotBeforeUtc) {
            continue
        }

        if (Get-Process -Id $candidate.ProcessId -ErrorAction SilentlyContinue) {
            try {
                Stop-Process -Id $candidate.ProcessId -Force -ErrorAction SilentlyContinue
                $stoppedCount += 1
            }
            catch {
                Write-Host ("[monitor] Warning: failed to stop orphan pid {0}: {1}" -f $candidate.ProcessId, $_.Exception.Message) -ForegroundColor Yellow
            }
        }
    }

    return $stoppedCount
}

try {
    if (-not (Test-Path $setupScript -PathType Leaf)) {
        Write-Host "Setup script not found: $setupScript" -ForegroundColor Red
        Write-Host "monitor_summary=reason:setup_script_missing;success_signals:none;error_signals:none;hints:none;exit_code:1;terminated_after_observation:false;observed_seconds:0"
        Write-Host "launch_status=failed_to_launch"
        exit 1
    }

    $npmCommand = Get-Command npm.cmd -ErrorAction SilentlyContinue
    if (-not $npmCommand) {
        $npmCommand = Get-Command npm -ErrorAction SilentlyContinue
    }

    if (-not $npmCommand) {
        Write-Host "npm was not found in PATH. Install Node.js and retry." -ForegroundColor Red
        Write-Host "monitor_summary=reason:npm_not_found;success_signals:none;error_signals:none;hints:none;exit_code:1;terminated_after_observation:false;observed_seconds:0"
        Write-Host "launch_status=failed_to_launch"
        exit 1
    }

    Write-Host "[monitor] Loading environment: $setupScript" -ForegroundColor Cyan
    try {
        . $setupScript
    }
    catch {
        Write-Host ("[monitor] setup-env failed: " + $_.Exception.Message) -ForegroundColor Red
        Write-Host "monitor_summary=reason:setup_script_failed;success_signals:none;error_signals:none;hints:none;exit_code:1;terminated_after_observation:false;observed_seconds:0"
        Write-Host "launch_status=failed_to_launch"
        exit 1
    }

    if ($global:ToasterEnvPreflightOk -eq $false) {
        Write-Host "[monitor] setup-env preflight detected Ninja-hostile env vars; aborting before cargo build." -ForegroundColor Red
        Write-Host "monitor_summary=reason:env_preflight_failed;success_signals:none;error_signals:none;hints:none;exit_code:1;terminated_after_observation:false;observed_seconds:0"
        Write-Host "launch_status=failed_to_launch"
        exit 1
    }

    $smokeScript = Join-Path $PSScriptRoot "check-cmake-ninja-env.ps1"
    if (Test-Path $smokeScript) {
        Write-Host "[monitor] Running CMake/Ninja env smoke (auto-wipes stale whisper-rs-sys caches)" -ForegroundColor Cyan
        & $smokeScript -WipeStaleCaches
        if ($LASTEXITCODE -ne 0) {
            Write-Host "[monitor] CMake/Ninja smoke failed; refusing to start cargo tauri dev." -ForegroundColor Red
            Write-Host "monitor_summary=reason:cmake_ninja_smoke_failed;success_signals:none;error_signals:none;hints:none;exit_code:1;terminated_after_observation:false;observed_seconds:0"
            Write-Host "launch_status=failed_to_launch"
            exit 1
        }
    }

    $successPatterns = @(
        @{ Key = "vite-local-url"; Pattern = "(?i)\blocal:\s*https?://" },
        @{ Key = "vite-ready-ms"; Pattern = "(?i)\bready in\s+\d+(\.\d+)?\s*ms\b" },
        @{ Key = "tauri-ready"; Pattern = "(?i)\btauri app.*(running|started|ready)\b|\brunning devcommand\b" }
    )

    $errorSignatures = @(
        @{ Key = "http404"; Pattern = "(?i)\bhttp(?:\s+status)?\s*404\b|\bstatus\s*[:=]?\s*404\b|\b404\s+(?:not found|error)\b|\bnot found\b.*\b(asset|resource|module|url|page)\b"; Hint = "404 detected. Check route/asset paths and Vite base URL." },
        @{ Key = "asset-load"; Pattern = "(?i)failed to load (resource|module|url)|asset.*(not found|404)|could not load asset"; Hint = "Asset load failure detected. Verify generated paths and static file availability." },
        @{ Key = "dev-server"; Pattern = "(?i)dev server.*(unreachable|not running)|could not connect to (vite|dev server)|econnrefused|err_connection_refused|connection refused"; Hint = "Dev server unreachable. Confirm Vite is running and ports are not blocked/in use." },
        @{ Key = "port-in-use"; Pattern = "(?i)eaddrinuse|address already in use|port\s+\d+\s+is already in use"; Hint = "A required port is in use. Stop the conflicting process or change the dev server port." },
        @{ Key = "npm-failure"; Pattern = "(?i)^npm ERR!|missing script:|is not recognized as an internal or external command|command not found"; Hint = "npm command/script failure detected. Verify package scripts and Node/npm installation." },
        @{ Key = "panic"; Pattern = "(?i)\bpanic\b|thread '.*' panicked"; Hint = "Rust panic detected. Check the first panic stack/message above." },
        @{ Key = "unhandled"; Pattern = "(?i)unhandled (error|exception)|fatal error|traceback \(most recent call last\)"; Hint = "Unhandled runtime error detected. Check the first error line above." }
    )

    $seenHints = [System.Collections.Generic.HashSet[string]]::new()
    $successSignalMatches = [System.Collections.Generic.HashSet[string]]::new()
    $errorSignalMatches = [System.Collections.Generic.HashSet[string]]::new()
    $script:sawSuccessSignal = $false
    $script:sawErrorSignal = $false
    $terminatedAfterObservation = $false

    function Handle-ObservedLine {
        param(
            [AllowEmptyString()]
            [string]$Line
        )

        $cleanLine = $Line -replace "`e\[[0-?]*[ -/]*[@-~]", ""
        $cleanLine = $cleanLine -replace "\[[0-9;]*m", ""
        $cleanLine = $cleanLine -replace "\]8;;[^\s]*", ""
        $cleanLine = -join ($cleanLine.ToCharArray() | Where-Object { -not [char]::IsControl($_) })
        $cleanLine = ($cleanLine -replace "\s+", " ").Trim()
        if ([string]::IsNullOrWhiteSpace($cleanLine)) {
            return
        }

        Write-Host $cleanLine

        foreach ($success in $successPatterns) {
            if ($cleanLine -match $success.Pattern) {
                $script:sawSuccessSignal = $true
                $null = $successSignalMatches.Add($success.Key)
            }
        }

        foreach ($sig in $errorSignatures) {
            if ($cleanLine -match $sig.Pattern) {
                $script:sawErrorSignal = $true
                $null = $errorSignalMatches.Add($sig.Key)
                if ($seenHints.Add($sig.Key)) {
                    Write-Host ("[diagnosis] " + $sig.Hint) -ForegroundColor Yellow
                }
            }
        }
    }

    function Process-NewLogLines {
        param(
            [Parameter(Mandatory = $true)]
            [string]$Path,
            [Parameter(Mandatory = $true)]
            [ref]$LastLineIndex
        )

        if (-not (Test-Path $Path)) {
            return
        }

        $content = @(Get-Content -Path $Path -ErrorAction SilentlyContinue)
        if ($content.Count -le $LastLineIndex.Value) {
            return
        }

        for ($i = $LastLineIndex.Value; $i -lt $content.Count; $i++) {
            Handle-ObservedLine -Line ([string]$content[$i])
        }

        $LastLineIndex.Value = $content.Count
    }

    $monitorDir = Join-Path $repoRoot ".launch-monitor"
    if (-not (Test-Path $monitorDir)) {
        $null = New-Item -Path $monitorDir -ItemType Directory -Force
    }

    $oldLogs = Get-ChildItem -Path $monitorDir -File -Filter "launch-*.log" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
    if ($oldLogs.Count -gt 40) {
        $oldLogs | Select-Object -Skip 40 | Remove-Item -Force -ErrorAction SilentlyContinue
    }

    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $stdoutLog = Join-Path $monitorDir ("launch-" + $timestamp + ".stdout.log")
    $stderrLog = Join-Path $monitorDir ("launch-" + $timestamp + ".stderr.log")
    $stdoutLineIndex = 0
    $stderrLineIndex = 0

    Write-Host "[monitor] Starting: npm run tauri dev" -ForegroundColor Cyan
    $process = Start-Process -FilePath $npmCommand.Source -ArgumentList @("run", "tauri", "dev") -WorkingDirectory $repoRoot -NoNewWindow -PassThru -RedirectStandardOutput $stdoutLog -RedirectStandardError $stderrLog
    $startTime = Get-Date
    $observationDeadline = $startTime.AddSeconds($ObservationSeconds)
    $reportedEarlyExitWithoutSuccess = $false

    while ((Get-Date) -lt $observationDeadline) {
        Process-NewLogLines -Path $stdoutLog -LastLineIndex ([ref]$stdoutLineIndex)
        Process-NewLogLines -Path $stderrLog -LastLineIndex ([ref]$stderrLineIndex)
        $process.Refresh()
        if ($process.HasExited -and -not $script:sawSuccessSignal -and -not $reportedEarlyExitWithoutSuccess) {
            Write-Host ("[monitor] Monitored process exited before success signal (exit code {0}); continuing observation window for child process output." -f $process.ExitCode) -ForegroundColor DarkYellow
            $reportedEarlyExitWithoutSuccess = $true
        }
        Start-Sleep -Milliseconds 200
    }

    Write-Host "[monitor] Observation window complete (${ObservationSeconds}s)." -ForegroundColor DarkCyan

    Write-Host "[monitor] Stopping monitored process tree after observation window." -ForegroundColor DarkCyan
    $notBeforeUtc = $startTime.ToUniversalTime().AddSeconds(-2)
    $stoppedProcessCount = Stop-ProcessTree -RootPid $process.Id -NotBeforeUtc $notBeforeUtc
    $stoppedProcessCount += Stop-LaunchOrphans -NotBeforeUtc $notBeforeUtc -RepoRoot $repoRoot -RootPid $process.Id
    $terminatedAfterObservation = $stoppedProcessCount -gt 0

    $drainDeadline = (Get-Date).AddSeconds($DrainSeconds)
    while ((Get-Date) -lt $drainDeadline) {
        Process-NewLogLines -Path $stdoutLog -LastLineIndex ([ref]$stdoutLineIndex)
        Process-NewLogLines -Path $stderrLog -LastLineIndex ([ref]$stderrLineIndex)
        Start-Sleep -Milliseconds 200
    }

    Process-NewLogLines -Path $stdoutLog -LastLineIndex ([ref]$stdoutLineIndex)
    Process-NewLogLines -Path $stderrLog -LastLineIndex ([ref]$stderrLineIndex)

    $process.Refresh()
    $exitCode = $null
    if ($process.HasExited) {
        $exitCode = [int]$process.ExitCode
    }
    $exitCodeSummary = if ($null -eq $exitCode) {
        "running_or_unknown"
    }
    elseif ($terminatedAfterObservation -and $exitCode -eq -1) {
        "terminated_by_monitor"
    }
    else {
        [string]$exitCode
    }

    $status = Get-LaunchStatus -SawSuccessSignal:$script:sawSuccessSignal -SawErrorSignal:$script:sawErrorSignal

    $statusReason = if (-not $script:sawSuccessSignal) {
        "no_success_signal_within_observation"
    }
    elseif ($script:sawErrorSignal) {
        "error_signatures_detected"
    }
    else {
        "success_signals_detected"
    }

    if (-not $script:sawSuccessSignal) {
        Write-Host ("[diagnosis] No startup success signal detected within {0}s. Consider increasing -ObservationSeconds if first build is slow." -f $ObservationSeconds) -ForegroundColor Yellow
    }

    $successSummary = if ($successSignalMatches.Count -gt 0) {
        (@($successSignalMatches) | Sort-Object) -join ","
    }
    else {
        "none"
    }

    $errorSummary = if ($errorSignalMatches.Count -gt 0) {
        (@($errorSignalMatches) | Sort-Object) -join ","
    }
    else {
        "none"
    }

    $hintSummary = if ($seenHints.Count -gt 0) {
        (@($seenHints) | Sort-Object) -join ","
    }
    else {
        "none"
    }

    Write-Host ("monitor_summary=reason:{0};success_signals:{1};error_signals:{2};hints:{3};exit_code:{4};terminated_after_observation:{5};observed_seconds:{6}" -f $statusReason, $successSummary, $errorSummary, $hintSummary, $exitCodeSummary, $terminatedAfterObservation.ToString().ToLowerInvariant(), $ObservationSeconds)
    Write-Host ("launch_logs_stdout=" + $stdoutLog)
    Write-Host ("launch_logs_stderr=" + $stderrLog)
    Write-Host ("launch_status=" + $status)
    if ($status -ne "launched_ok") {
        Write-Host ("[diagnosis] Inspect launch logs: stdout={0} stderr={1}" -f $stdoutLog, $stderrLog) -ForegroundColor Yellow
    }

    switch ($status) {
        "launched_ok" { exit 0 }
        "launched_with_errors" { exit 2 }
        default { exit 1 }
    }
}
catch {
    Write-Host ("[monitor] Unexpected failure: " + $_.Exception.Message) -ForegroundColor Red
    Write-Host "monitor_summary=reason:unexpected_exception;success_signals:none;error_signals:none;hints:none;exit_code:1;terminated_after_observation:false;observed_seconds:0"
    Write-Host "launch_status=failed_to_launch"
    exit 1
}
