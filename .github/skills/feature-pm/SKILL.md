---
name: feature-pm
description: 'Use whenever a user request would otherwise jump straight into code without a PRD/Blueprint/coverage map. Forces invocation of the product-manager agent to produce features/<slug>/ planning artifacts and a machine-checkable coverage.json before any production edit. Toaster-specific; runs alongside superpowers:brainstorming and superpowers:writing-plans.'
---

# Feature PM

## Why this skill exists

Toaster's guardrails (AGENTS.md "Verified means the live app, not
`cargo check`", "Single source of truth for dual-path logic", "Local-only
inference") only hold if every feature is planned against them **before** code
is written. Without a structured planning artifact, the rules are enforced
post-hoc by review — which is too late.

This skill imports afkode's spec-driven-development discipline: a feature is a
**structured unit of work** (PRD + Blueprint + Task graph + Coverage map), not
a chat. Every acceptance criterion is traced to a real verifier (eval script,
cargo test, agent, or named live-app check) so the gate is machine-enforced,
not agent-enforced.

## When to invoke

Invoke this skill (which then invokes the `product-manager` agent) when the
user asks for:

- A new feature ("add ___", "build ___", "support ___").
- A refactor large enough to span >= 2 files or >= 1 manager.
- A migration (backend/storage/format/ASR backend swap).
- A batch of related bug fixes that share a regression surface.
- Anything where the user says "plan", "spec", "PM this", "write a PRD",
  "design", or "scope".

**Do not** invoke for:

- Single-file fixes (typo, one-line bug, lint cleanup).
- Documentation-only changes that are not AGENTS.md edits.
- Pure invocation of an existing skill/agent (e.g. "run the eval harness").

## How to invoke

**Do NOT use `agent_type: product-manager` with the `task` tool.** That
agent type is registered in this CLI environment with a `view`-only tool
restriction that makes file creation impossible — the agent will narrate
file contents back instead of writing them (known failure mode, six
documented occurrences before the fix below).

Invoke via `general-purpose` with the product-manager role spec inlined
as the prompt prefix:

```text
task(
  agent_type="general-purpose",
  name="pm-<slug>",
  mode="background",
  prompt="""You are acting as the Toaster Product Manager agent. Your
role specification lives at `.github/agents/product-manager.md` — read
it fully before starting, then follow it (RULE 0, Startup ritual, 8
phases, Hand-off self-check).

TASK: <user's feature request, plus slug, plus any fixed inputs>.
"""
)
```

This gives the subagent the full toolset (`create`, `edit`, `view`,
`powershell`, `grep`, `glob`) so RULE 0 is actually satisfiable.

After the agent completes, the controller MUST verify on disk
(`Get-ChildItem features/<slug>`) regardless of what the agent reports,
and rerun `check-feature-coverage.ps1` + `check-feature-tasks.ps1` to
confirm the gates.

Expected outcome:
- `features/<slug>/` with REQUEST, PRD, BLUEPRINT, tasks.sql,
  coverage.json, journal.md, and per-task context briefings.
- `scripts/check-feature-coverage.ps1 -Feature <slug>` exits 0.
- `scripts/check-feature-tasks.ps1 -Feature <slug>` exits 0.
- `features/<slug>/STATE.md` set to `planned`.
- Hand off to `superpowers:executing-plans` or
   superpowers:subagent-driven-development for implementation.
```

## Coverage gate

Every `AC-NNN-x` in `features/<slug>/PRD.md` must appear as a key in
`features/<slug>/coverage.json`. The verifier must be one of:

| kind         | example                                        |
|--------------|------------------------------------------------|
| `skill`      | `transcript-precision-eval`, `audio-boundary-eval` |
| `agent`      | `eval-harness-runner`, `cut-drift-fuzzer`, `waveform-diff` |
| `cargo-test` | `cd src-tauri; cargo test <name>`              |
| `script`     | `pwsh scripts/eval-edit-quality.ps1 ...`       |
| `manual`     | `.\scripts\launch-toaster-monitored.ps1` + numbered steps |

`scripts/check-feature-coverage.ps1` enforces this and is wired into CI.

## State machine

`features/<slug>/STATE.md` contains exactly one of:

- `defined`   - REQUEST exists, PRD does not.
- `planned`   - PRD + BLUEPRINT + coverage.json all green.
- `executing` - `superpowers:executing-plans` started.
- `reviewing` - implementation done, in `requesting-code-review` /
                `code-reviewer` loop.
- `shipped`   - merged to main.
- `archived`  - soft-deleted; restorable.

`scripts/feature-board.ps1` reads every `STATE.md` and prints a 6-lane
terminal Kanban. State transitions are made by the relevant skill/agent, not
by hand.

## Anti-patterns

- **"Just plan it"** without coverage - defeats the entire skill. Refuse.
- **"Use Stripe / OpenAI / Whisper-API"** - hosted inference is a hard No
  per AGENTS.md "Local-only inference".
- **Implementation instructions in REQUEST.md** ("use Zustand store `xyz`")
  - REQUEST captures *what*, BLUEPRINT captures *how*. Push implementation
  hints down into the blueprint.
- **Duplicating AGENTS.md guidance** into the new feature's docs - link,
  don't copy (`canonical-instructions`).
- **Skipping `superpowers:brainstorming`** for a request the user is still
  exploring - finish brainstorming first, then invoke this skill on the
  resolved intent.

## Related

- `.github/agents/product-manager.md` - the agent this skill invokes.
- `superpowers:brainstorming` - run first if requirements are unclear.
- `superpowers:writing-plans` - the PM agent's "tasks" phase replaces this
  for full features but the breakdown style (2-5 minute tasks, dep order)
  is identical.
- `superpowers:executing-plans` / `subagent-driven-development` - consume
  the per-task `context.md` briefings produced by the PM agent.
- `superpowers:verification-before-completion` - every AC's verifier
  command in `coverage.json` is what this skill runs to satisfy that rule.
- `canonical-instructions` - AGENTS.md remains the single source of truth.
