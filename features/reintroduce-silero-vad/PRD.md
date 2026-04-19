# PRD: reintroduce Silero VAD (file-based, editor use cases)

## Background

Handy — Toaster's fork parent — used Silero VAD (via `vad-rs`) in the
microphone path for push-to-talk endpointing and silence trimming.
Toaster is a transcript-first video/audio editor, not a live dictation
tool, so the mic path was deleted: `src-tauri/src/audio_toolkit/vad/*`
is gone, `vad-rs` is on the `dep-hygiene` skill's Handy-era-only
kill-list, and the `handy-legacy-pruning` skill lists
`audio_toolkit/vad/*` in the "fully removed" set.

The completed research report
(`C:\Users\alexm\.copilot\session-state\12c1b358-6581-40c1-87ae-4516a84e344b\research\https-github-com-snakers4-silero-vad-how-would-sil.md`)
identifies three **file-based** Toaster use cases where Silero has
measurable ROI. None are mic-adjacent. This PRD authorises bringing
the VAD module back with live file-based callers only, updating the
skill kill-lists in lockstep, and gating the change behind
measurable eval deltas.

## Problem & Goals

- Whisper hallucinates text on silent / music regions; ASR also pays
  full compute on silence. Use case 1 pre-filters the input so only
  speech windows reach the ASR.
- `splice/boundaries.rs` snaps to zero-crossings (5 ms) and
  energy-valleys (20 ms) but is blind to speech vs. non-speech. Use
  case 2 adds a P(speech)-aware tiebreak inside the existing radii.
- `filler.rs` reasons about pauses only from ASR word-gap
  timestamps. Use case 3 lets a long gap be classified by acoustic
  content (silence / non-speech / missed-speech) without changing
  auto-delete defaults.

Goal: reintroduce `audio_toolkit/vad/` with a single
`VoiceActivityDetector` trait instance shared by all three use
cases, behind a downloaded ONNX model, with fixture-based eval
deltas gating the PR.

## Scope

### In scope

- Recreate `src-tauri/src/audio_toolkit/vad/{mod.rs, silero.rs,
  smoothed.rs}`. Trait and `SmoothedVad` hysteresis lifted verbatim
  from Handy `af6ec6c`.
- Pick ONNX runtime: `ort` direct (preferred — already in tree via
  `transcribe-rs`) or `vad-rs` crate. Decision in BLUEPRINT.md.
- Add a `silero-vad` catalog entry + downloader wiring in
  `src-tauri/src/managers/model/`.
- Pre-filter integration in
  `src-tauri/src/managers/transcription/` — runs before any ASR
  pass, rewrites speech windows into file-time, preserves per-word
  microsecond timestamps.
- Boundary refinement in
  `src-tauri/src/managers/splice/boundaries.rs` — optional
  P(speech)-aware snap, shared single source of truth for preview
  and export.
- Filler-gap classifier in `src-tauri/src/managers/filler.rs` — new
  metadata field on long gaps; no default-behavior change.
- Two settings fields (`vad_prefilter_enabled`,
  `vad_refine_boundaries`), specta-exported, rendered in
  Transcription / Editor settings panels.
- i18n keys in all 20 locales for the new UI strings.
- Update `.github/skills/dep-hygiene/SKILL.md` and
  `.github/skills/handy-legacy-pruning/SKILL.md` to remove the
  `vad-rs` and `audio_toolkit/vad/*` kill-list entries, replacing
  them with "live-caller" notes that cite the three new use cases.
- Update `AGENTS.md` repository-layout block to add
  `audio_toolkit/vad/` back.

### Out of scope (explicit)

- Any microphone / push-to-talk / live dictation reintroduction. The
  deleted Handy files (`actions.rs`, `shortcut/`, `overlay.rs`,
  `tray*.rs`, `clipboard.rs`, `input.rs`, `audio_feedback.rs`,
  `apple_intelligence.rs`, `audio_toolkit/audio/recorder.rs`,
  `PushToTalk.tsx`, `AudioFeedback.tsx`, etc.) stay deleted.
- Forced-alignment or ASR word-timing replacement. Silero is binary;
  per-word timestamps continue to come from `transcribe-rs`.
- Speaker diarization.
- Installer-bundled Silero ONNX. Model flows through the existing
  downloader.
