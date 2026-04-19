# Blueprint: reintroduce Silero VAD (file-based, editor use cases)

## Architecture decisions

### AD-1 — Reuse `ort` directly; do **not** pull `vad-rs` back

`transcribe-rs` already brings `ort` into the Rust dependency graph
for its ONNX-backed ASR models. `vad-rs` is a thin wrapper around
`ort` + the Silero model. Reusing `ort` directly gives us:

- One ONNX runtime, one set of native DLL/SO loading quirks, one
  upgrade path.
- No reintroduction of a Handy-era crate name on the `dep-hygiene`
  kill-list (the entry is removed because we have live callers in a
  different shape, not because we brought the exact crate back).
- Full control over session options (thread count, providers, tensor
  pool reuse), which matters for AC-002-b (runtime delta).

Trade-off: ~50 lines of input-tensor plumbing we would otherwise get
for free. Acceptable; Handy's `silero.rs` wrapper is ~52 lines and is
the template.

### AD-2 — Lift Handy's trait and hysteresis verbatim

`cjpais/Handy@af6ec6c:src-tauri/src/audio_toolkit/vad/mod.rs` and
`smoothed.rs` are MIT and already satisfy the invariants we need
(30 ms frame push, prefill / onset / hangover). The research report
confirmed these are "directly reusable". We copy them byte-for-byte
(plus MIT header + provenance comment) rather than rewrite.

### AD-3 — One `VoiceActivityDetector` instance per file analysis

All three use cases operate on a **decoded file** at analysis time,
not a live stream. A single ONNX session is created on the first use
and reused across the three callers via an
`Arc<Mutex<SmoothedVad<SileroVad>>>` held on the relevant manager.
No duplicate instances, no per-caller model loads. The manager owns
the handle; callers pass the `&mut` wrapper.

### AD-4 — Pre-filter emits windows, not rewritten audio

Use case 1 does **not** construct a shortened audio buffer and hand
it to the ASR. That would couple VAD to the ASR's buffer format and
make timestamp remapping fragile. Instead, pre-filter emits a list
of `SpeechWindow { start_us, end_us }` spans (with pre-roll /
hangover already applied). The transcription manager calls the ASR
per-window, and a single remap step (`word.start_us += window.start_us`)
reprojects timestamps into file time before the result goes to the
editor. This keeps the timestamp math localized to one function.

### AD-5 — Boundary refinement is additive, not replacement

`splice/boundaries.rs` keeps its existing zero-crossing and
energy-valley logic. The VAD step is a **tiebreak** applied inside
the existing search window: among all zero-crossings / energy
minima within the existing radii, prefer the one co-located with
the lowest P(speech). When `vad_refine_boundaries=false`, the code
path is byte-identical to pre-feature (AC-003-d). This is what makes
the dual-path invariant holdable: preview and export both call the
same function, so they both see the same P(speech)-aware choice.

### AD-6 — Filler classifier is metadata-only

`filler.rs` currently reports gaps >= 1.5 s. This feature attaches a
`GapClassification` tag but does **not** change
`auto_delete_fillers` or `auto_silence_pauses` defaults. Future UX
features can consume the tag. This minimises risk and keeps the PR
reviewable.

### AD-7 — Model distribution through existing downloader

The Silero ONNX is ~2 MB and is fetched on demand by
`src-tauri/src/managers/model/` (same flow used for Whisper /
Parakeet / Moonshine models). No installer bundling.

### AD-8 — Graceful degradation is the default everywhere

Missing model, ORT init failure, frame-size mismatch, and resample
failure all fall back to the pre-feature behaviour with one
`tracing::warn!` on the first incidence. Never panic. Covered by
AC-002-d, AC-005-c.

## Module layout

