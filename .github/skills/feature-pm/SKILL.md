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

In VS Code Copilot Chat, invoke the `@product-manager` agent directly:

```
@product-manager <user's feature request, plus slug, plus any fixed inputs>
```

The agent is defined at `.github/agents/product-manager.agent.md` and has
the tools and model it needs. It will read its own role spec on startup
(RULE 0, Startup ritual, 8 phases, Hand-off self-check).

After the agent completes, verify on disk (`ls features/<slug>/`) and rerun
the promotion gates:

```bash
pwsh scripts/feature/check-feature-coverage.ps1 -Feature <slug>
pwsh scripts/feature/check-feature-tasks.ps1 -Feature <slug>
```

Expected outcome:
- `features/<slug>/` with REQUEST, PRD, BLUEPRINT, tasks.sql,
  coverage.json, journal.md, and per-task context briefings.
- Both gate scripts exit 0.
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
| `script`     | `pwsh scripts/eval/eval-edit-quality.ps1 ...`       |
| `manual`     | `.\scripts\launch-toaster-monitored.ps1` + numbered steps |

`scripts/feature/check-feature-coverage.ps1` enforces this and is wired into CI.

## State machine

`features/<slug>/STATE.md` contains exactly one of:

- `defined`   - REQUEST exists, PRD does not.
- `planned`   - PRD + BLUEPRINT + coverage.json all green.
- `executing` - `superpowers:executing-plans` started.
- `reviewing` - implementation done, in `requesting-code-review` /
                `code-reviewer` loop.
- `shipped`   - merged to main.
- `archived`  - soft-deleted; restorable.

`scripts/feature/feature-board.ps1` reads every `STATE.md` and prints a 6-lane
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

- `.github/agents/product-manager.agent.md` - the agent this skill invokes.
- `superpowers:brainstorming` - run first if requirements are unclear.
- `superpowers:writing-plans` - the PM agent's "tasks" phase replaces this
  for full features but the breakdown style (2-5 minute tasks, dep order)
  is identical.
- `superpowers:executing-plans` / `subagent-driven-development` - consume
  the per-task `context.md` briefings produced by the PM agent.
- `superpowers:verification-before-completion` - every AC's verifier
  command in `coverage.json` is what this skill runs to satisfy that rule.
- `canonical-instructions` - AGENTS.md remains the single source of truth.
