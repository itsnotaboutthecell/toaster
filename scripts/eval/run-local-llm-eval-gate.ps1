[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$MediaPath,
    [Parameter(Mandatory = $true)]
    [string]$AsrModelPath,
    [string]$OutputDir
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not (Test-Path $MediaPath -PathType Leaf)) {
    throw "Media file not found: $MediaPath"
}

if (-not (Test-Path $AsrModelPath -PathType Leaf)) {
    throw "ASR model file not found: $AsrModelPath"
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutputDir = Join-Path $repoRoot ".launch-monitor\local-llm-eval-gate-$stamp"
}

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$logsDir = Join-Path $OutputDir "logs"
New-Item -ItemType Directory -Path $logsDir -Force | Out-Null

function Invoke-GateSubcheck {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Id,
        [Parameter(Mandatory = $true)]
        [string]$Criteria,
        [Parameter(Mandatory = $true)]
        [string]$WorkingDirectory,
        [Parameter(Mandatory = $true)]
        [scriptblock]$CommandBlock
    )

    $logPath = Join-Path $logsDir "$Id.log"
    $startedAt = Get-Date
    $outputLines = New-Object System.Collections.Generic.List[string]
    $exitCode = 0
    $threw = $false

    Write-Host "[local-llm-eval-gate] running $Id"
    Push-Location $WorkingDirectory
    try {
        & $CommandBlock 2>&1 |
            Tee-Object -FilePath $logPath |
            ForEach-Object {
                [void]$outputLines.Add($_.ToString())
            }
        $exitCode = $LASTEXITCODE
    }
    catch {
        $threw = $true
        $exitCode = if ($LASTEXITCODE -is [int]) { $LASTEXITCODE } else { 1 }
        $message = $_.Exception.Message
        [void]$outputLines.Add($message)
        Add-Content -Path $logPath -Value $message
    }
    finally {
        Pop-Location
    }

    $durationMs = [int]((Get-Date) - $startedAt).TotalMilliseconds
    $pass = ($exitCode -eq 0) -and (-not $threw)
    $failureReasons = @()
    if (-not $pass) {
        $failureReasons += "command exited with code $exitCode"
        if ($threw) {
            $failureReasons += "command threw an exception"
        }

        $summaryLine = $outputLines |
            Where-Object { $_ -match "test result: FAILED|live validation failed" } |
            Select-Object -Last 1
        if ($null -ne $summaryLine -and -not [string]::IsNullOrWhiteSpace($summaryLine)) {
            $failureReasons += $summaryLine.Trim()
        }

        $detailLines = $outputLines |
            Where-Object { $_ -match "panicked at|^error:|FAILED|Assertion failed|not found" } |
            Select-Object -First 4
        foreach ($line in $detailLines) {
            if (-not [string]::IsNullOrWhiteSpace($line)) {
                $failureReasons += $line.Trim()
            }
        }
    }

    return [ordered]@{
        id              = $Id
        criteria        = $Criteria
        command         = $CommandBlock.ToString().Trim()
        working_dir     = $WorkingDirectory
        pass            = $pass
        exit_code       = $exitCode
        duration_ms     = $durationMs
        log_path        = $logPath
        failure_reasons = @($failureReasons | Select-Object -Unique)
    }
}

function New-GateCheck {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Id,
        [Parameter(Mandatory = $true)]
        [string]$Criteria,
        [Parameter(Mandatory = $true)]
        [array]$Subchecks
    )

    $failedSubchecks = @($Subchecks | Where-Object { -not $_.pass })
    $failureReasons = @()
    foreach ($subcheck in $failedSubchecks) {
        foreach ($reason in $subcheck.failure_reasons) {
            $failureReasons += "$($subcheck.id): $reason"
        }
    }

    return [ordered]@{
        id              = $Id
        criteria        = $Criteria
        pass            = ($failedSubchecks.Count -eq 0)
        subchecks       = $Subchecks
        failure_reasons = @($failureReasons | Select-Object -Unique)
    }
}

Write-Host "[local-llm-eval-gate] media: $MediaPath"
Write-Host "[local-llm-eval-gate] asr_model_path: $AsrModelPath"
Write-Host "[local-llm-eval-gate] output: $OutputDir"

. (Join-Path $PSScriptRoot "setup-env.ps1")

$srcTauriDir = Join-Path $repoRoot "src-tauri"
$liveValidationDir = Join-Path $OutputDir "asr-live-validation"
New-Item -ItemType Directory -Path $liveValidationDir -Force | Out-Null

$cleanupSubcheck = Invoke-GateSubcheck `
    -Id "cleanup-quality-contract-tests" `
    -Criteria "cargo test actions::tests:: exits 0 (cleanup contract and safe rewrite checks)" `
    -WorkingDirectory $srcTauriDir `
    -CommandBlock {
    cargo test actions::tests:: -- --nocapture
}

$precisionLocalLlmSubcheck = Invoke-GateSubcheck `
    -Id "precision-safety-local-llm-tests" `
    -Criteria "cargo test managers::editor::tests::local_llm_apply_ exits 0 (mapping-preserving local LLM apply checks)" `
    -WorkingDirectory $srcTauriDir `
    -CommandBlock {
    cargo test managers::editor::tests::local_llm_apply_ -- --nocapture
}

$precisionBenchmarkSubcheck = Invoke-GateSubcheck `
    -Id "precision-safety-benchmark-tests" `
    -Criteria "cargo test commands::transcribe_file::precision_benchmarks:: exits 0 (timestamp precision benchmark suite)" `
    -WorkingDirectory $srcTauriDir `
    -CommandBlock {
    cargo test commands::transcribe_file::precision_benchmarks:: -- --nocapture
}

