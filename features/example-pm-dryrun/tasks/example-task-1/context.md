# Context briefing: example-task-1 (Author the example bundle)

> Curated for a fresh subagent. Read only this file and the paths it
> references; do NOT load other PRD sections or unrelated repo files.

## R-IDs covered

- R-001 (Bundle is structurally complete)
- R-002 (Bundle remains inert)

## PRD slice

See `features/example-pm-dryrun/PRD.md` sections for R-001 and R-002 only.

## Blueprint slice

See `features/example-pm-dryrun/BLUEPRINT.md` "Architecture decisions" for
R-001 and R-002.

## Files to read first

- `.github/agents/product-manager.md` (Phase 8 — defines the file layout)
- `scripts/check-feature-coverage.ps1` (the gate)

## Files to write

- `features/example-pm-dryrun/STATE.md` (single line: `defined`)
- `features/example-pm-dryrun/REQUEST.md`
- `features/example-pm-dryrun/PRD.md`
- `features/example-pm-dryrun/BLUEPRINT.md`
- `features/example-pm-dryrun/tasks.sql`
- `features/example-pm-dryrun/coverage.json`
- `features/example-pm-dryrun/journal.md`

## Verifier (from coverage.json)

```
pwsh scripts/check-feature-coverage.ps1 -Feature example-pm-dryrun
```

Expect exit code 0.

## Journal entries tagged with these R-IDs

(none yet — this is the first task)