```
src-tauri/src/
  audio_toolkit/
    vad/
      mod.rs           # trait VoiceActivityDetector { push_frame, reset }
                       # plus SILERO_FRAME_SAMPLES + VadFrame enum.
                       # LIFT VERBATIM from Handy af6ec6c vad/mod.rs.
      silero.rs        # SileroVad: ort Session + 30ms f32 frame push.
                       # Structurally equivalent to Handy silero.rs but
                       # wires ort directly instead of via vad-rs.
      smoothed.rs      # SmoothedVad<V>: prefill / onset / hangover.
                       # LIFT VERBATIM from Handy af6ec6c smoothed.rs.
  managers/
    model/
      catalog/         # add silero-vad entry (id, url, sha256, size)
      download.rs      # no change beyond catalog entry pick-up
    transcription/
      prefilter.rs     # NEW. fn prefilter_speech_windows(
                       #   samples: &[f32], sr: u32, vad: &mut SmoothedVad<...>
                       # ) -> Vec<SpeechWindow>
                       # fn remap_words(words, window_offset_us) -> ()
      mod.rs           # feature-gate: if settings.vad_prefilter_enabled
                       # and model_present(), run prefilter; else fall
                       # back to the existing full-file path.
    splice/
      boundaries.rs    # add optional `vad_curve: Option<&[f32]>` param
                       # (P(speech) sampled at energy-frame cadence).
                       # When Some and settings.vad_refine_boundaries,
                       # re-rank candidates by local min P(speech).
                       # Preview + export both call through this. SSoT.
    filler.rs          # add GapClassification enum; when gap >= 1.5 s
                       # and vad available, classify the gap and
                       # attach to the emitted metadata.
  settings/
    types.rs           # + pub vad_prefilter_enabled: bool
                       # + pub vad_refine_boundaries: bool
    defaults.rs        # defaults true / false respectively
  commands/
    (no new commands — settings flow through the generic settings
     command surface; model download flows through the existing
     model commands.)
```

Frontend:

```
src/components/settings/
  transcription/
    TranscriptionSettings.tsx   # + <VadPrefilterToggle/>
  editor/
    EditorSettings.tsx          # + <VadRefineBoundariesToggle/>
```

i18n:

```
src/i18n/locales/*/translation.json
  settings.transcription.vadPrefilter.{label,help}
  settings.editor.vadRefineBoundaries.{label,help}
```

Skill docs:

```
.github/skills/dep-hygiene/SKILL.md         # remove vad-rs entry;
                                             # document ort callers
.github/skills/handy-legacy-pruning/SKILL.md # remove audio_toolkit/vad/*
                                             # from "fully removed";
                                             # add "VAD reintroduced"
                                             # section citing R-002..4
AGENTS.md                                   # add audio_toolkit/vad/
                                             # back to repo-layout block
```

## Data flow

### Use case 1 — ASR silence pre-filter (R-002)

```
file on disk
   -> ffmpeg decode (existing) -> f32 PCM @ source sample rate
   -> resample to 16 kHz (local utility) -> f32 PCM 16 kHz
   -> prefilter_speech_windows(pcm_16k, 16000, &mut smoothed_vad)
      -> iterate 30 ms frames
      -> SmoothedVad emits VadFrame::Speech / Noise with prefill +
         onset + hangover applied
      -> collapse consecutive Speech frames into SpeechWindow spans
      -> each span expanded by PREROLL_MS / HANGOVER_MS (see below)
      -> returns Vec<SpeechWindow { start_us, end_us }>
   -> for each SpeechWindow:
        ASR(pcm[window.start..window.end])
        -> WordList { start_us, end_us relative to window }
        -> remap_words: w.start_us += window.start_us;
                        w.end_us   += window.start_us
   -> concat WordLists in file-time order
   -> hand to EditorManager (existing path)
```

Parameters (all in `audio_toolkit/vad/smoothed.rs` constants,
single source of truth):

- `PREROLL_MS = 120` — 4 frames of 30 ms. Matches Handy's default.
- `ONSET_FRAMES = 2` — 60 ms of speech required to open a window.
- `HANGOVER_MS = 200` — ~7 frames of silence grace before closing.
- `SPEECH_PROB_THRESHOLD = 0.5` — Silero's own default.

### Use case 2 — splice-boundary refinement (R-003)

```
editor requests cut at target_us
   -> boundaries::snap_cut(target_us, samples, sr, vad_curve_opt)
      -> candidates = zero_crossings_within(DEFAULT_SNAP_RADIUS_US)
                    ∪ energy_valleys_within(DEFAULT_ENERGY_RADIUS_US)
      -> if vad_curve_opt.is_some() && settings.vad_refine_boundaries:
           score(c) = existing_score(c) + α * P_speech_at(c)
           pick argmin score
         else:
           pick by existing logic
      -> return snapped_us
   <- preview and export both consume snapped_us verbatim.
```

`vad_curve` is sampled at the same cadence as the energy-valley
search (2 ms frames per `ENERGY_FRAME_MS`) so the two signals
co-register without interpolation.

