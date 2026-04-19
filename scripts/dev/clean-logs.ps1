# Remove ad-hoc log artifacts. Safe to run anytime.
Get-ChildItem -Path . -Filter '*.log' -File | Remove-Item -Force -ErrorAction SilentlyContinue
if (Test-Path .launch-monitor) { Get-ChildItem -Path .launch-monitor -Filter '*.log' -File | Remove-Item -Force -ErrorAction SilentlyContinue }
