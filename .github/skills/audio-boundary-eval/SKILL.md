---
name: audio-boundary-eval
description: 'Use on any PR that modifies managers/editor, commands/waveform, export splice logic, preview audio rendering, or boundary snapping. Extends transcript-precision-eval with seam-specific gates: deleted audio must not leak across the splice, seam windows must stay click-free, and preview must use the identical boundary policy as export.'
---

# Audio Boundary Eval

## Overview

`transcript-precision-eval` enforces timing correctness (per-word timestamps, keep-segment arithmetic, duration parity). It does **not** listen to the splice. Every regression in this area — "tiny remnant of the deleted word", "click at the cut", "preview sounds clean but export does not" — has slipped through duration-and-silence metrics because those metrics are blind to what happens in the first ~80 ms after a seam.

**Core principle:** Duration parity is necessary but not sufficient. A splice that preserves duration can still leak the tail of a deleted phoneme or introduce a zero-crossing discontinuity. The boundary itself is an acceptance surface and must be measured at sample resolution.

## What the Eval Must Cover

1. **No cross-seam leakage.** For every cut, compute the normalized cross-correlation between
   - the deleted word's tail (the last 80 ms of the removed audio, mono, 48 kHz), and
   - the 0–80 ms window **after** the seam in the rendered output.
   Fail if `|xcorr_peak| >= 0.15` in that window. A clean splice sits well under 0.05.

2. **Seam is click-free.** In a ±10 ms window around each seam, compute a spectral-discontinuity z-score against the surrounding 200 ms baseline (HF-band energy jump + sample-level first-difference spike). Fail if `z >= 4.0` at any seam. Clicks almost always show `z >> 8`.

3. **Preview ↔ export parity.** Render the same edit via the preview path (audio routed from the frontend player) and the export path (FFmpeg splice). For each seam, compare seam-center offsets and first-200-ms PCM. Fail if seam offsets differ by more than **1 sample** at 48 kHz, or if the PCM L2 delta exceeds **-40 dBFS RMS**. Both paths must come from the **same backend-authored boundary policy** (single source of truth — see AGENTS.md "dual-path logic" rule).

4. **No silent-pass-with-artifact.** If duration/silence metrics pass but any of (1)–(3) fail, the overall gate fails. Record explicitly in the report which class failed.

## Runner

Harness: **`scripts/eval-audio-boundary.ps1`** (owner: `p2-eval-bundle`).

```powershell
# Run every fixture (CI-ready; non-zero exit on any failing gate).
pwsh -NoProfile -File scripts/eval-audio-boundary.ps1

# One fixture only.
pwsh -NoProfile -File scripts/eval-audio-boundary.ps1 -Fixture phrase_01

# Negative test — swaps phrase_01_edited_leaky.wav in to prove gates fire.
pwsh -NoProfile -File scripts/eval-audio-boundary.ps1 -Fixture phrase_01 -ForceLeaky
```

Reports land at `eval/output/audio-boundary/<fixture>/<timestamp>/{report.json,report.md}`.

Helper library: `scripts/lib/AudioBoundary.psm1`
(normalized cross-correlation, spectral-discontinuity z-score, WER, sample-boundary).

Fixture generator: `scripts/generate-boundary-fixtures.ps1`
(deterministic synthetic tones — no TTS dep, no licensed audio).

### Gates implemented

| # | Name                     | Metric                                   | Threshold       |
| - | ------------------------ | ---------------------------------------- | --------------- |
| E1 | `E1_leak_xcorr`         | Normalized xcorr(stem, post-seam 80 ms) | `< 0.15`        |
| E2 | `E2_seam_zscore`        | Max(diff-zscore, HF-band zscore)         | `< 4.0`         |
| E3 | `E3_preview_parity`     | xcorr(preview, export) and RMS delta     | `> 0.995` / `≤ -40 dBFS` |
| E4 | `E4_transcript_wer`     | WER(expected, re-transcribed)            | `≤ 0.05`        |
| E5 | `E5_sample_boundary`    | Per-boundary + total sample error        | `≤ 1 sample`    |

### Known STUB / partial coverage

- **E4** currently feeds a baked `hypothesis_clean` / `hypothesis_leaky` list
  from the fixture JSON instead of invoking the full whisper re-transcribe
  pipeline. The WER algorithm is real and enforces the threshold; wiring
  into the live re-transcribe path is tracked for the eval-harness-runner
  (parameterize once the export pipeline is reachable from this script).
- **E3** requires a ≥3-seam project — satisfied by the `multicut_01`
  fixture. The `phrase_01` fixture has only 2 seams (one natural source
  boundary + one true splice); E3 runs against it anyway with a
  `fewer than 3 seams` note to keep coverage honest.
- Fixtures are synthetic tones, not TTS speech. Rationale: reproducibility,
  no licensing encumbrance, and harsher xcorr signal than speech. Speech
  fixtures can be dropped into `src-tauri/tests/fixtures/boundary/` and
  added to the generator without changing the harness.

## Fixture Assets

- `eval/fixtures/toaster_example.mp4` and `eval/fixtures/toaster_example-edited.mp4` as source material.
- `src-tauri/tests/fixtures/boundary/phrase_01.*` — synthetic 3-tone phrase with manifest, stems, clean edit, leaky edit, preview parity target, expected-transcript JSON.
- `src-tauri/tests/fixtures/boundary/multicut_01.*` — 4-tone edge-to-edge fixture for ≥3-seam preview↔export parity (E3).
- Regenerate on demand: `pwsh -NoProfile -File scripts/generate-boundary-fixtures.ps1`.

## Gate Function

**Modifying boundary snapping / splice logic:**

```
1. Regenerate seam PCM for each fixture via preview and export paths.
2. Run boundary eval: xcorr, spectral-discontinuity z-score, preview↔export diff.
3. Every seam must be classified "clean". One "leak" or "click" fails the PR.
4. Compare against the prior baseline report; justify any metric that got worse,
   even if still under threshold.
```

**Modifying preview audio rendering:**

```
1. Confirm the boundary policy is still read from the backend, not re-derived in JS.
2. Run the preview↔export parity check; the 1-sample / -40 dBFS bound is hard.
```

## Red Flags — STOP

- Duration/silence metrics pass but seams were never listened to or measured
- Preview audio path re-implements keep-segment math independently of the backend
- A "fix" that lowers xcorr threshold or widens the z-score bound instead of fixing the seam
- Any code path that crossfades or fades the seam to hide a leak instead of cutting cleanly
- Claiming the fix works because unit tests pass, without seam-level numbers

## Relationship to `transcript-precision-eval`

This skill **extends** `transcript-precision-eval`; it does not replace it. The precision eval asserts "the cut happened at the right timestamp". This eval asserts "the cut sounds right at sample resolution". A PR touching the splice must pass both.

## When To Apply

- Any change to `src-tauri/src/managers/editor*` splice or boundary logic
- Any change to `src-tauri/src/commands/waveform*`
- Any change to export splice in `managers/export*` or FFmpeg filter-graph construction
- Any change to preview audio rendering in `src/components/player/` or `src/components/editor/`
- Any change to boundary snapping (zero-crossing, silence-pad, fade-in/out) policy
