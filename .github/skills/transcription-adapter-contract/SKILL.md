---
name: transcription-adapter-contract
description: 'Use before merging any PR that adds or swaps an ASR or forced-alignment backend. Enforces that the backend produces a NormalizedTranscriptionResult matching the canonical schema, that word-timing invariants hold, and that a round-trip fixture test proves editor + export still satisfy the precision gates with the new backend.'
---

# Transcription Adapter Contract

## Overview

Toaster's editor authority depends on per-word timestamps being **real, monotonic, and non-synthesized**. Swapping an ASR or forced-alignment backend silently breaks this when the new backend emits synthesized equal-duration timestamps, overlaps between words, zero-duration tokens, or embedded non-speech markers. Every such regression cascades into keep-segment math, seam boundaries, and captions — usually discovered only after export.

**Core principle:** A transcription backend is a supplier with a contract. If it cannot meet the contract, it does not ship — regardless of WER, latency, or language support.

## Canonical Schema

Every backend adapter must emit a `NormalizedTranscriptionResult`. Naming here is the contract; field names are authoritative even if the internal representation differs.

```rust
struct NormalizedTranscriptionResult {
    words: Vec<CanonicalWord>,
    language: String,                      // BCP-47, e.g. "en-US"
    word_timestamps_authoritative: bool,   // true iff the backend itself measured them
    input_sample_rate_hz: u32,             // the sample rate the backend ingested
}

struct CanonicalWord {
    text: String,                // post-normalization, no non-speech tokens
    start_us: u64,               // microseconds from start of input audio
    end_us: u64,                 // microseconds; end_us > start_us
    confidence: Option<f32>,     // 0.0..=1.0 if available, else None
}
```

## Invariants (hard gates)

1. **Monotonic non-overlap.** For all `i`, `words[i].end_us <= words[i+1].start_us`. Gaps are allowed (and expected — silence, breaths).
2. **No zero-duration words.** `end_us > start_us` for every word. Zero-duration tokens fail the gate.
3. **Non-speech tokens stripped.** No `[MUSIC]`, `<silence>`, `...`, `(inaudible)`, `♪`, or backend-specific sentinels leak into `text`. They are removed before emitting the struct.
4. **No equal-duration synthesis.** If `word_timestamps_authoritative == false`, the adapter must **not** spread tokens at equal spacing across an utterance. Either carry through the backend's true per-word timing, or document explicitly in the PR why a specific, bounded fallback is acceptable and gate the flag behind a named config. Silent synthesis is a rejection.
5. **Sample-rate truth.** `input_sample_rate_hz` reflects what the backend actually consumed; do not claim 48 kHz if the adapter internally downsampled to 16 kHz.
6. **Language reported.** `language` is populated, even if hardcoded to the user-selected locale.

## Gate Function

**Adding or swapping a backend:**

```
1. Implement the adapter to emit NormalizedTranscriptionResult. Reject at the
   adapter boundary anything that violates invariants 1–6 — do NOT paper over
   it downstream.
2. Write an adapter unit test that feeds a known fixture and asserts each
   invariant explicitly. Red-green the test before any integration wiring.
3. Run the round-trip fixture test (see below). Precision + boundary gates
   must still pass with the new backend selected.
4. Declare in the PR: WER delta on the fixture, timing-delta histogram
   (p50 / p95 word-start drift vs golden), and whether
   word_timestamps_authoritative is true. If false, justify.
```

**Round-trip fixture test:**

```
1. Transcribe eval/fixtures/toaster_example.mp4 with the new backend.
2. Feed the resulting NormalizedTranscriptionResult into the editor; perform
   the canonical midstream-deletion sequence from the precision eval.
3. Export. Run:
   - transcript-precision-eval gates (per-word timing, keep-segment math,
     undo round-trip, export parity)
   - audio-boundary-eval gates (xcorr < 0.15, z-score < 4.0, preview↔export
     parity within 1 sample)
   - multi-backend-parity gates (below) — required whenever an adapter
     ships, changes, or flips word_timestamps_authoritative
4. All gates green = adapter acceptable. Any red = adapter rejected.
```

**Multi-backend parity eval (invoke on every adapter change):**

