<#
.SYNOPSIS
Dumps current Toaster state for debugging.

.DESCRIPTION
Reads the settings store and prints key configuration values
that AI agents need when diagnosing issues.
#>

$settingsPath = Join-Path $env:APPDATA "com.toaster.app" "settings_store.json"

Write-Host "=== Toaster Debug State Dump ==="
Write-Host ""

# Settings store
if (Test-Path $settingsPath) {
    Write-Host "[Settings Store] $settingsPath"
    $settings = Get-Content $settingsPath -Raw | ConvertFrom-Json

    if ($settings.settings) {
        $s = $settings.settings
        Write-Host "  settings_version: $($s.settings_version)"
        Write-Host "  caption_font_size: $($s.caption_font_size)"
        Write-Host "  caption_position: $($s.caption_position)"
        Write-Host "  caption_text_color: $($s.caption_text_color)"
        Write-Host "  caption_bg_color: $($s.caption_bg_color)"
        Write-Host "  selected_model: $($s.selected_model)"
        Write-Host "  app_language: $($s.app_language)"
        Write-Host "  normalize_audio: $($s.normalize_audio_on_export)"
        Write-Host "  export_volume_db: $($s.export_volume_db)"
    } else {
        Write-Host "  [WARN] No 'settings' key found in store"
    }
} else {
    Write-Host "[Settings Store] NOT FOUND at $settingsPath"
}

Write-Host ""

# FFmpeg
$ffmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue
if ($ffmpeg) {
    $ver = & ffmpeg -version 2>&1 | Select-Object -First 1
    Write-Host "[FFmpeg] $ver"
} else {
    Write-Host "[FFmpeg] NOT FOUND - caption burn-in will fail"
}

# FFprobe
$ffprobe = Get-Command ffprobe -ErrorAction SilentlyContinue
if ($ffprobe) {
    Write-Host "[FFprobe] Available"
} else {
    Write-Host "[FFprobe] NOT FOUND - video height probing will fall back to 720p"
}

Write-Host ""

# Rust toolchain
Write-Host "[Rust Toolchain]"
$rustc = rustc --version 2>&1
Write-Host "  $rustc"
$target = rustup show active-toolchain 2>&1
Write-Host "  Target: $target"

Write-Host ""

# Project state
Write-Host "[Project]"
$cargoToml = Get-Content "src-tauri\Cargo.toml" -Raw
if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
    Write-Host "  Version: $($Matches[1])"
}

# Check for running instances
$running = Get-Process -Name "toaster*" -ErrorAction SilentlyContinue
if ($running) {
    Write-Host "  [WARN] Toaster is running (PIDs: $($running.Id -join ', '))"
    Write-Host "  Stop before rebuilding to avoid link errors"
} else {
    Write-Host "  No running instances"
}

Write-Host ""
Write-Host "=== End Debug Dump ==="
