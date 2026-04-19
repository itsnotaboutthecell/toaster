# Blueprint: Example PM dry-run

## Architecture decisions

- **R-001** (bundle complete): mirror the file layout produced by
  `.github/agents/product-manager.md` Phase 8. Pattern reference:
  `.github/agents/product-manager.md` "Phases" section.
- **R-002** (inert): hard-code `STATE.md` to `defined`. `feature-board.ps1`
  surfaces this lane unconditionally.

## Component & module touch-list

- `features/example-pm-dryrun/` — new directory, no other touchpoints.

## Single-source-of-truth placement

n/a — no preview/export concern. This is documentation.

## Data flow

```
contributor -> reads features/example-pm-dryrun/*
            -> runs scripts/feature-board.ps1
            -> runs scripts/check-feature-coverage.ps1 -Feature example-pm-dryrun
            -> copies layout to features/<their-slug>/
```

## Migration / compatibility

- None. New directory, no schema impact.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Coverage gate evolves and breaks the example | Keep example minimal; rely only on documented `kind` values | AC-001-b |
| Example accidentally treated as real work | STATE pinned to `defined`; PM agent rules forbid auto-advancing examples | AC-002-a |
