# Feature request: readme launch pass

## 1. Problem & Goals

Toaster's top-level `README.md` still reads like a Handy-era fork in
places and does not surface the launch-relevant assets a Milestone 3
reviewer expects to see. For the v1.0 launch the README must:

- Frame Toaster as a transcript-first video/audio editor (not a
  dictation tool or voice-input utility).
- Document the Toaster-specific onboarding path (env setup ->
  monitored launcher -> eval suite).
- Link the current launch assets: monitored launcher
  (`scripts/launch-toaster-monitored.ps1`), portable mode
  (`is_portable` command at `src-tauri/src/commands/mod.rs:26-30`),
  eval scripts under `scripts/eval/`, the local-LLM gate, and the
  build docs (`docs/build.md`).
- Point contributors at `CONTRIBUTING.md` and the launch roadmap at
  `features/product-map-v1/PRD.md` (Milestone 3, Section 6).
- Be honest about cross-platform support: Windows primary,
  macOS/Linux community-supported pending their respective
  `-build-verify` bundles.

This feature is the `readme-launch-pass` bundle called out in
`features/product-map-v1/PRD.md:495` (Milestone 3, row 3.4) and is
**documentation-only** — it produces planning artifacts only in this
pass, and a README patch in the follow-up execution pass.

## 2. Desired Outcome & Acceptance Criteria

See PRD.md. Summary:

- README tagline reads as transcript-first editor in the first three
  lines.
- Quickstart links `docs/build.md` and
  `scripts/launch-toaster-monitored.ps1`.
- An Evals section links `scripts/eval/` and explains what the evals
  cover.
- A Roadmap section links `features/product-map-v1/PRD.md`.
- "Handy" appears at most once in the README, only inside a fork-
  acknowledgment footer paragraph.
- All internal relative links resolve to real files.
- Only Toaster-owned badges (if any) — no badges pointing at the
  upstream Handy repo.

## 3. Scope Boundaries

### In scope

- A plan for rewriting `README.md` end-to-end to match the structure
  below (hero tagline, screenshot placeholder, features, quickstart,
  build, launch, evals, roadmap, contributing, license, fork-ack).
- Tightly-scoped companion doc edits only if needed for a link
  target. No new top-level docs in this pass.
- Screenshot **placeholders** (e.g.
  `[TODO: screenshot of export dialog]`) in the sections that will
  need art in a follow-up.

### Out of scope (explicit)

- Any production-code change (Rust, TS, Tauri config).
- New screenshots or video captures.
- New i18n keys. README is English-only by policy; the `i18n-pruning`
  skill applies to in-app UI strings, not repo docs.
- Inlining the Milestone 3 table. The README links to
  `features/product-map-v1/PRD.md` instead.
- Signing / installer / release-notes work (handled by other
  Milestone 3 bundles: `windows-installer-sign`, etc.).

## 4. References to Existing Code

- `README.md:1-96` — current content; line 5 is the one Handy mention
  that will be moved into a fork-ack footer, lines 83-87 ("Current
  launch focus") are the section that will be replaced by Roadmap.
- `docs/build.md:1-40` — canonical build prerequisites and Windows
  setup. README quickstart must point here, not duplicate.
- `scripts/launch-toaster-monitored.ps1` — monitored dev launcher;
  referenced by AGENTS.md Critical Rules and
  `README.md:56`.
- `src-tauri/src/commands/mod.rs:26-30` — `is_portable` command
  backing portable mode; currently undocumented in README per
  `features/product-map-v1/PRD.md:165`.
- `scripts/eval/` — 12 eval scripts covering edit quality, audio
  boundary, caption parity, multi-backend parity, disfluency /
  cleanup / captions verifiers, local-LLM gate, fixture generators.
- `CONTRIBUTING.md:1-30` — referenced from README; keeps setup
  instructions canonical there, README only links.
- `features/product-map-v1/PRD.md:486-503` — Milestone 3 goals and
  Section 6 roadmap context.
- `AGENTS.md:37-45` — "Critical rules" (local-only inference,
  monitored launcher) that the README positioning must stay
  consistent with.

## 5. Edge Cases & Constraints

- Documentation-only: no `.rs`, `.ts`, `.tsx`, `tauri.conf.json`,
  `Cargo.toml`, or `package.json` changes.
- No new i18n keys. `bun scripts/check-translations.ts` must remain
  green trivially.
- Structure follows the common README template: hero tagline,
  screenshot placeholder, features bullets, quickstart, build,
  launch, evals, roadmap, contributing, license, fork-ack.
- All internal `[text](relative-path)` links resolve to files that
  either exist today or are explicitly planned in
  `features/product-map-v1/PRD.md`. No dead links.
- The string "Handy" appears only inside the fork-acknowledgment
  footer paragraph — nowhere in feature bullets, quickstart, or
  tagline.
- Badges: keep only Toaster-related ones (CI on
  `itsnotaboutthecell/toaster`, license). Remove anything pointing
  at the upstream Handy repo.

## 6. Data Model (optional)

N/A — documentation bundle.

## Q&A

The seed request pre-answered the usual Phase 5 questions. Recorded
here verbatim so future readers can trace the decisions:

- **Target audience ordering?**
  End users evaluating Toaster first, contributors second, Handy-
  forkers third. README structure follows that ordering.

- **Screenshot strategy?**
  Existing screenshots stay; new ones are follow-up work. BLUEPRINT
  flags the sections that need a `[TODO: screenshot ...]` placeholder.

- **Roadmap disclosure?**
  Link to `features/product-map-v1/PRD.md` for the launch roadmap.
  Do not inline the milestone table.

- **Cross-platform framing?**
  Windows is the primary supported platform today. macOS and Linux
  are community-supported pending the `mac-build-verify` /
  `linux-build-verify` bundles (see
  `features/product-map-v1/PRD.md:491-494`). README must say this
  honestly.

- **Does the README still mention Handy?**
  Yes, at `README.md:5`. That sentence moves into a dedicated fork-
  acknowledgment footer paragraph in the rewrite.