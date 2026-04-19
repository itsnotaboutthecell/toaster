# Feature request: Scriptable video edit assessment

> **Exploratory only.** Per the user's framing - "I want you to be aware of and to start assessing" / "what are some of the considerations we should start thinking about" - this bundle is intentionally not a plannable feature. There is **no PRD, no BLUEPRINT, no coverage gate, no tasks.sql**. The substantive output lives in `ASSESSMENT.md`. This bundle stays at `STATE.md = defined` until the user picks an architecture direction; at that point it spawns one or more real PM features.

## 1. Problem & Goals

User feedback:

> "This is a bigger project, but I want you to be aware of and to start assessing, we want to be able to edit videos via scripts, we have the export ffmpeg - what are some of the considerations we should start thinking about to ensure we could use this workflow in the future and dynamically edit videos based on properties (ex. today I'm doing a desktop video, tomorrow I'm doing a mobile video - caption positions would be different; or maybe changing brand colors of the boxes, etc. etc.)"

The user wants Toaster to be programmable in addition to GUI-driven: an "edit profile" that captures orientation, caption position, brand colors, filler words, output target etc., applied to a source video without manually clicking through the editor each time.

**Goal of this assessment:** map the surface area, name the risks, propose three candidate architectures, and identify a no-regrets first step that any of them would build on. Do **not** prescribe an architecture - that is the user's call.

## 2. Desired Outcome & Acceptance Criteria

This is exploratory. There are no acceptance criteria; the deliverable is `ASSESSMENT.md` and the user's subsequent direction-setting decision.

## 3. Scope Boundaries

### In scope (this assessment)

- Sketch the data model for a "video edit profile".
- Map the existing pipeline touchpoints (export, project, captions, FFmpeg invocation).
- Enumerate the risks - especially the AGENTS.md "Local-only inference" tripwire.
- Three candidate architectures with trade-offs.
- A wedge first step that is valuable regardless of architecture chosen.

### Out of scope (this assessment)

- Writing PRDs / BLUEPRINTs.
- Choosing an architecture.
- Any production code changes.
- Any commitment to a particular config format / scripting language.

## 4. References to Existing Code

(See `ASSESSMENT.md` for full citations - this section is intentionally short.)

- `src-tauri/src/commands/export.rs` - export command boundary.
- `src-tauri/src/managers/splice/` - audio splice + boundaries pipeline (no `export/` manager exists today; export logic lives across commands + splice).
- `src-tauri/src/managers/captions/` - caption-layout authority (single-source-of-truth target).
- `src-tauri/src/managers/editor/` - keep-segment / time-mapping authority.
- `AGENTS.md` "Local-only inference", "Single source of truth for dual-path logic".

## 5. Edge Cases & Constraints

- Toaster must remain local-only. Any "scripted edit" capability that even tempts a future contributor to call out to a hosted LLM ("auto-pick the best brand color") is forbidden by AGENTS.md.
- Profiles must respect the SSOT rule: a profile encodes user *intent*, the backend remains the single source of truth for layout / boundaries / time mapping.
- Scripted runs must reuse the existing eval harness fixtures (`eval/fixtures/`) to stay regression-tested.

## 6. Data Model (sketch)

See `ASSESSMENT.md` Section 1.

## Q&A

(Outstanding question for the user before any of this becomes a real feature: which of the three candidate architectures in `ASSESSMENT.md` Section 5 do you want to pursue, and in what order? Until that is answered, no further PM work happens here.)
