<# .SYNOPSIS
    Architecture guard hook: deny edits that add Qt/UI includes to libtoaster.
    Called by Copilot hooks before edit/create tool use on libtoaster/ files.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$raw = [Console]::In.ReadToEnd()
$input_data = $raw | ConvertFrom-Json

$toolName = $input_data.toolName

# Only check edit and create operations
if ($toolName -ne 'edit' -and $toolName -ne 'create') {
    exit 0
}

$toolArgs = $input_data.toolArgs | ConvertFrom-Json

# Determine the file path being edited
$filePath = ''
if ($toolArgs.PSObject.Properties['path']) {
    $filePath = $toolArgs.path
} elseif ($toolArgs.PSObject.Properties['filePath']) {
    $filePath = $toolArgs.filePath
}

# Only guard libtoaster/ files
if ($filePath -notmatch 'libtoaster[/\\]') {
    exit 0
}

# Check for Qt/UI includes in the new content
$newContent = ''
if ($toolArgs.PSObject.Properties['newString']) {
    $newContent = $toolArgs.newString
} elseif ($toolArgs.PSObject.Properties['content']) {
    $newContent = $toolArgs.content
}

$forbiddenPatterns = @(
    '#include\s*<Q\w+>',
    '#include\s*"Q\w+\.h"',
    '#include\s*<QtCore',
    '#include\s*<QtGui',
    '#include\s*<QtWidgets'
)

foreach ($pattern in $forbiddenPatterns) {
    if ($newContent -match $pattern) {
        $output = @{
            permissionDecision = 'deny'
            permissionDecisionReason = "Architecture violation: Qt includes are forbidden in libtoaster/. The core library must have zero knowledge of Qt. Move UI code to frontend/."
        }
        $output | ConvertTo-Json -Compress
        exit 0
    }
}

# Allow by default
exit 0