### Use case 3 — filler / pause acoustic classifier (R-004)

```
filler::analyze(words, FillerConfig, Option<&VadCurve>)
   -> for each gap where (w[i+1].start_us - w[i].end_us) >= 1_500_000:
        if vad_curve.is_some():
          frames = vad_curve.slice(gap.start..gap.end)
          if mean(frames) < 0.1:         TrueSilence
          elif max(frames) < 0.5:        NonSpeechAcoustic
          else:                          MissedSpeech
        else:
          GapClassification::Unknown
      -> Gap { start_us, end_us, classification }
   -> FillerAnalysis { fillers, gaps }
```

No default behavior change: `auto_delete_fillers` and
`auto_silence_pauses` stay `false`. Metadata only.

## Single-source-of-truth placement

- **VAD model and hysteresis:** one `SmoothedVad<SileroVad>`
  instance, held by whichever manager drives the current analysis.
  Passed by `&mut` to callers; never cloned.
- **Speech-window expansion (prefill / hangover):** constants live
  in `audio_toolkit/vad/smoothed.rs`. Frontend never knows them.
- **Boundary refinement:** single function in
  `splice/boundaries.rs`; preview and export both call it.
  `waveform-diff` (AC-003-b) verifies parity, not asserts it.
- **Filler list:** already backend-canonical in
  `filler.rs::DEFAULT_FILLERS`; this feature does not touch it.
- **Settings schema:** `src-tauri/src/settings/types.rs` is the
  source; `bindings.ts` is specta-generated. Frontend never declares
  an independent type for the two new bools.

## Migration / compatibility

- **Settings JSON on user disk.** Two new bool fields with serde
  defaults (`true`, `false`). `#[serde(default)]` already the
  project pattern; upgrades transparently.
- **Model cache.** Fresh installs do not have the Silero ONNX. All
  three use cases detect absence and fall back; user hits Download
  in Settings to activate. AC-005-c locks this in.
- **`bindings.ts`.** Regenerated on `cargo tauri dev` / `npm run
  build`; two new settings fields appear automatically.
- **Existing fixtures.** The `eval/` fixture set is unchanged. The
  pre-filter result on any existing fixture must be byte-identical
  to the non-VAD path for the precision eval — this is the
  timestamp-remapping correctness bar (AC-002-a).

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Pre-filter remapping bug leaks window-relative timestamps into the transcript | Single remap call-site; unit test per R-002; precision eval gate | AC-002-a, AC-008-a |
| Silero 30 ms frame requirement mis-aligned with decoded sample rate | Resample to 16 kHz unconditionally before VAD; local utility | AC-001-a (cargo check), AC-002-a |
| ORT init fails on a user platform | Detect, warn once, fall back to full-file path | AC-002-d, AC-005-c |
| Boundary refinement subtly changes outputs when disabled | `vad_refine_boundaries=false` takes the pre-feature code path; SHA-256 byte-equality on fixture export | AC-003-d |
| Preview and export pick different boundaries | Single function in `splice/boundaries.rs`; both paths import it; waveform-diff gate | AC-003-b |
| Cut-drift grows under VAD-aware refinement | Fuzzer with 1000 ops | AC-003-c |
| Filler classifier silently auto-deletes gaps | Defaults unchanged; grep gate on `auto_delete_fillers: true` additions | AC-004-c |
| `vad-rs` silently creeps back via a transitive | AD-1: do not add `vad-rs` to `Cargo.toml`; `cargo machete` gate | AC-007-a |
| Skill kill-lists forgotten, future CI rejects `audio_toolkit/vad/*` | Two doc edits in R-007; grep-verified | AC-007-b, AC-007-c |
| 800-line cap breach when folding Handy's files in | smoothed.rs ~150 LOC, silero.rs ~100 LOC, mod.rs ~30 LOC per Handy — well under 800 | AC-001-c |
| Hallucination delta unmeasurable (fixtures too clean) | Curate a music/silence-mixed fixture under `eval/fixtures/`; record baseline numbers in journal.md before toggling | AC-002-c, AC-008-d |

## Open questions (none blocking)

None. The user's prompt pinned scope, ranked the use cases, selected
defaults for the two settings, and enumerated the constraints. Any
implementation-level ambiguity is resolved by AD-1 through AD-8
above.
