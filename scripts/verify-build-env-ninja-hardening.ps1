<#
.SYNOPSIS
    Per-AC automated verifier for the build-env-ninja-hardening feature.

.DESCRIPTION
    Each AC in features/build-env-ninja-hardening/coverage.json points at
    this script with `-Ac <id>`. Spawns a fresh `pwsh -NoProfile` per
    check so each verification runs in an isolated env (no leakage
    between cases). Exits 0 on pass, non-zero on fail.

    Supported -Ac values:
      AC-001-a  Strip removes every Ninja-hostile var; compiler vars
                (INCLUDE/LIB/LIBPATH/PATH) survive.
      AC-001-b  Smoke script reports OK after a clean setup-env source.
      AC-002-a  Pre-source injection of VSINSTALLDIR=C:\stub flips
                $global:ToasterEnvPreflightOk to $false and prints [FAIL].
      AC-002-b  launch-toaster-monitored.ps1 with a polluted parent shell
                exits with launch_status=failed_to_launch and never invokes
                cargo. (Live-app gate; observation 30s.)
      AC-003-a  docs/build.md "Ninja-hostile vcvars vars" subsection lists
                every name in $script:NinjaHostileVars.
      AC-003-b  docs/build.md Troubleshooting table has a row for the
                CMAKE_GENERATOR_INSTANCE / VSINSTALLDIR leak.
      AC-004-a  Smoke script exits 0 in <5 seconds.
      AC-004-b  Smoke script catches each Ninja-hostile var injected one
                at a time (CMAKE_GENERATOR_INSTANCE / Platform /
                CMAKE_GENERATOR_TOOLSET).
      AC-004-c  See coverage.json -- still kind:manual (live-app gate per
                AGENTS.md "Verified means the live app").

.PARAMETER Ac
    AC identifier from PRD.md.

.PARAMETER All
    Run every supported AC in turn; exit non-zero if any fails.
