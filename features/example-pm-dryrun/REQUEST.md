# Feature request: Example PM dry-run

> Reference example illustrating the six-element REQUEST format consumed by
> `.github/agents/product-manager.md`. Not a real feature; do not execute.
> Lives at state `defined` so `scripts/feature-board.ps1` shows it in the
> first lane.

## 1. Problem & Goals

Contributors arriving on Toaster have no concrete example of what a fully
PM-planned feature folder looks like. A reference bundle (REQUEST + PRD +
BLUEPRINT + tasks.sql + coverage.json) makes the feature-pm skill
self-documenting.

**Goal:** ship a checked-in example feature directory that
`scripts/check-feature-coverage.ps1 -All` accepts, so contributors can copy
the structure for real features.

## 2. Desired Outcome & Acceptance Criteria

When a contributor runs `pwsh scripts/feature-board.ps1`, they see this
feature in the `defined` lane with its short title.
When they run `pwsh scripts/check-feature-coverage.ps1 -Feature example-pm-dryrun`,
the script exits 0.

(See `PRD.md` for the formalised AC list.)

## 3. Scope Boundaries

### In scope

- A REQUEST.md, PRD.md, BLUEPRINT.md, tasks.sql, coverage.json, and
  per-task context.md under tasks/example-task-1/.
- STATE.md initialised to `defined`.

### Out of scope (explicit)

- Any production code change. This bundle is documentation-by-example.
- A real worktree (no `feat/example-pm-dryrun` branch should be created).
- Execution. STATE.md must never advance past `defined` for this example.

## 4. References to Existing Code

- `.github/agents/product-manager.md` — the agent that consumes this format.
- `.github/skills/feature-pm/SKILL.md` — the skill that invokes the agent.
- `scripts/check-feature-coverage.ps1` — the gate that this example must pass.
- `scripts/feature-board.ps1` — the terminal Kanban that surfaces this lane.

## 5. Edge Cases & Constraints

- Must remain valid even if more allowed verifier `kind` values are added later.
- Must not reference fixtures the repo does not ship (no proprietary media).
- Must use ASCII only (no smart quotes).

## 6. Data Model (optional)

n/a — this example does not introduce data.

## Q&A

(Populated during Phase 5 of a real feature. None for this example.)