```powershell
# Full runner — JSON + markdown under eval/output/multi-backend-parity/<ts>/
pwsh -NoProfile -File scripts/eval-multi-backend-parity.ps1

# One fixture only
pwsh -NoProfile -File scripts/eval-multi-backend-parity.ps1 -Fixture phrase_alpha

# Negative tests — prove the gates fire on known-bad adapter shapes
pwsh -NoProfile -File scripts/eval-multi-backend-parity.ps1 -ForceRegression equal-duration
pwsh -NoProfile -File scripts/eval-multi-backend-parity.ps1 -ForceRegression pre-speech-padding
pwsh -NoProfile -File scripts/eval-multi-backend-parity.ps1 -ForceRegression authoritative-lie

# CI mode — missing backend outputs promote from skip to fail
pwsh -NoProfile -File scripts/eval-multi-backend-parity.ps1 -StrictMode

# Rust-side CI entry (same thresholds, cargo-native)
cd src-tauri && cargo test --test precision_eval_multi_backend
```

Gates enforced (mirror AGENTS.md precision guardrails):

| # | Name                         | Threshold                              |
| - | ---------------------------- | -------------------------------------- |
| G1 | Median boundary error vs oracle | `<= 20 000 us` per backend          |
| G2 | p95 boundary error vs oracle    | `<= 40 000 us` per backend          |
| G3 | Cross-backend seam count parity | `seams_a == seams_b` on same edit   |
| G4 | Cross-backend duration delta    | `<= 20 000 us` on same edit         |

Failure modes these catch:

- Whisper char-split synthesis regression → G2 blows past 40 ms.
- Parakeet pre-speech padding leakage (outer-trim gating broken) → G1/G2 fail.
- An engine declaring `word_timestamps_authoritative = true` incorrectly
  → its p95 blows up vs the independent oracle, G2 fails.
- A backend split-joins silently (e.g. merges two adjacent words into one
  token) → G3 fails because the seam count on the same delete diverges.

Oracle policy (see `src-tauri/tests/fixtures/parity/*.oracle.meta.json`):

- Synthetic fixtures: analytical ground truth from FFmpeg synthesis spec
  (strictly tighter than any forced aligner).
- Real speech fixtures: forced-alignment via Gentle (MIT) or whisper.cpp
  `--max-len 1` (MIT, once authoritative-timings lands) or MFA (MIT).
  Document the oracle source + version + invocation in the per-fixture
  `oracle.meta.json`.

Backend outputs live at
`src-tauri/tests/fixtures/parity/backend_outputs/<backend>/<fixture>.result.json`.
A backend adapter change must re-cache these from a real adapter run
before the gate is meaningful. Fixtures without a backend result are
logged as `skip`; in `-StrictMode` skip promotes to fail (CI behavior).

Regenerate fixtures: `pwsh -NoProfile -File scripts/generate-parity-fixtures.ps1`.


## Red Flags — STOP

- Adapter emits `start_us == end_us` anywhere and the PR says "we filter that later"
- Token list contains `[MUSIC]` / `<eot>` / `(laughter)` in `text`
- `word_timestamps_authoritative = true` but word timings are in fact character-count proportional
- Integration-level post-processing attempts to "fix" overlapping words by mutating `end_us` — the fix belongs in the adapter, not downstream
- PR swaps the backend but only runs WER; never ran the precision or boundary evals
- Adapter adds a new dependency without going through `dep-hygiene`

## Relationship to other skills

- `transcript-precision-eval` — the round-trip test above runs it; this skill assumes its gates exist.
- `audio-boundary-eval` — a backend that jitters word ends by even a few milliseconds can push seams into click territory; both must pass.
- `dep-hygiene` — new backends usually come with new crates; justify them.
- `multi-backend-parity` (runner `scripts/eval-multi-backend-parity.ps1` +
  `src-tauri/tests/precision_eval_multi_backend.rs`) is owned by this skill;
  invoke it whenever an adapter changes.

## When To Apply

- Any PR adding a new ASR or forced-alignment crate / model / service
- Any PR changing the adapter glue between a backend and `managers/transcription`
- Any PR changing normalization (tokenizer, filler-word filter, punctuation restoration) that sits between the backend and `CanonicalWord`
- Any PR that flips `word_timestamps_authoritative` for an existing backend
