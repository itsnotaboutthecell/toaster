# Feature request: reintroduce Silero VAD (file-based, editor use cases)

## 1. Who & what

Who: Toaster maintainers / transcript-editor users.

What: Reintroduce Silero VAD into Toaster as a **file-based analyzer**
(not a microphone endpointer) to serve three narrow editor use cases:

1. ASR silence pre-filter — run Silero over an input media file
   before any `transcribe-rs` pass and feed only speech windows (with
   pre-roll / hangover padding) to the ASR. Goals: reduce
   transcription wall-time on silence-heavy files and eliminate
   Whisper hallucinations on silent / music regions.
2. Splice-boundary refinement — extend
   `src-tauri/src/managers/splice/boundaries.rs` so the existing
   zero-crossing (5 ms) + energy-valley (20 ms) snap can consult a
   Silero P(speech) curve in a ±100 ms window and prefer the local
   minimum of P(speech) that is still within the existing radii.
3. Filler / pause semantics — in `src-tauri/src/managers/filler.rs`,
   when a gap between adjacent word timestamps exceeds
   `DEFAULT_PAUSE_THRESHOLD_US` (1.5 s), run a Silero pass over the
   gap and classify it as true silence / non-speech acoustic /
   ASR-missed speech. Additive metadata only — auto-delete defaults
   unchanged.

## 2. Why

Research report
`C:\Users\alexm\.copilot\session-state\12c1b358-6581-40c1-87ae-4516a84e344b\research\https-github-com-snakers4-silero-vad-how-would-sil.md`
documents three Toaster-specific failure modes that Silero can
measurably improve:

- Whisper hallucinates on silent / music regions; the
  `transcript-precision-eval` skill already guards against this
  failure mode but has no input-side filter to offer.
- `splice/boundaries.rs` docstring (line 26 in the current tree)
  explicitly calls out the risk that the zero-crossing / energy-valley
  radii can "leak into an adjacent syllable". A P(speech) curve would
  make that tradeoff adaptive rather than fixed.
- `filler.rs:32` defines a pause as a gap between ASR word timestamps
  only; there is no acoustic signal distinguishing silence from
  breath / music / dropped ASR.

Handy — Toaster's fork parent — already solved the neural VAD wrapper
and hysteresis problem. Handy's `silero.rs` and `smoothed.rs` are
directly reusable; the hard work is the three **new** callers and the
model-distribution plumbing, not the VAD itself.

## 3. Constraints (non-negotiable)

Sourced from `AGENTS.md` and the invoked skills:

- **Local-only inference.** Silero ONNX runs in-process via the same
  `ort` / `onnxruntime` stack `transcribe-rs` already uses. No hosted
  API anywhere in the pipeline.
- **Backend owns logic, frontend renders state.** Any new UI toggle
  only flips a backend-owned config field.
- **Single source of truth for dual-path logic.** Use case 2 edits
  `splice/boundaries.rs`, which is consumed verbatim by both preview
  and export. Preview and export must produce identical boundaries.
- **800-line cap** on every `.rs` / `.ts` / `.tsx` file under `src/`
  and `src-tauri/src/`.
- **Dep hygiene.** Every VAD call site must have a named caller.
  Reintroducing `vad-rs` (or an `ort`-direct path) is only acceptable
  if `cargo machete` stays clean and if the `dep-hygiene` and
  `handy-legacy-pruning` skill kill-lists are updated in the same PR
  to reflect the live-caller status.
- **i18n.** Any new user-visible string lands in all 20 locale files
  in the same commit; `scripts/check-translations.ts` exits 0.
- **Model distribution.** The Silero ONNX flows through the existing
  downloader under `src-tauri/src/managers/model/`. It is NOT bundled
  in the installer.
- **`bindings.ts` is specta-generated.** New commands flow through
  specta; no hand edits.
- **No mic / PTT / live dictation.** Silero is file-only in this
  feature.

## 4. Non-goals (explicit)

- Reintroducing microphone push-to-talk, live dictation, recorder,
  input hotkeys, or any of the files in the `handy-legacy-pruning`
  "fully removed" set (`actions.rs`, `shortcut/`, `overlay.rs`,
  `tray*.rs`, `clipboard.rs`, `input.rs`, `audio_feedback.rs`,
  `apple_intelligence.rs`, `audio_toolkit/audio/recorder.rs`,
  `PushToTalk.tsx`, `AudioFeedback.tsx`, etc.). Only
  `audio_toolkit/vad/*` comes back, and only with file-based callers.
- Replacing forced alignment or ASR word-timing. Silero is binary
  speech / non-speech, not a word aligner.
- Speaker diarization.
- Bundling the Silero ONNX in the installer.
- Changing existing auto-delete or auto-silence defaults. The filler
  classifier is metadata-only until a future UX feature acts on it.

## 5. Success

- Transcription runtime on a silence-heavy fixture drops by a
  measurable, recorded percentage vs. baseline (number captured in
  the PR body, not "should be faster").
- Whisper hallucination-on-silence count drops by a measurable,
  recorded delta on a music/silence-mixed fixture.
- `transcript-precision-eval`, `audio-boundary-eval`, and
  `cut-drift-fuzzer` all stay green. Per-word microsecond timestamps
  in file time are preserved — the pre-filter never leaks
  window-relative time into the transcript.
- `cargo machete` is clean; any new crate has at least one live
  caller; the `dep-hygiene` and `handy-legacy-pruning` skill docs are
  updated in lockstep.

## 6. References

- Research report:
  `C:\Users\alexm\.copilot\session-state\12c1b358-6581-40c1-87ae-4516a84e344b\research\https-github-com-snakers4-silero-vad-how-would-sil.md`
- `AGENTS.md` — repository layout and non-negotiable boundaries.
- `.github/skills/handy-legacy-pruning/SKILL.md` — kill-list entry for
  `audio_toolkit/vad/*`.
- `.github/skills/dep-hygiene/SKILL.md` — kill-list entry for
  `vad-rs`.
- `.github/skills/transcript-precision-eval/SKILL.md`,
  `.github/skills/audio-boundary-eval/SKILL.md`.
- Handy sources (verbatim-reusable):
  - `cjpais/Handy@af6ec6c:src-tauri/src/audio_toolkit/vad/mod.rs`
  - `cjpais/Handy@af6ec6c:src-tauri/src/audio_toolkit/vad/silero.rs`
  - `cjpais/Handy@af6ec6c:src-tauri/src/audio_toolkit/vad/smoothed.rs`
- Current Toaster call-site files this feature edits:
  - `src-tauri/src/managers/splice/boundaries.rs:1-60`
  - `src-tauri/src/managers/filler.rs:1-57`
  - `src-tauri/src/managers/model/` (catalog + downloader)
  - `src-tauri/src/managers/transcription/` (pre-filter integration)

## Q&A

Phase 5 (Q&A) is deferred. The user's prompt already disambiguated
the three use cases, the non-goals, the constraints, and the success
criteria. STATE.md remains `defined`; the PRD below is gated on
explicit user approval before `superpowers:executing-plans` runs.
