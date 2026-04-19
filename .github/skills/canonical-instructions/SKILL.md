---
name: canonical-instructions
description: 'Use whenever editing AI-instruction files (AGENTS.md, .github/copilot-instructions.md, README.md). Prevents instruction sprawl and contradictions by enforcing AGENTS.md as the single source of truth.'
---

# Canonical Instructions

## Overview

Toaster previously had four overlapping AI-instruction files that drifted out of sync (e.g., `-ObservationSeconds 45` vs `120`; `npm run tauri dev` vs monitored-script-only). That class of bug is a documentation problem, not a code problem.

**Core principle:** `AGENTS.md` is the single source of truth. Every other instruction file is a pointer.

## The Iron Law

```
ONE EDIT TO GUARDRAILS = ONE EDIT, IN AGENTS.md
```

If a rule exists in more than one place, the copies will drift. Do not introduce copies.

## What Lives Where

| File | Purpose | May contain |
| --- | --- | --- |
| `AGENTS.md` | Canonical rules, launch protocol, boundaries, skills/agents index | Everything |
| `.github/copilot-instructions.md` | Pointer for GitHub Copilot | "See AGENTS.md" + tooling-specific notes only |
| `README.md` | Human-oriented project overview | User-facing description, quick start; must not contradict AGENTS.md |
| `docs/PRD.md` | Product requirements | Vision / scope / acceptance criteria |
| `docs/build.md` | Platform build setup | Toolchain details referenced from AGENTS.md |

## Gate Function

Before editing any instruction file:

```
1. ASK: Does this rule belong in AGENTS.md? (answer is almost always yes)
2. If yes: edit AGENTS.md. Stop.
3. If the rule is truly tool-specific (Claude-only, Copilot-only): edit
   only that tool's pointer file and prefix the section with "Tool-specific:"
4. NEVER duplicate a rule across files.
```

## Red Flags — STOP

- About to paste the same paragraph into two files
- Updating a launch-protocol number in one file but not AGENTS.md
- Adding a new guardrail to a pointer file because "that's where I'm reading from"
- Creating a new `*.md` instruction file at the repo root

## Reconciliation Procedure

If you discover a contradiction:

1. Determine which value is correct (ask the user if unclear).
2. Fix AGENTS.md first.
3. Replace the contradicting section in every pointer file with a reference to AGENTS.md.
4. Run a grep for the old value across the repo to confirm no third copy exists.

## When To Apply

- Any edit to `AGENTS.md`, `.github/copilot-instructions.md`
- Any time you notice the same instruction in two files
- Before adding a new top-level `*.md` to the repo
- Whenever a launch-protocol or guardrail number changes
