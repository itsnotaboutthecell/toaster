# Silero VAD — live QC evidence (R-002 prefilter)

Recorded 2026-04-19 during Phase 5 QC on `feat/vad-runtime-delta-r002`.

## Why "And uh" → "And" after enabling the prefilter

The user observed a splice that previously bled "And uh" now plays back
as a clean "And". Two independent mechanisms explain this — both
visible in the launch log `launch-20260419-203520.stdout.log`:

1. **Prefilter skipped the disfluency.** Log line:
   `VAD prefilter: 5 window(s) covering 14620000/21717312 µs (67.3% of
   buffer)`.
   Parakeet was only invoked on the 67.3 % of the buffer that Silero
   classified as speech. The remaining 32.7 % (inter-utterance silence
   + low-energy filler onsets that failed to trigger the default
   `onset_frames = 2` threshold) was never transcribed. A short "uh"
   whose onset is below the threshold simply never reaches the ASR.
2. **Boundary refinement snapped the splice.** Log line:
   `VAD boundary refinement: computed 723 frame probabilities`.
   The playback cut uses the per-frame P(speech) curve to snap into
   the deepest silence valley rather than the pre-feature
   zero-crossing + 20 ms energy-valley heuristic. Even if a fragment
   like "uh" were transcribed, the cut would now land beyond its
   acoustic tail.

## Regression framing

This is content-aware noise suppression, not magic. A very short real
word (e.g. "um" pronounced as a standalone discourse marker vs. "I'm")
could, in principle, be skipped by the same mechanism.

- Regression gate: `transcript-precision-eval`. Re-run before opening
  the PR and cite the word-count delta against the baseline fixture
  in the PR body.
- Graceful absence: AC-005-c — when the Silero ONNX is not installed,
  `TranscriptionManager::transcribe()` falls back to the full-file
  engine path unmodified. Prefilter and boundary-refine are both
  no-ops when the model is missing.

## Live evidence pointers

- Prefilter fires: `.launch-monitor/launch-20260419-203520.stdout.log`
  — grep `VAD prefilter:`.
- Boundary-refine fires: same log — grep
  `VAD boundary refinement:`.
- Binary wiring gate: `pwsh scripts/eval/eval-vad.ps1` → G9
  `prefilter_live_wired` must remain PASS.
