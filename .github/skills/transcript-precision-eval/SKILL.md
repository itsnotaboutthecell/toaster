---
name: transcript-precision-eval
description: 'Use when adding or modifying word operations, keep-segment logic, time mapping, export, or transcription segment post-processing. Extends or runs the fixture-based precision eval that enforces the PRD acceptance criteria (per-word timing preserved, no equal-duration synthesis, midstream deletions stay clean).'
---

# Transcript Precision Eval

## Overview

The PRD requires precise transcript-driven editing with per-word timing preserved, no synthetic equal-duration timestamps, and clean midstream-deletion replay (including delete/undo cycles). These criteria are repeated across `AGENTS.md`, `CLAUDE.md`, and `.github/copilot-instructions.md` — yet currently there is **no automated eval** that fails a PR when any of them is violated.

**Core principle:** The acceptance gate stated in guardrails must be machine-enforced, not agent-enforced.

## What the Eval Must Cover

1. **Per-word timing preservation** — given a fixture media file, the transcription pipeline must produce per-word start/end timestamps that match the golden JSON within a small tolerance. Drifts caused by introducing equal-duration logic must fail the eval.
2. **Keep-segment arithmetic** — deleting a middle segment and computing source → edit time mapping must round-trip correctly for a set of sampled points.
3. **Midstream replay cleanliness** — after deletion, the rendered preview audio at the cut point contains no material from the deleted region (checked via a short silence window at the splice and absence of deleted-word phonemes in a re-transcription sanity pass).
4. **Undo round-trip** — delete → undo must restore the original timeline byte-for-byte in the serialized project state.
5. **Export parity** — `scripts/eval-edit-quality.ps1` output compared against a stored baseline for duration, silence gaps, and leading/trailing silence.

## Fixture Assets

- `extras/toaster_example.mp4` (original) — already in the repo.
- `extras/toaster_example-edited.mp4` (edited baseline) — already in the repo.
- `tests/fixtures/toaster_example.words.golden.json` — **to be created**. Produced from a reference transcription run that is manually verified and then frozen.

## Gate Function

**Adding a new word operation or timeline transform:**

```
1. Read the existing golden JSON. Confirm your change does not need to
   regenerate it (if it does, regenerating is a plan deviation and needs
   explicit approval — see receiving-code-review skill).
2. Extend the eval to cover the new operation with at least one case.
3. Run the eval. Watch it fail against the golden for the new behavior.
4. Implement the change.
5. Run the eval. Watch it pass.
```

**Modifying transcription segment post-processing:**

```
1. Regeneration of the golden JSON is a plan deviation. Stop and confirm.
2. If confirmed: capture the new golden, diff it against the old, document
   every changed word in the PR description with justification.
```

**Modifying export:**

```
1. Run scripts/eval-edit-quality.ps1 -Original extras/toaster_example.mp4 \
     -Edited <path/to/your/export.mp4> -OutputJson eval-before.json
2. Apply your change.
3. Re-run, outputting eval-after.json.
4. Diff. Justify every regression; no silent drift accepted.
```

## Red Flags — STOP

- Changing per-word timing logic without regenerating or validating the golden
- Introducing any code path that synthesizes equal-duration timestamps
- Adding logic to the frontend that independently computes keep-segments or cut points
- Swapping the video element source to an audio preview file (explicitly banned)
- Claiming a fix is complete because unit tests pass, without running the acceptance gate

## Wiring to CI

When this skill graduates from "manual" to "gated", the following are expected to run on every PR touching backend timeline or export code:

- `cargo test precision_eval -- --nocapture`
- `pwsh scripts/eval-edit-quality.ps1` against the fixture, compared to baseline
- `pwsh scripts/run-live-midstream-validation.ps1` in headless mode

Track progress under the `eval-precision-fixture`, `eval-midstream-ci`, and `eval-export-parity` todos.

## When To Apply

- Any change to `managers/editor.rs`, `managers/transcription.rs`, `managers/export.rs`
- Any change to `commands/waveform.rs`, `commands/transcribe_file.rs`, `commands/editor.rs`, `commands/export.rs`
- Any change to frontend playback logic in `components/player/` or `components/editor/`
- Any change to word operations, time mapping, or serialization