$asrSubcheck = Invoke-GateSubcheck `
    -Id "asr-leakage-oracle-live-validation" `
    -Criteria "cargo test commands::waveform::tests::live_validation_backend_media_pipeline (ignored) exits 0 and emits a report with asr_metric_pass=true" `
    -WorkingDirectory $srcTauriDir `
    -CommandBlock {
    $env:TOASTER_LIVE_MEDIA_PATH = $MediaPath
    $env:TOASTER_LIVE_OUTPUT_DIR = $liveValidationDir
    $env:TOASTER_LIVE_ASR_MODEL_PATH = $AsrModelPath
    cargo test commands::waveform::tests::live_validation_backend_media_pipeline -- --ignored --nocapture
}

$liveValidationReportPath = Join-Path $liveValidationDir "live-validation-report.json"
if (-not (Test-Path $liveValidationReportPath -PathType Leaf)) {
    $asrSubcheck.pass = $false
    $asrSubcheck.failure_reasons = @(
        $asrSubcheck.failure_reasons +
            @("expected live validation report missing: $liveValidationReportPath")
    ) | Select-Object -Unique
}
else {
    try {
        $liveValidationReport = Get-Content -Path $liveValidationReportPath -Raw | ConvertFrom-Json
    }
    catch {
        $asrSubcheck.pass = $false
        $asrSubcheck.failure_reasons = @(
            $asrSubcheck.failure_reasons +
                @("failed to parse live validation report JSON: $($_.Exception.Message)")
        ) | Select-Object -Unique
        $liveValidationReport = $null
    }

    if ($null -ne $liveValidationReport) {
        $liveFailureReasons = @()
        if ($liveValidationReport.asr_metric_pass -ne $true) {
            $liveFailureReasons += "asr_metric_pass=false in live-validation-report.json"
        }
        if (-not [string]::IsNullOrWhiteSpace($liveValidationReport.asr_leakage_oracle.error)) {
            $liveFailureReasons += "asr oracle error: $($liveValidationReport.asr_leakage_oracle.error)"
        }

        $previewLeaks = @($liveValidationReport.asr_leakage_oracle.preview_leaked_deleted_phrases)
        if ($previewLeaks.Count -gt 0) {
            $liveFailureReasons += "preview leaked deleted phrases: $($previewLeaks -join ', ')"
        }
        $exportLeaks = @($liveValidationReport.asr_leakage_oracle.export_leaked_deleted_phrases)
        if ($exportLeaks.Count -gt 0) {
            $liveFailureReasons += "export leaked deleted phrases: $($exportLeaks -join ', ')"
        }

        if ($liveValidationReport.overall_pass -ne $true -and @($liveValidationReport.failure_reasons).Count -gt 0) {
            foreach ($reason in $liveValidationReport.failure_reasons) {
                $liveFailureReasons += "live validation: $reason"
            }
        }

        if ($liveFailureReasons.Count -gt 0) {
            $asrSubcheck.pass = $false
            $asrSubcheck.failure_reasons = @($asrSubcheck.failure_reasons + $liveFailureReasons) | Select-Object -Unique
        }
    }
}

$asrSubcheck.live_validation_report_path = $liveValidationReportPath

$cleanupCheck = New-GateCheck `
    -Id "cleanup_quality" `
    -Criteria "cleanup contract quality gates pass when cleanup contract tests succeed" `
    -Subchecks @($cleanupSubcheck)

$precisionCheck = New-GateCheck `
    -Id "precision_safety" `
    -Criteria "precision safety gates pass when local LLM apply safety tests and precision benchmark tests both succeed" `
    -Subchecks @($precisionLocalLlmSubcheck, $precisionBenchmarkSubcheck)

$asrCheck = New-GateCheck `
    -Id "asr_leakage_oracle" `
    -Criteria "ASR oracle gate passes only when live-validation report exists, asr_metric_pass=true, no oracle error, and no leaked deleted phrases" `
    -Subchecks @($asrSubcheck)

$checks = @($cleanupCheck, $precisionCheck, $asrCheck)
$overallPass = (@($checks | Where-Object { -not $_.pass }).Count -eq 0)
$overallFailureReasons = @()
foreach ($check in $checks) {
    if (-not $check.pass) {
        foreach ($reason in $check.failure_reasons) {
            $overallFailureReasons += "$($check.id): $reason"
        }
    }
}

$report = [ordered]@{
    schema_version = "local_llm_eval_gate_v1"
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    command = ".\scripts\run-local-llm-eval-gate.ps1 -MediaPath <path> -AsrModelPath <path> [-OutputDir <path>]"
    required_inputs = [ordered]@{
        media_path = [ordered]@{
            required = $true
            value = $MediaPath
        }
        asr_model_path = [ordered]@{
            required = $true
            value = $AsrModelPath
        }
    }
    output_dir = $OutputDir
    checks = $checks
    overall_pass = $overallPass
    failure_reasons = @($overallFailureReasons | Select-Object -Unique)
}

$gateReportPath = Join-Path $OutputDir "local-llm-eval-gate-report.json"
$report | ConvertTo-Json -Depth 20 | Set-Content -Path $gateReportPath -Encoding UTF8

Write-Host "[local-llm-eval-gate] report: $gateReportPath"
Write-Host "[local-llm-eval-gate] overall_pass=$overallPass"

if (-not $overallPass) {
    Write-Error "Local LLM eval gate failed. See report: $gateReportPath"
    exit 1
}

