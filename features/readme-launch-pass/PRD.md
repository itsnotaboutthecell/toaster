# PRD: readme launch pass

## Problem & Goals

Top-level `README.md` is the first artifact a v1.0 evaluator, a new
contributor, or a downstream forker sees. Today it still echoes the
Handy-era fork framing (`README.md:5`), does not surface the launch-
relevant assets Milestone 3 is built around (monitored launcher,
portable mode, eval suite, roadmap), and has no explicit statement of
cross-platform support status. This bundle produces the planning
artifacts needed to rewrite the README so it correctly frames Toaster
as a transcript-first video/audio editor and links every launch-
relevant asset. This is the `readme-launch-pass` item called out at
`features/product-map-v1/PRD.md:495` (Milestone 3, row 3.4).

Goal: after the follow-up execution bundle runs, a user who lands on
the GitHub repo can understand what Toaster is in three lines, find a
working quickstart + build + launch path in under 60 seconds of
reading, discover the eval suite, and reach the launch roadmap — all
without reading `Handy` anywhere except a fork-acknowledgment footer.

## Scope

### In scope

- Planning artifacts for a full rewrite of `README.md` with the
  structure pinned in `BLUEPRINT.md`.
- Screenshot placeholder markers (no new images).
- A short fork-acknowledgment footer paragraph — the only place
  "Handy" may appear.

### Out of scope (explicit)

- Any production code change (`.rs`, `.ts`, `.tsx`,
  `tauri.conf.json`, `Cargo.toml`, `package.json`).
- New screenshots, GIFs, or video captures.
- New i18n keys or locale-file edits.
- Inlining the Milestone 3 table; README links to
  `features/product-map-v1/PRD.md` instead.
- Edits to `docs/build.md`, `CONTRIBUTING.md`, or `AGENTS.md`. They
  remain canonical and are linked.
- Signing, installer, or release-note work — owned by other
  Milestone 3 bundles (`windows-installer-sign` etc.).

## Requirements

### R-001 — README frames Toaster correctly and surfaces launch assets

- Description: The rewritten `README.md` describes Toaster as a
  transcript-first video/audio editor in its hero tagline, contains
  the launch-relevant sections (Quickstart, Evals, Roadmap), and
  links every launch asset named in the BLUEPRINT.
- Rationale: Milestone 3 exit criteria require the README onboarding
  path to match reality
  (`features/product-map-v1/PRD.md:500-503`). A transcript-first
  tagline is the difference between an evaluator recognizing Toaster
  as a video editor vs. mistaking it for a dictation tool.
- Acceptance Criteria
  - AC-001-a — The rewritten `README.md` tagline describes Toaster
    as a transcript-first video/audio editor within its first three
    non-empty content lines (after the `# Toaster` heading).
  - AC-001-b — The Quickstart section contains a markdown link to
    `docs/build.md` and a markdown reference to
    `scripts/launch-toaster-monitored.ps1`.
  - AC-001-c — The Evals section contains a markdown link to
    `scripts/eval/` (or a file inside it) and at least one sentence
    describing what the eval suite covers.
  - AC-001-d — The Roadmap section contains a markdown link to
    `features/product-map-v1/PRD.md`.

### R-002 — Handy references and link integrity

- Description: The rewritten README contains no stale Handy-era
  framing outside a single fork-acknowledgment footer paragraph, and
  every internal relative link resolves.
- Rationale: Evaluators form first impressions from the tagline and
  feature bullets. Handy references there mis-position Toaster.
  Dead links undermine trust in documentation quality.
- Acceptance Criteria
  - AC-002-a — `grep -c "Handy" README.md` returns at most 1, and
    that single match (if present) is inside the fork-acknowledgment
    footer section.
  - AC-002-b — Every relative markdown link of the form
    `[text](relative-path)` in `README.md` resolves to an existing
    file or directory in the repository.

### R-003 — Badges only point at Toaster

- Description: Any badge retained in the rewritten README points at
  Toaster-owned URLs (e.g. `itsnotaboutthecell/toaster`,
  `shields.io` queries for this repo, the MIT license). No badge
  links to the upstream Handy repository or its CI.
- Rationale: Badges are the first visual; a Handy-pointing badge
  misrepresents ownership and can link to an unrelated CI status.
- Acceptance Criteria
  - AC-003-a — No image-link line in `README.md` (lines matching
    `!\[.*\]\(.*\)` or `\[!\[.*\]\(.*\)\]\(.*\)`) references the
    upstream Handy repository (case-insensitive match on `Handy`
    inside a badge URL).

## Edge cases & constraints

- Documentation-only: zero production-code changes.
- README is English-only; no i18n impact and no new locale keys.
- Screenshot insertions are TODO placeholders — no binary assets
  added in this or the follow-up bundle.
- Internal links must resolve today. Any link to a file planned but
  not yet created would be a dead link and violates AC-002-b.
- The fork-acknowledgment footer is required (MIT fork hygiene) but
  is the **only** place "Handy" may appear.

## Data model (if applicable)

N/A — documentation bundle.

## Non-functional requirements

- File cap: `README.md` stays comfortably under the 800-line repo cap
  that applies to `.rs`/`.ts`/`.tsx`. Target: under 200 lines for
  scanability.
- Readability: an evaluator reaches "what is Toaster" within three
  lines and a working quickstart within one screen of scrolling.
- Maintenance: README links to `CONTRIBUTING.md`, `docs/build.md`,
  and `AGENTS.md` rather than duplicating any of their content, so
  future edits do not create divergence.