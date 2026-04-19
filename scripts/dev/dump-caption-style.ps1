# Reads caption settings from the settings store and prints what FFmpeg would receive
# Usage: .\scripts\dump-caption-style.ps1

Write-Host "Caption Style Debug Dump"
Write-Host "========================"
Write-Host ""
Write-Host "To see the actual FFmpeg args used during export,"
Write-Host "look for this log line in the Tauri dev console:"
Write-Host '  [INFO] Running FFmpeg export: ffmpeg ...'
Write-Host ""
Write-Host "ASS Style Reference:"
Write-Host "  BorderStyle=3 → Opaque box mode (OutlineColour = box fill)"
Write-Host "  OutlineColour=&HAABBGGRR& → Box background (alpha inverted: 00=opaque)"
Write-Host "  PrimaryColour=&H00BBGGRR& → Text color (00 alpha = opaque)"
Write-Host "  Outline=4 → Box padding in pixels"
Write-Host "  MarginV=N → Bottom margin in pixels (relative to video height)"
Write-Host ""
Write-Host "Common issues:"
Write-Host "  - Background missing? Check OutlineColour, NOT BackColour"
Write-Host "  - Text too big/small? FontSize scales with FFmpeg internal PlayResY (~288)"
Write-Host "  - Position wrong? MarginV uses probed video height, fallback 720"
