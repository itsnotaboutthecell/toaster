# Pre-tool-use policy hook for Toaster.
# Enforces AGENTS.md hard gates at the tool-call layer.
#   1. No bare `cargo tauri dev` / `npm run tauri dev` (see AGENTS.md "Launch protocol")
#   2. No name-based process kills (see AGENTS.md Windows requirements)
#   3. No unscoped full-workspace cargo clippy/check (see AGENTS.md "Cargo runtime expectations")
#   4. No new *.md at repo root (see AGENTS.md "Tips and tricks")
# Fail soft on any internal error: warn on stderr, exit 0, never spuriously deny.

$ErrorActionPreference = 'Stop'

function Emit-Deny([string]$reason) {
    $obj = [ordered]@{ permissionDecision = 'deny'; permissionDecisionReason = $reason }
    $json = $obj | ConvertTo-Json -Compress
    [Console]::Out.Write($json)
    exit 0
}

function Warn([string]$msg) { [Console]::Error.WriteLine($msg) }

try {
    $raw = [Console]::In.ReadToEnd()
    if ([string]::IsNullOrWhiteSpace($raw)) { exit 0 }
    $payload = $raw | ConvertFrom-Json
    $toolName = [string]$payload.toolName
    $argsRaw  = [string]$payload.toolArgs
    if ([string]::IsNullOrWhiteSpace($argsRaw)) { exit 0 }
    $toolArgs = $argsRaw | ConvertFrom-Json
} catch {
    Warn "pretool-policy: input parse failed: $($_.Exception.Message)"
    exit 0
}

# --- shell-command gates (bash/powershell tools) ---
if ($toolName -eq 'bash' -or $toolName -eq 'powershell') {
    $cmd = [string]$toolArgs.command
    if (-not $cmd) { exit 0 }
    $lc = $cmd.ToLowerInvariant()

    # Gate 2: name-based process kills (non-negotiable, no bypass).
    if ($lc -match 'stop-process\s+-name' -or
        $lc -match 'taskkill\s+(/f\s+)?/im' -or
        $lc -match '\bpkill\s+-f\b') {
        Emit-Deny "AGENTS.md: use Stop-Process -Id <PID>. Name-based process killing is not allowed."
    }

    # Gate 1: bare tauri dev launches.
    if ($env:COPILOT_ALLOW_BARE_TAURI_DEV -ne '1') {
        $isBareTauri = ($lc -match 'cargo\s+tauri\s+dev' -or $lc -match 'npm\s+run\s+tauri\s+dev')
        $isWrapped   = ($lc -match 'launch-toaster-monitored')
        if ($isBareTauri -and -not $isWrapped) {
            Emit-Deny "AGENTS.md: use .\scripts\launch-toaster-monitored.ps1 so startup is observed. See AGENTS.md 'Launch protocol'. Override with COPILOT_ALLOW_BARE_TAURI_DEV=1."
        }
    }

    # Gate 3: unscoped full-workspace cargo clippy/check.
    # Supports toolchain selector (e.g. `cargo +nightly clippy`) and both the
    # space-separated and equals-separated scope forms (`-p foo`, `--package=foo`).
    # Info queries (`--help`/`-h`/`--version`/`-V`) are allowed through.
    #
    # We strip quoted segments first so commit messages, echo strings, etc.
    # that mention "cargo check" or "cargo clippy" don't trigger the gate.
    # Then we require `cargo` to sit at a shell-command boundary (start of
    # line, after `;`, `&&`, `||`, `|`, or `$(`/backtick) so that text like
    # `git commit -m "...cargo check..."` cannot match even before stripping.
    if ($env:COPILOT_ALLOW_FULL_CLIPPY -ne '1') {
        # Strip "..." and '...' segments (including escaped quotes inside)
        # before scanning. Non-greedy, multi-pass.
        $scan = $lc
        $scan = [regex]::Replace($scan, '"(?:\\.|[^"\\])*"', ' ')
        $scan = [regex]::Replace($scan, "'(?:\\.|[^'\\])*'", ' ')
        if ($scan -match '(?:^|[;&|`]|\$\()\s*cargo(\s+\+\S+)?\s+(clippy|check)\b') {
            $isInfoQuery = ($scan -match '(?:^|\s)(?:--help|-h|--version|-V)(?:\s|$)')
            $hasScope    = ($scan -match '(?:^|\s)(?:-p|--package)(?:\s|=)')
            if (-not $isInfoQuery -and -not $hasScope) {
                Emit-Deny "AGENTS.md cargo runtime: cold full-workspace cargo clippy/check on this tree takes 2-10+ minutes. During iteration, scope with -p <crate>. Override with COPILOT_ALLOW_FULL_CLIPPY=1."
            }
        }
    }
}

# --- Gate 4: new *.md at repo root via `create` tool ---
if ($toolName -eq 'create') {
    $path = [string]$toolArgs.path
    if ($path) {
        try {
            $full = [System.IO.Path]::GetFullPath($path)
            $repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
            $leaf = Split-Path -Leaf $full
            $parent = Split-Path -Parent $full
            $isRoot = ($parent.TrimEnd('\','/') -ieq $repoRoot.TrimEnd('\','/'))
            if ($isRoot -and $leaf -match '\.md$') {
                $allow = @('AGENTS.md','CLAUDE.md','CRUSH.md','CONTRIBUTING.md','CONTRIBUTING_TRANSLATIONS.md','README.md','SECURITY.md','PRD.md')
                # -ccontains is case-SENSITIVE; -contains is not. Enforce canonical casing.
                $onAllowlist = $allow -ccontains $leaf
                if (-not $onAllowlist -and $env:COPILOT_ALLOW_ROOT_MARKDOWN -ne '1') {
                    Emit-Deny "Session-scoped notes go in the Copilot session workspace, not the repo root. See AGENTS.md 'Tips and tricks' (no markdown in repo for planning/notes/tracking). Override with COPILOT_ALLOW_ROOT_MARKDOWN=1."
                }
            }
        } catch {
            Warn "pretool-policy: path resolve failed: $($_.Exception.Message)"
        }
    }
}

exit 0
