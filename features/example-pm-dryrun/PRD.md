# PRD: Example PM dry-run

## Problem & Goals

Provide a reference planning bundle so contributors can copy the structure
for real features. See `REQUEST.md` for the full motivation.

## Scope

### In scope
- Static example artifacts under `features/example-pm-dryrun/`.
- Inclusion in `scripts/check-feature-coverage.ps1 -All` and
  `scripts/feature-board.ps1` output.

### Out of scope (explicit)
- Production code edits.
- Worktree / branch creation.
- Advancing past STATE = `defined`.

## Requirements

### R-001 — Bundle is structurally complete

- Description: every file the PM agent's Phase 8 promises to produce exists
  for this example.
- Rationale: a contributor cloning the structure must see a green example.
- Acceptance Criteria
  - AC-001-a — `features/example-pm-dryrun/{STATE.md,REQUEST.md,PRD.md,BLUEPRINT.md,tasks.sql,coverage.json,journal.md}` all exist.
  - AC-001-b — `pwsh scripts/check-feature-coverage.ps1 -Feature example-pm-dryrun` exits 0.

### R-002 — Bundle remains inert

- Description: this example must not be executed or shipped accidentally.
- Rationale: it is documentation, not work.
- Acceptance Criteria
  - AC-002-a — `STATE.md` contains exactly `defined`.

## Edge cases & constraints

- ASCII only.
- No proprietary fixtures.
- Verifier `kind` values used here must remain valid as the script evolves
  (the gate script itself is the source of truth).

## Data model

n/a.

## Non-functional requirements

- The bundle must remain under 800 lines per file (AGENTS.md file-size cap).
