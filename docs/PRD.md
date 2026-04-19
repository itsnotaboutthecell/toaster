# Toaster PRD

## 1. Product vision

Toaster is a local-first transcript editor for spoken media:

> Open media -> transcribe -> edit text -> preview -> export.

Primary launch objective: deliver a reliable, precise, and understandable transcript-driven editing workflow on the current **Tauri + Rust + React** stack.

## 2. Product principles

1. **Local-first by default** for core transcription/edit/export workflows
2. **Non-destructive editing model** with undo/redo and reversible actions
3. **Backend timeline authority** for keep-segments and time mapping
4. **Transcript-first UX** where word operations are the primary editing surface
5. **Launch reliability over feature sprawl**

## 3. Target users

- Creators editing talking-head/tutorial/interview content
- Podcasters and educators cleaning speech-heavy audio/video
- Users who prefer offline-capable tooling

## 4. In-scope architecture

| Layer | Implementation |
|---|---|
| Desktop app | Tauri 2.x |
| Backend | Rust managers + Tauri commands |
| Frontend | React + TypeScript + Tailwind |
| State | Zustand stores |
| Export/transforms | FFmpeg-driven backend paths |

Out of scope for launch documentation: host-integration tracks outside the Tauri desktop app.

## 5. Core capabilities (launch baseline)

1. Open media and transcribe to word-level timeline
2. Word operations: delete, silence, restore, split
3. Keep-segment playback mapping for edited preview
4. Waveform/transcript/playhead synchronization
5. Project save/load
6. Export edited media + captions + script
7. Deterministic filler/pause detection workflows

## 6. Current known risks

1. Documentation drift from Handy-era content causes contributor confusion
2. Precision regressions can appear in midstream deletion playback if timestamp/mapping invariants drift
3. UX readability regressions in detect/highlight states on dark theme
4. Windows setup failures when env/toolchain guardrails are skipped

## 7. Launch readiness workstreams

### WS1: Playback precision and timeline correctness
- Maintain backend-authoritative keep-segment mapping
- Validate long-form and midstream deletion scenarios
- Keep preview and export boundaries aligned

### WS2: Editor UX reliability
- Ensure detect actions target only highlighted words
- Keep destructive shortcuts scoped to active selection/highlight sets
- Preserve readability in dark-theme highlight states

### WS3: Documentation and contributor onboarding
- Align README/BUILD/CONTRIBUTING/templates with current stack
- Remove stale Handy-only and conflicting architecture guidance
- Provide a clean first-run path for new contributors

### WS4: Agent and skill alignment
- Keep AGENTS/CLAUDE/Copilot instructions synchronized with real architecture
- Keep build/lint/test command reference in AGENTS.md and `docs/build.md` current

## 8. Milestones (sequential + parallel)

### Milestone A (sequential gate)
- Lock architecture narrative to Tauri-first launch docs
- Publish corrected README + PRD + BUILD baseline

### Milestone B (parallel)
- Refresh CONTRIBUTING + PR/issue templates
- Refresh AGENTS + CLAUDE + Copilot instructions
- Audit AGENTS.md Development commands + Windows requirements sections and `docs/build.md`

### Milestone C (sequential closeout)
- Run launch-doc consistency pass
- Run new-contributor dry-run from docs only
- Publish remaining blockers and owners

## 9. Acceptance criteria for public-launch readiness

1. No top-level docs describe a conflicting architecture
2. Contributor setup path is reproducible from documentation alone
3. Agent instructions and skills match repository reality
4. Precision/timeline guardrails are explicit in instructions and enforced in review
5. Public-facing project identity is fully Toaster-branded and coherent