- Changing auto-delete / auto-silence defaults in `filler.rs`.
- New VAD commands beyond those needed to expose the two settings
  and trigger a model download.

## Requirements

### R-001 — Reintroduce the `audio_toolkit/vad` module with a single trait

- Description: Recreate `src-tauri/src/audio_toolkit/vad/mod.rs`
  (trait), `silero.rs` (ONNX-backed implementation), and
  `smoothed.rs` (hysteresis with prefill / onset / hangover). All
  three use cases consume the trait; there is exactly one
  implementation in-tree. File-size cap 800 lines each.
- Acceptance Criteria
  - AC-001-a — `cd src-tauri && cargo check` exits 0 after the new
    module lands.
  - AC-001-b — `rg "pub (trait|struct) VoiceActivityDetector|SmoothedVad|SileroVad" src-tauri/src/audio_toolkit/vad`
    returns exactly one definition site per symbol (no duplicates).
  - AC-001-c — every `.rs` file under
    `src-tauri/src/audio_toolkit/vad/` is <= 800 lines, verified by
    `Get-ChildItem src-tauri/src/audio_toolkit/vad -Recurse -Filter *.rs | % { (Get-Content $_ | Measure-Object -Line).Lines } | Sort-Object -Descending | Select -First 1`.

### R-002 — ASR silence pre-filter

- Description: Before any `transcribe-rs` pass, when
  `settings.vad_prefilter_enabled` is true and the model is
  downloaded, run Silero over the decoded file audio to produce
  speech windows with pre-roll / hangover padding (parameters
  defined in BLUEPRINT.md). Feed only those windows to the ASR.
  Word timestamps emitted by the ASR are remapped to **absolute
  file time** (microsecond precision) before being handed to the
  editor. If the model is missing or ORT init fails, the feature
  degrades silently to the current full-file ASR path.
- Acceptance Criteria
  - AC-002-a — `pwsh scripts/eval/eval-edit-quality.ps1` exits 0
    with pre-filter enabled on the standard fixture set (per-word
    timing preserved, no equal-duration synthesis, midstream
    deletions still clean).
  - AC-002-b — Manual measurement on a silence-heavy fixture:
    transcription wall-time drops by a recorded percentage vs. the
    pre-filter-disabled baseline; both numbers recorded in
    `journal.md` and the PR body.
  - AC-002-c — Manual measurement on a music/silence-mixed fixture:
    count of Whisper-emitted words whose timestamps land entirely
    inside VAD-flagged non-speech regions drops to zero (or to a
    recorded non-zero value with explanation).
  - AC-002-d — With the Silero model file removed from the cache,
    `cd src-tauri && cargo test --test prefilter_degrades_gracefully`
    (test introduced by this feature) exits 0 and the full-file
    ASR path runs.

### R-003 — Splice-boundary refinement

- Description: Extend
  `src-tauri/src/managers/splice/boundaries.rs` with an optional
  P(speech)-aware tiebreak. When
  `settings.vad_refine_boundaries` is true and the model is
  available, the snap picks the local minimum of P(speech) within a
  ±100 ms window that is **still within** the existing
  `DEFAULT_SNAP_RADIUS_US` / `DEFAULT_ENERGY_RADIUS_US` radii.
  Preview and export both consume the same function; single source
  of truth preserved.
- Acceptance Criteria
  - AC-003-a — `pwsh scripts/eval/eval-audio-boundary.ps1` exits 0
    with `vad_refine_boundaries=true`; no seam clicks; deleted
    audio does not leak across the splice.
  - AC-003-b — `waveform-diff` agent run on a standard splice
    fixture reports preview and export PCM within 1-sample seam
    parity across all cuts (identical boundary policy verified, not
    asserted).
  - AC-003-c — `cut-drift-fuzzer` agent run (1000 ops, seeded)
    reports zero cumulative duration drift, monotonic time maps,
    zero panics with `vad_refine_boundaries=true`.
  - AC-003-d — With `vad_refine_boundaries=false` (default),
    `pwsh scripts/eval/eval-audio-boundary.ps1` exits 0 and
    produces byte-identical output vs. the pre-feature baseline
    (verified by SHA-256 of the fixture export).

