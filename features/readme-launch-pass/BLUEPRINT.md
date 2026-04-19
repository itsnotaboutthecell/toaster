# Blueprint: readme launch pass

## Architecture decisions

- **R-001 (framing + launch assets):** Replace `README.md` end-to-
  end. Follow the common open-source README template already used
  in-tree in spirit by the current file
  (`README.md:1-96`): hero heading, one-line tagline, short "what
  Toaster does today" bullets, Quickstart, Build, Launch (monitored
  launcher + portable mode), Evals, Roadmap, Contributing, License,
  Fork-acknowledgment. The rewrite **links** canonical docs
  (`docs/build.md`, `CONTRIBUTING.md`, `AGENTS.md`,
  `features/product-map-v1/PRD.md`) rather than duplicating content.
- **R-002 (Handy purge + link integrity):** Single fork-
  acknowledgment paragraph at the bottom is the only permitted
  mention of "Handy". Tagline (currently
  `README.md:3-5`) is rewritten to drop the "Forked from Handy"
  clause. Every relative link is hand-verified against `git ls-files`
  before the follow-up bundle claims done.
- **R-003 (badge hygiene):** Survey the current README for badges
  (today: none at `README.md:1-96`). If the follow-up adds any, they
  point only at Toaster-owned URLs.

## Component & module touch-list

- `README.md` — full rewrite in the follow-up execution bundle.
  Source of current text: `README.md:1-96`.
- (Read-only, linked from README) `docs/build.md`,
  `CONTRIBUTING.md`, `AGENTS.md`, `scripts/launch-toaster-
  monitored.ps1`, `scripts/eval/` directory,
  `features/product-map-v1/PRD.md`,
  `src-tauri/src/commands/mod.rs:26-30` (for the portable-mode
  mention — the README references the feature, not the source
  directly).

## Single-source-of-truth placement

Documentation bundle — no runtime dual-path logic. Documentation
SSOT rules applied:

- **Build steps SSOT:** `docs/build.md`. README shows a minimal
  "bun install -> setup-env -> launch" skeleton and links out for
  everything else.
- **Contribution guide SSOT:** `CONTRIBUTING.md`. README links,
  does not duplicate.
- **Launch-protocol rules SSOT:** `AGENTS.md` §"Launch protocol" /
  Critical rules (`AGENTS.md:37-45`). README references these rules
  rather than restating them.
- **Roadmap SSOT:** `features/product-map-v1/PRD.md` (Milestone
  table at §6). README links to it; does not inline the table.
- **Eval script catalog SSOT:** `scripts/eval/` (filenames are self-
  describing). README links to the directory and provides a one-
  sentence summary of what the suite covers.

## Data flow

N/A — documentation edit only.

## README structure (authoritative for the follow-up bundle)

Pin the section order so the follow-up execution bundle has no
design latitude:

1. `# Toaster` heading.
2. One-line hero tagline: "Toaster is a transcript-first desktop
   editor for spoken audio and video — edit media by editing text."
3. Optional badge row (Toaster-only; see R-003). Empty is allowed.
4. `[TODO: screenshot of editor/transcript view]` placeholder.
5. `## What Toaster does today` — 5 to 8 outcome-oriented bullets.
   No "Forked from Handy" phrasing here.
6. `## Quickstart` — links `docs/build.md`; shows only the minimum
   `bun install --frozen-lockfile`, `.\scripts\setup-env.ps1`, and
   `.\scripts\launch-toaster-monitored.ps1` lines.
7. `## Build` — one-line pointer to `docs/build.md`.
   `[TODO: screenshot of export dialog]` placeholder if export-UI
   framing is added here.
8. `## Launch protocol` — explains the monitored launcher is
   required on Windows, mentions portable mode (cite
   `src-tauri/src/commands/mod.rs:26-30` in a note), links to
   AGENTS.md Critical rules (`AGENTS.md:37-45`).
9. `## Evals` — links `scripts/eval/` and one sentence per eval
   family (edit quality, audio boundary, caption parity, multi-
   backend parity, verifier suite, local-LLM gate).
10. `## Platform support` — honest cross-platform status: Windows
    primary; macOS and Linux community-supported pending
    `mac-build-verify` / `linux-build-verify`
    (`features/product-map-v1/PRD.md:491-494`).
11. `## Roadmap` — one sentence + link to
    `features/product-map-v1/PRD.md`. No inlined milestone table.
12. `## Contributing` — one sentence + link to `CONTRIBUTING.md`.
13. `## License` — "MIT — see [LICENSE](LICENSE)."
14. `## Acknowledgments` — fork-acknowledgment footer. **Only** place
    "Handy" may appear. Example copy: "Toaster is a fork of Handy;
    see [LICENSE](LICENSE) for attribution."

## Screenshot placeholder sites (for follow-up work)

BLUEPRINT flags where a follow-up bundle should add images. The
rewrite inserts markdown comment placeholders at each site:

- After the hero tagline: `[TODO: screenshot of editor/transcript
  view]`.
- Inside the Build section: `[TODO: screenshot of export dialog]`.
- Optionally inside Evals: `[TODO: screenshot of eval JSON report]`.

No binary assets are added in this bundle or the follow-up execution
pass.

## Migration / compatibility

- No code, API, or config migration.
- Downstream consumers of the README (GitHub landing page, vendored
  forks) see a content diff only. No front-matter, no HTML anchors
  that external sites might link to, so anchor-link breakage is not
  a realistic risk.
- The AGENTS.md canonical-instructions rule is preserved: README
  links to AGENTS.md and does not duplicate any of its critical
  rules verbatim.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| README drifts back into Handy framing during future edits | Coverage gate + AC-002-a grep contract reruns anytime README changes | AC-002-a |
| A link added in the rewrite points at a not-yet-created file | `check-readme-links.ps1` (planned) resolves every relative link and fails fast | AC-002-b |
| Tagline buried below a long badge row, moving the transcript-first framing past line 3 | AC-001-a pins "within first three non-empty content lines" | AC-001-a |
| Roadmap table gets inlined and then rots | BLUEPRINT §"README structure" item 11 forbids inlining; AC-001-d verifies link presence | AC-001-d |
| Badge pointing at upstream Handy CI status silently reappears | AC-003-a regex gate | AC-003-a |
| Evals section drops to "see scripts/" with no context | AC-001-c requires both the link and a descriptive sentence | AC-001-c |