#>
[CmdletBinding(DefaultParameterSetName = 'Single')]
param(
    [Parameter(ParameterSetName = 'Single', Mandatory = $true)]
    [string]$Ac,

    [Parameter(ParameterSetName = 'All', Mandatory = $true)]
    [switch]$All
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$setupEnv = Join-Path $PSScriptRoot 'setup-env.ps1'
$smoke = Join-Path $PSScriptRoot 'check-cmake-ninja-env.ps1'
$buildDoc = Join-Path $repoRoot 'docs\build.md'

# Wipe every name we care about + the sentinel + the ones vcvars sets.
# Used as a prefix in every child pwsh so the case starts truly clean.
$envClear = "Remove-Item Env:VSINSTALLDIR,Env:VCINSTALLDIR,Env:VCToolsInstallDir,Env:VisualStudioVersion,Env:Platform,Env:CMAKE_GENERATOR_INSTANCE,Env:CMAKE_GENERATOR_PLATFORM,Env:CMAKE_GENERATOR_TOOLSET,Env:TOASTER_ENV_INITIALIZED -ErrorAction SilentlyContinue;"

$tracked = @('Platform','CMAKE_GENERATOR_PLATFORM','CMAKE_GENERATOR_TOOLSET','CMAKE_GENERATOR_INSTANCE','VSINSTALLDIR','VCINSTALLDIR','VCToolsInstallDir','VisualStudioVersion')

function Invoke-CleanPwsh {
    param([string]$Snippet)
    & pwsh -NoProfile -Command "$envClear $Snippet"
    return $LASTEXITCODE
}

function Pass($msg) { Write-Host "[PASS] $msg" -ForegroundColor Green; return 0 }
function Fail($msg) { Write-Host "[FAIL] $msg" -ForegroundColor Red; return 1 }

function Test-AC-001-a {
    $expr = ". '$setupEnv' *> `$null; " +
            '$missing = @(); $present = @();' +
            "foreach (`$v in @('$($tracked -join "','")')) {" +
            ' if (Test-Path "Env:$v") { $present += $v } else { $missing += $v } };' +
            "foreach (`$c in 'INCLUDE','LIB','LIBPATH','PATH') {" +
            ' if (-not (Test-Path "Env:$c")) { Write-Host "MISSING_COMPILER_VAR:$c"; exit 2 } };' +
            'if ($present.Count -gt 0) { Write-Host ("STILL_PRESENT:" + ($present -join ",")); exit 1 };' +
            'Write-Host "STRIPPED_ALL"; exit 0'
    $out = & pwsh -NoProfile -Command "$envClear $expr" 2>&1
    if ($LASTEXITCODE -eq 0) { return Pass "AC-001-a: all 8 tracked vars stripped, compiler vars (INCLUDE/LIB/LIBPATH/PATH) survived" }
    return Fail "AC-001-a: $($out -join ' | ')"
}

function Test-AC-001-b {
    $expr = ". '$setupEnv' *> `$null; & '$smoke'; exit `$LASTEXITCODE"
    & pwsh -NoProfile -Command "$envClear $expr" *> $null
    if ($LASTEXITCODE -eq 0) { return Pass "AC-001-b: smoke script reports OK after clean setup-env" }
    return Fail "AC-001-b: smoke exited $LASTEXITCODE after clean setup-env"
}

function Test-AC-002-a {
    $expr = "`$env:VSINSTALLDIR='C:\stub'; . '$setupEnv' 2>&1 | Out-Null;" +
            'if ($global:ToasterEnvPreflightOk) { Write-Host "PREFLIGHT_DID_NOT_FAIL"; exit 1 };' +
            'exit 0'
    & pwsh -NoProfile -Command "$envClear $expr" *> $null
    if ($LASTEXITCODE -eq 0) { return Pass "AC-002-a: pre-source VSINSTALLDIR=C:\stub flips ToasterEnvPreflightOk to False" }
    return Fail "AC-002-a: preflight did not fail on injected VSINSTALLDIR"
}

function Test-AC-002-b {
    $launcher = Join-Path $PSScriptRoot 'launch-toaster-monitored.ps1'
    $tmpLog = Join-Path $env:TEMP "toaster-ac002b-$(Get-Random).log"
    try {
        $expr = "`$env:VSINSTALLDIR='C:\stub'; & '$launcher' -ObservationSeconds 30 *> '$tmpLog'; exit `$LASTEXITCODE"
        & pwsh -NoProfile -Command "$envClear $expr" *> $null
        $launcherExit = $LASTEXITCODE
        $logContent = if (Test-Path $tmpLog) { Get-Content $tmpLog -Raw } else { '' }
        $statusLine = ($logContent -split "`r?`n" | Where-Object { $_ -match '^launch_status=' } | Select-Object -Last 1)
        $cargoInvoked = $logContent -match 'Compiling\s+toaster\b' -or $logContent -match 'Finished\s+`?dev`?'
        if ($statusLine -match 'launch_status=failed_to_launch' -and -not $cargoInvoked) {
            return Pass "AC-002-b: launcher refused to start with polluted env (exit=$launcherExit, status=$statusLine, cargo not invoked)"
        }
        return Fail "AC-002-b: launcher exit=$launcherExit, status=$statusLine, cargoInvoked=$cargoInvoked"
    } finally {
        Remove-Item $tmpLog -ErrorAction SilentlyContinue
    }
}

function Test-AC-003-a {
    if (-not (Test-Path $buildDoc)) { return Fail "AC-003-a: docs/build.md not found" }
    $content = Get-Content $buildDoc -Raw
    if ($content -notmatch '(?m)^#{1,6}\s+Ninja-hostile vcvars vars\s*$') {
        return Fail "AC-003-a: docs/build.md is missing 'Ninja-hostile vcvars vars' heading"
    }
    $missing = @()
    foreach ($v in $tracked) {
        if ($content -notmatch [regex]::Escape($v)) { $missing += $v }
    }
    if ($missing.Count -gt 0) { return Fail "AC-003-a: docs/build.md missing names: $($missing -join ', ')" }
    return Pass "AC-003-a: docs/build.md lists every Ninja-hostile var"
}

function Test-AC-003-b {
    if (-not (Test-Path $buildDoc)) { return Fail "AC-003-b: docs/build.md not found" }
    $content = Get-Content $buildDoc -Raw
    if ($content -notmatch '(?m)^\|.*CMAKE_GENERATOR_INSTANCE.*\|') {
        return Fail "AC-003-b: docs/build.md Troubleshooting table missing CMAKE_GENERATOR_INSTANCE row"
    }
    return Pass "AC-003-b: docs/build.md Troubleshooting table has the CMAKE_GENERATOR_INSTANCE row"
}

function Test-AC-004-a {
    $expr = ". '$setupEnv' *> `$null; `$sw = [Diagnostics.Stopwatch]::StartNew(); & '$smoke' *> `$null; `$sw.Stop(); Write-Host (`"SECONDS:`" + `$sw.Elapsed.TotalSeconds); exit `$LASTEXITCODE"
    $out = & pwsh -NoProfile -Command "$envClear $expr" 2>&1
    $code = $LASTEXITCODE
    $line = ($out | Select-String 'SECONDS:').Line
    $secs = if ($line -match 'SECONDS:([\d\.]+)') { [double]$matches[1] } else { -1 }
    if ($code -ne 0) { return Fail "AC-004-a: smoke exit=$code (expected 0)" }
    if ($secs -lt 0) { return Fail "AC-004-a: could not measure elapsed time" }
    if ($secs -ge 5.0) { return Fail "AC-004-a: smoke took $secs s, budget <5 s" }
    return Pass "AC-004-a: smoke exits 0 in $([math]::Round($secs,2))s (<5 s budget)"
}

function Test-AC-004-b {
    $cases = @('CMAKE_GENERATOR_INSTANCE','Platform','CMAKE_GENERATOR_TOOLSET')
    foreach ($v in $cases) {
        $expr = ". '$setupEnv' *> `$null; `$env:$v='C:\stub'; & '$smoke' *> `$null; exit `$LASTEXITCODE"
        & pwsh -NoProfile -Command "$envClear $expr" *> $null
        if ($LASTEXITCODE -eq 0) { return Fail "AC-004-b: smoke did NOT detect injected $v (exit=0)" }
    }
    return Pass "AC-004-b: smoke detects each of $($cases -join ', ') when injected"
}

$dispatch = @{
    'AC-001-a' = { Test-AC-001-a }
    'AC-001-b' = { Test-AC-001-b }
    'AC-002-a' = { Test-AC-002-a }
    'AC-002-b' = { Test-AC-002-b }
    'AC-003-a' = { Test-AC-003-a }
    'AC-003-b' = { Test-AC-003-b }
    'AC-004-a' = { Test-AC-004-a }
    'AC-004-b' = { Test-AC-004-b }
}

if ($All) {
    $worst = 0
    foreach ($key in $dispatch.Keys | Sort-Object) {
        $rc = & $dispatch[$key]
        if ($rc -gt $worst) { $worst = $rc }
    }
    exit $worst
}

if (-not $dispatch.ContainsKey($Ac)) {
    Write-Host "Unknown -Ac value '$Ac'. Supported: $($dispatch.Keys -join ', ')" -ForegroundColor Red
    Write-Host "Note: AC-004-c is still kind:manual (live-app gate per AGENTS.md)." -ForegroundColor Yellow
    exit 2
}

exit (& $dispatch[$Ac])