### R-004 — Filler / pause acoustic classifier

- Description: In `src-tauri/src/managers/filler.rs`, when a gap
  between adjacent word timestamps exceeds
  `DEFAULT_PAUSE_THRESHOLD_US` (1.5 s) **and** the Silero model is
  available, run a Silero pass over the gap and attach a
  `GapClassification { True Silence | Non Speech Acoustic |
  Missed Speech }` tag to the gap metadata surfaced to the editor.
  Default `auto_delete_fillers` and `auto_silence_pauses` stay
  false; this feature adds metadata only.
- Acceptance Criteria
  - AC-004-a — Unit test `cd src-tauri && cargo test --test
    filler_gap_classification` (introduced by this feature) exits
    0; fixture includes one gap of each class and asserts the
    correct tag.
  - AC-004-b — `pwsh scripts/eval/eval-edit-quality.ps1` exits 0
    with the classifier active (no regression in precision eval).
  - AC-004-c — `rg -n "auto_delete_fillers: true|auto_silence_pauses: true" src-tauri/src`
    returns no new matches introduced by this feature (defaults
    unchanged).

### R-005 — Model catalog, downloader, and graceful absence

- Description: Add a `silero-vad` entry to the model catalog under
  `src-tauri/src/managers/model/catalog/` with the ONNX URL,
  SHA-256, and ~2 MB size. The existing downloader under
  `src-tauri/src/managers/model/` handles fetch / verify / on-disk
  placement. When the model is not present, use cases 1, 2, and 3
  silently fall back to their pre-feature behaviour.
- Acceptance Criteria
  - AC-005-a — `rg -n "silero" src-tauri/src/managers/model/catalog`
    returns the new catalog entry.
  - AC-005-b — Manual step: from a fresh cache, toggle
    "Pre-filter silences before transcribing" on in Settings, click
    Download, observe the file appear on disk with the expected
    SHA-256; uncheck the toggle; observe the settings change
    persists across an app restart.
  - AC-005-c — `cd src-tauri && cargo test --test
    vad_missing_model_degrades` (introduced by this feature) exits
    0 in a sandbox where the model file does not exist.

### R-006 — Settings and UI

- Description: Two bool settings fields introduced in
  `src-tauri/src/settings/types.rs`:
  - `vad_prefilter_enabled: bool` — default `true`.
  - `vad_refine_boundaries: bool` — default `false` (pending
    eval-win in R-003).
  Frontend surfaces each as a single toggle in the Transcription
  and Editor settings panels respectively, following the existing
  `design-system` patterns (hero + SettingsGroup + SettingContainer,
  no hex colors, button-variant compliant). Backend-authoritative:
  the toggle only writes the setting; Silero runs in the backend.
- Acceptance Criteria
  - AC-006-a — `bun run scripts/check-translations.ts` exits 0 after
    the new i18n keys land in all 20 locales.
  - AC-006-b — `npm run build` exits 0; `src/bindings.ts` includes
    the two new settings fields (specta-generated, not hand-edited).
  - AC-006-c — Manual live-app step
    (`pwsh scripts/launch-toaster-monitored.ps1
    -ObservationSeconds 180`): toggle each setting on and off;
    confirm the change persists in `settings.json` and propagates
    to the running backend (journal.md records timestamp + operator
    initials).
  - AC-006-d — `pwsh scripts/eval/eval-verifier.ps1 -Pattern "#\[allow\(dead_code\)\].*vad_(prefilter_enabled|refine_boundaries)" -Path src-tauri/src -ExpectZero`
    confirms neither new setting is dead-coded at merge time.

### R-007 — Dep hygiene and skill doc updates

- Description: Decide in BLUEPRINT.md between `ort`-direct and
  `vad-rs`. Whichever is chosen, `cargo machete` stays clean after
  the feature lands, and the following skill docs are updated in
  lockstep:
  - `.github/skills/dep-hygiene/SKILL.md` — remove the `vad-rs`
    kill-list entry (or record `ort` as the direct caller);
    document the named callers (pre-filter, boundary, filler).
  - `.github/skills/handy-legacy-pruning/SKILL.md` — remove
    `audio_toolkit/vad/*` from the "fully removed" list and add a
    "reintroduced with file-only callers" note citing R-002, R-003,
    R-004.
