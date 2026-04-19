# Feature request: Product map to v1 launch

## 1. Problem & Goals

User-verbatim:

> Produce a structured assessment of where the project stands today, what
> capabilities exist (documented and undocumented), what gaps stand
> between us and a credible v1.0 launch, and a prioritized roadmap of
> features — including FFmpeg-backed capabilities that fit Toaster's
> "simplicity first, but don't leave great features off the table"
> philosophy.

The repository has shipped four feature bundles in the last cycle
(`brand-title-sizing`, `caption-settings-preview`,
`remove-history-and-legacy` — all in `reviewing` —
`build-env-ninja-hardening` in `planned`). The transcript-first editor
core (open → transcribe → edit → preview → export) works end-to-end on
a single Whisper backend. What is missing is a coherent picture of
**what is actually shipped versus partial versus orphaned**, **what a
credible v1.0 needs that we don't have**, and **which FFmpeg
capabilities we should adopt without violating the simplicity rule.**

## 2. Desired Outcome & Acceptance Criteria

A discovery bundle at `features/product-map-v1/` whose `PRD.md` is the
deliverable. ACs gate that each required PRD section exists and is
substantive. See PRD §AC list.

## 3. Scope Boundaries

### In scope

- Audit current capabilities (backend + frontend + scripts/evals)
- Identify undocumented / under-documented surfaces
- Gap analysis to v1.0 launch
- FFmpeg opportunity map evaluating ≥12 candidate capabilities
- 3-milestone roadmap (Foundation / Polish / Launch-Ready)
- Anti-scope list with AGENTS.md citations
- Open questions for the human

### Out of scope (explicit)

- Production code edits (this is a planning artifact)
- Re-proposing in-flight features (the four bundles already on the
  board)
- Hosted-inference dependencies (forbidden by AGENTS.md)
- New feature `<slug>/` bundles — those will be scaffolded later, one
  per roadmap item

## 4. References to Existing Code

- `AGENTS.md:83-91` — non-negotiable boundaries (single source of
  truth, local-only inference)
- `AGENTS.md:240-283` — spec-driven lifecycle and per-feature artifacts
- `PRD.md:1-98` — original product vision and launch readiness
  workstreams
- `src-tauri/src/commands/waveform/mod.rs:562-687` — current FFmpeg
  export pipeline (concat / atrim / afade / volume / loudnorm /
  subtitles burn-in)
- `src-tauri/src/managers/splice/loudness.rs:7,40` — EBU R128
  measurement infra ready but not wired to export gating
- `src-tauri/src/managers/captions/mod.rs:12` — single-source caption
  layout authority consumed by preview + export
- `src-tauri/src/managers/transcription/adapter.rs` — multi-backend
  ASR adapter trait (7 engines)

## 5. Edge Cases & Constraints

- Coverage gate (`scripts/feature/check-feature-coverage.ps1`) requires every
  `AC-NNN-x` to map to a real verifier; planning artifacts must use
  `manual` verifiers with concrete `steps` arrays.
- Must not add an AGENTS.md duplicate or drift; cite, do not copy.
- Must not propose anything that requires a network call at runtime.

## 6. Data Model (optional)

N/A — this bundle produces only Markdown and JSON planning artifacts.

## Q&A

Phase 5 skipped by user request — the deliverable is a discovery
report, not a normal feature, and the user supplied the full eight-section
brief in the original prompt. No clarifying questions were needed
because (a) scope is fully specified, (b) there is no implementation,
(c) the human will resolve open questions in PRD §8 before any
roadmap item becomes a real bundle.
