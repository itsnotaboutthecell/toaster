#!/usr/bin/env bash
# Pre-tool-use policy hook for Toaster (bash).
# Mirror of pretool-policy.ps1. See that file + AGENTS.md for rule citations.
# Fail soft on any internal error.
set -euo pipefail

warn() { printf '%s\n' "$*" >&2; }
deny() {
  # Single-line compact JSON on stdout, nothing else.
  printf '{"permissionDecision":"deny","permissionDecisionReason":%s}' \
    "$(printf '%s' "$1" | jq -Rs .)"
  exit 0
}

if ! command -v jq >/dev/null 2>&1; then
  warn "pretool-policy: jq not found; skipping enforcement."
  exit 0
fi

input="$(cat || true)"
if [ -z "$input" ]; then exit 0; fi

tool_name=$(printf '%s' "$input" | jq -r '.toolName // ""')
tool_args_raw=$(printf '%s' "$input" | jq -r '.toolArgs // ""')
if [ -z "$tool_args_raw" ]; then exit 0; fi

# toolArgs is a JSON string — parse as JSON.
parse() { printf '%s' "$tool_args_raw" | jq -r "$1" 2>/dev/null || printf ''; }

if [ "$tool_name" = "bash" ] || [ "$tool_name" = "powershell" ]; then
  cmd=$(parse '.command // ""')
  if [ -n "$cmd" ]; then
    lc=$(printf '%s' "$cmd" | tr '[:upper:]' '[:lower:]')

    # Gate 2: name-based process kills (no bypass).
    if printf '%s' "$lc" | grep -Eq 'stop-process[[:space:]]+-name|taskkill[[:space:]]+(/f[[:space:]]+)?/im|\bpkill[[:space:]]+-f\b'; then
      deny "AGENTS.md: use Stop-Process -Id <PID>. Name-based process killing is not allowed."
    fi

    # Gate 1: bare tauri dev launches.
    if [ "${COPILOT_ALLOW_BARE_TAURI_DEV:-}" != "1" ]; then
      if printf '%s' "$lc" | grep -Eq 'cargo[[:space:]]+tauri[[:space:]]+dev|npm[[:space:]]+run[[:space:]]+tauri[[:space:]]+dev'; then
        if ! printf '%s' "$lc" | grep -q 'launch-toaster-monitored'; then
          deny "AGENTS.md: use .\\scripts\\launch-toaster-monitored.ps1 so startup is observed. See AGENTS.md 'Launch protocol'. Override with COPILOT_ALLOW_BARE_TAURI_DEV=1."
        fi
      fi
    fi

    # Gate 3: unscoped full-workspace cargo clippy/check.
    # Supports toolchain selector (e.g. `cargo +nightly clippy`) and both the
    # space-separated and equals-separated scope forms (`-p foo`, `--package=foo`).
    # Info queries (`--help`/`-h`/`--version`/`-V`) are allowed through.
    #
    # We strip quoted segments first so commit messages, echo strings, etc.
    # that mention "cargo check"/"cargo clippy" don't trigger the gate, and
    # require `cargo` to sit at a shell-command boundary (start of line,
    # after `;`, `&&`, `||`, `|`, backtick, or `$(`).
    if [ "${COPILOT_ALLOW_FULL_CLIPPY:-}" != "1" ]; then
      scan=$(printf '%s' "$lc" \
        | sed -E 's/"(\\.|[^"\\])*"/ /g' \
        | sed -E "s/'(\\\\.|[^'\\\\])*'/ /g")
      if printf '%s' "$scan" | grep -Eq '(^|[;&|`]|\$\()[[:space:]]*cargo([[:space:]]+\+[^[:space:]]+)?[[:space:]]+(clippy|check)\b'; then
        if ! printf '%s' "$scan" | grep -Eq '(^|[[:space:]])(--help|-h|--version|-V)([[:space:]]|$)'; then
          if ! printf '%s' "$scan" | grep -Eq '(^|[[:space:]])(-p|--package)([[:space:]]|=)'; then
            deny "AGENTS.md cargo runtime: cold full-workspace cargo clippy/check on this tree takes 2-10+ minutes. During iteration, scope with -p <crate>. Override with COPILOT_ALLOW_FULL_CLIPPY=1."
          fi
        fi
      fi
    fi
  fi
fi

if [ "$tool_name" = "create" ]; then
  path=$(parse '.path // ""')
  if [ -n "$path" ]; then
    repo_root=$(cd "$(dirname "$0")/../.." && pwd)
    # Resolve absolute path (best-effort).
    case "$path" in
      /*|?:*|?:/*|?:\\*) full="$path" ;;
      *) full="$repo_root/$path" ;;
    esac
    leaf=$(basename "$full")
    parent=$(dirname "$full")
    # Normalize trailing slashes for comparison.
    parent_norm="${parent%/}"
    root_norm="${repo_root%/}"
    if [ "$parent_norm" = "$root_norm" ] && printf '%s' "$leaf" | grep -qE '\.md$'; then
      case "$leaf" in
        AGENTS.md|CLAUDE.md|CRUSH.md|CONTRIBUTING.md|CONTRIBUTING_TRANSLATIONS.md|README.md|SECURITY.md|PRD.md)
          ;;
        *)
          if [ "${COPILOT_ALLOW_ROOT_MARKDOWN:-}" != "1" ]; then
            deny "Session-scoped notes go in the Copilot session workspace, not the repo root. See AGENTS.md 'Tips and tricks' (no markdown in repo for planning/notes/tracking). Override with COPILOT_ALLOW_ROOT_MARKDOWN=1."
          fi
          ;;
      esac
    fi
  fi
fi

exit 0