- Acceptance Criteria
  - AC-007-a — `cd src-tauri && cargo machete` reports zero unused
    dependencies.
  - AC-007-b — `pwsh scripts/eval/eval-verifier.ps1 -Pattern "vad-rs|audio_toolkit/vad/\*" -Path .github/skills -ExpectZero`
    confirms the kill-list entries are gone.
  - AC-007-c — `.github/skills/handy-legacy-pruning/SKILL.md`
    contains a new section whose heading includes the string
    "VAD reintroduced" and which cites each of R-002, R-003, R-004
    by name (verified by doc-section grep).

### R-008 — Cross-cutting eval gates

- Description: The three precision / boundary / drift evals stay
  green with all VAD features enabled, and the transcription
  runtime + hallucination deltas are recorded.
- Acceptance Criteria
  - AC-008-a — `pwsh scripts/eval/eval-edit-quality.ps1` exits 0
    with `vad_prefilter_enabled=true` and
    `vad_refine_boundaries=true` both on.
  - AC-008-b — `pwsh scripts/eval/eval-audio-boundary.ps1` exits 0
    with both VAD features on.
  - AC-008-c — `pwsh scripts/eval/run-eval-harness.ps1` exits 0
    (umbrella harness; confirms no sibling eval regresses).
  - AC-008-d — Manual: PR body reports before/after numbers for
    transcription runtime (silence-heavy fixture) and
    hallucination-on-silence count (music/silence fixture).

## Edge cases & constraints

- **ORT init failure on exotic platforms.** If `ort` fails to
  create a session (missing CUDA runtime, missing DLL, corrupted
  model), pre-filter and boundary refinement must log once and
  degrade to pre-feature behaviour; the app must not panic or
  refuse to transcribe. Covered by AC-002-d and AC-005-c.
- **Frame-size mismatch.** Silero requires 30 ms frames at 8 or
  16 kHz. Decoded audio at other sample rates must be resampled
  before VAD. The resampler is a local utility; no new dependency.
- **Timestamp remapping correctness.** Use case 1 is the highest
  regression risk for `transcript-precision-eval`. Every word
  timestamp from the ASR is shifted by the start-offset of the
  speech window it came from; the remapping is unit-tested and
  exercised by AC-002-a.
- **Dual-path invariant.** Use case 2 edits a single function in
  `splice/boundaries.rs`; preview and export both import it. AC-003-b
  (waveform-diff) verifies the invariant, not just asserts it.
- **i18n parity.** Two new toggles means two new keys across 20
  locales. AC-006-a gates this.
- **Kill-list drift.** If the skill docs are not updated in the
  same PR, `dep-hygiene` and `handy-legacy-pruning` will still
  report `vad-rs` / `audio_toolkit/vad/*` as forbidden, breaking
  future CI runs. AC-007-b and AC-007-c gate this.

## Data model

New settings fields:

| Path | Field | Type | Default |
|------|-------|------|---------|
| `src-tauri/src/settings/types.rs` | `vad_prefilter_enabled` | `bool` | `true` |
| `src-tauri/src/settings/types.rs` | `vad_refine_boundaries` | `bool` | `false` |

New gap-metadata enum (scoped to `filler.rs`):

```
pub enum GapClassification {
    TrueSilence,
    NonSpeechAcoustic,
    MissedSpeech,
}
```

Model catalog entry:

| Field | Value |
|-------|-------|
| `id` | `silero-vad` |
| `url` | upstream ONNX release URL (recorded at BLUEPRINT time) |
| `sha256` | recorded at BLUEPRINT time |
| `size_bytes` | ~2 000 000 |
| `sample_rates_hz` | `[8000, 16000]` |

## Non-functional requirements

- Every file under `src-tauri/src/audio_toolkit/vad/` and every
  touched file under `src-tauri/src/managers/` stays <= 800 lines.
- No hex literals in the new TS components (`design-system` gate).
- All planning artifacts and code comments are ASCII; no smart
  quotes.
- Single source of truth: no duplication of VAD logic into the
  frontend; the two settings toggles only flip backend config.
- AGENTS.md is the only AI-instruction file touched (per
  `canonical-instructions` skill). Skill docs are documentation,
  not instructions, and are updated explicitly by R-007.
