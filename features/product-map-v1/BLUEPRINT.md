# Blueprint: Product map to v1 launch

This blueprint documents how the discovery in `PRD.md` was produced
and how subsequent feature bundles should branch off from it. It is
not an implementation blueprint (this bundle has no implementation).

## Architecture decisions

This bundle produces planning artifacts only. The "architecture" is
the inventory + roadmap structure in `PRD.md`. No production code
patterns are introduced. The single-source-of-truth and
local-only-inference rules from `AGENTS.md:83-91` are honored by
**not** proposing anything that violates them.

## How the inventory was derived (methodology)

So a future maintainer can re-run the discovery against an evolved
codebase, the steps were:

1. **Canonical context first.** Read in this order: `AGENTS.md`,
   `PRD.md` (root), `README.md`, `docs/build.md`, `docs/testing-kb.md`.
   Cite line numbers for every claim about repo conventions.

2. **Existing feature bundles.** For every subdirectory of
   `features/` except `.templates/` and `example-pm-dryrun/`, read
   `STATE.md`, the head of `REQUEST.md`, and the latest `journal.md`
   entry to determine what is in flight and avoid re-proposing it.
   Captured as a one-paragraph summary per bundle.

3. **Backend capability crawl.** Walk `src-tauri/src/` and report,
   per file: (a) summary, (b) public commands / pub fns,
   (c) experimental / dead / undocumented surfaces, (d) FFmpeg
   invocations, (e) entry-point file:line. Used `task explore` agent
   with explicit "no large code dumps" instruction so the inventory
   is summary-only.

4. **Frontend capability crawl.** Walk `src/`, focusing on routes /
   navigation / settings panels / stores / hooks / i18n namespaces.
   Flag every settings key without a backend handler, every i18n
   key without a UI consumer, every component file under
   `components/settings/` not reachable from
   `components/Sidebar.tsx:24-43` `SECTIONS_CONFIG`.

5. **Decide "shipped vs partial vs dead-code-still-present vs
   undocumented" with explicit heuristics:**
   - **shipped** = code path runs end-to-end in a normal user
     session AND has a UI affordance OR is consumed by another
     shipped path.
   - **partial** = code path runs but only part of the surface is
     exposed (e.g. backend command exists, UI is missing).
   - **dead-code-still-present** = compiled, marked
     `#[allow(dead_code)]`, or has zero callers outside tests.
   - **undocumented** = working in code, missing from README /
     AGENTS.md / settings labels / user-visible strings.

6. **Gap analysis.** For each gap, ask: "If a stranger downloads the
   v1.0 installer and runs it, will they hit this?" If yes →
   Blocker. "Will they complain about it on launch day?" → Strongly
   Recommended. "Differentiator?" → Nice-to-have.

7. **FFmpeg opportunity map.** For each candidate filter / capability,
   evaluate (a) value, (b) complexity, (c) UX risk, (d) Include /
   Defer / Reject, with a one-line justification. Reject any item
   that requires a network call (none on this list do).

8. **Anti-scope.** Cite `AGENTS.md` line for every exclusion to make
   the rule, not the agent's judgement, the reason.

9. **Open questions.** Anything that materially changes a roadmap
   item's scope and is not unambiguously decided by the codebase
   becomes a numbered §8 question.

## Component & module touch-list

This bundle creates only files under `features/product-map-v1/`. It
does not touch any source file, script, or instruction file.

## Single-source-of-truth placement

N/A — no dual-path code is introduced. The roadmap items that
**will** introduce dual-path code (caption-parity-eval, time-stretch,
chapter markers) all carry the SST rule forward as part of their own
bundle's BLUEPRINT requirement.

## How subsequent feature bundles branch off this map

Each item in `PRD.md` §6 (roadmap) becomes its own
`features/<slug>/` bundle when the human is ready to execute it. The
recommended workflow is:

```powershell
# Pick a roadmap item, e.g. 1.3 loudness-preflight
pwsh scripts/scaffold-feature.ps1 -Slug loudness-preflight -Worktree

# Then invoke the PM agent (via the feature-pm skill) with a brief
# that points back at this map:
#   "Implement product-map-v1 §6 Milestone 1 item 1.3 loudness-preflight.
#    Coverage hint: see features/product-map-v1/coverage.json
#    roadmap_hints['1.3-loudness-preflight'].
#    Resolved open questions from product-map-v1 §8: Q2 = preflight only."
```

The PM agent then runs its normal eight-phase loop scoped to that
single roadmap item. Each item's PRD must:

1. Quote the matching `PRD.md` §4 gap ID(s) and §5 FFmpeg item
   ID(s) it closes.
2. Cite the resolved §8 open question(s) it depends on.
3. Use the verifier suggested in `coverage.json roadmap_hints`
   (or document why a different one is needed).

## Data flow

```
features/product-map-v1/PRD.md     <-- this bundle
        |
        | (human picks a roadmap item)
        v
scripts/scaffold-feature.ps1       <-- creates features/<roadmap-slug>/
        |
        v
feature-pm skill (product-manager) <-- 8-phase loop, scoped to ONE item
        |
        v
features/<roadmap-slug>/PRD.md + coverage.json + tasks.sql
        |
        v
superpowers:executing-plans / subagent-driven-development
```

## Migration / compatibility

Not applicable. The map is additive; existing feature bundles in
`reviewing` / `planned` retain authority over their own surface.
Where a roadmap item overlaps with an in-flight bundle (e.g. roadmap
1.1 `unreachable-surface-purge` extends `remove-history-and-legacy`),
the roadmap item must wait until the in-flight bundle reaches
`shipped` and may then cite its journal as prior art.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Inventory drifts from reality between this map and execution | Re-run §1-§3 of the methodology at the start of each Milestone (cheap; the explore-agent crawl is < 5 min) | AC-001-a / AC-002-a re-checked per milestone |
| Coverage gate accepts `manual` verifiers that only point back at the doc, not at observable behavior | Documented in `journal.md` as Q9; proposed `kind: doc-section` script amendment | AC-005-a + the `journal.md` deviation note |
| Human resolves §8 open questions inconsistently between roadmap items | Each subsequent bundle must cite the §8 Q# it relies on; cross-bundle conflicts surface at `feature-board.ps1` review | Per-bundle PRD will inherit |
| FFmpeg item complexity estimate is wrong | S/M/L is a hint, not a contract; each scaffolded bundle re-estimates | Per-bundle PRD will inherit |
| Roadmap proposes a Handy-era extension by accident | Anti-scope §7 cites `handy-legacy-pruning`; PM agent must invoke that skill before scaffolding any roadmap item that touches a Handy file | AC-006-a |
| Coverage gate is bypassed | `scripts/check-feature-coverage.ps1 -All` runs in CI per `AGENTS.md:274` | n/a (CI gate) |
