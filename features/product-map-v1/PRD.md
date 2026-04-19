# PRD: Product map to v1 launch

> Discovery + planning deliverable. Not implementable. Each roadmap
> item under §6 must be re-scaffolded as its own `features/<slug>/`
> bundle before any code is written.

## Problem & Goals

Make Toaster's path to a credible v1.0 visible and actionable. Today
the team has shipped a working transcript-first edit loop on top of
local Whisper, but there is no consolidated view of:

- which capabilities are truly shipped vs. partial vs. orphaned;
- which surfaces exist in code but are unreachable from the running
  UI;
- what an outside user would call missing on a v1.0 launch;
- which of the many FFmpeg capabilities we should adopt without
  violating the "simplicity first" rule.

## Scope

### In scope

- Inventory + gap analysis across backend, frontend, scripts, evals
- Roadmap structured as 3 milestones (Foundation / Polish / Launch-Ready)
- FFmpeg opportunity map with explicit Include / Defer / Reject calls
- Anti-scope and open-question list for the human

### Out of scope (explicit)

- Production code edits
- Re-proposing in-flight bundles (`brand-title-sizing`,
  `caption-settings-preview`, `remove-history-and-legacy`,
  `build-env-ninja-hardening`)
- Anything requiring runtime network calls
- Real-time dictation / push-to-talk / tray surfaces (Handy-era;
  see anti-scope §7)

---

## 1. Executive summary

Toaster today is a Tauri 2.x desktop app that opens an audio/video
file, transcribes it locally with one of seven ASR backends (Whisper
default), lets the user edit at the word level (delete, silence,
restore, split, undo/redo), keeps preview and waveform in sync via a
backend-authoritative keep-segment / time-mapping engine, and exports
edited media via an FFmpeg pipeline (H.264/AAC mp4) plus caption
sidecars (SRT/VTT/script) and optional ASS burn-in. The
single-source-of-truth rule for dual-path logic is real and largely
honored: caption layout (`captions::build_blocks`), seam fades (20 ms
symmetric), zero-crossing snap (`splice::boundaries`), and filler
detection are all backend-owned.

"v1.0 launch" means: a stranger downloads the installer, runs through
onboarding, transcribes a 20-minute file, removes fillers, exports a
clean mp4 + SRT, and never hits a Handy-era artifact, an unreachable
settings panel, or a SmartScreen install warning that scares them off.

**Top 3 blockers:**
1. Unreachable / orphaned UI surfaces (post-processing, debug,
   `general` sidebar key) leak through translations and store code.
2. Export is mp4/H.264-only with no audio-only export and no loudness
   normalization gating — `splice::loudness` measurement infra exists
   but is not wired to the pipeline.
3. No code-signed installers and no monitored-launcher equivalent for
   non-Windows users; Linux/macOS first-run paths are unverified.

**Top 3 quick wins:**
1. Wire `splice::loudness::measure_loudness` into the export pipeline
   as an opt-in pre-flight warning (infra ready —
   `src-tauri/src/managers/splice/loudness.rs:40`).
2. Add audio-only export presets (mp3 / wav / m4a) — pure FFmpeg
   filter-chain reuse, no UI invention beyond a format dropdown.
3. Delete the unreachable `sidebar.general` / `sidebar.debug` /
   `sidebar.postProcessing` translation keys and their orphan
   components (extends `remove-history-and-legacy` patterns).

---

## 2. Current capability inventory

State legend: **shipped** = wired end-to-end and reachable from the
running UI; **partial** = present and tested but not exposed or only
covers part of the surface; **dead-code-still-present** = code exists
but no consumer; **undocumented** = working in code but not in
README/AGENTS/settings labels.

### Transcription

| Capability | Source of truth | State | User-facing surface |
|---|---|---|---|
| Multi-backend ASR adapter (7 engines) | `src-tauri/src/managers/transcription/adapter.rs` | shipped | Models settings panel |
| Whisper backend | `managers/transcription/` + `whisper-rs-sys` | shipped | Default model |
| Parakeet / Moonshine / SenseVoice / GigaAM / Canary / Cohere | `managers/transcription/adapter.rs` | partial | Selectable in catalog; only Whisper exercised by evals |
| Forced-alignment word timing | `audio_toolkit/forced_alignment.rs` | shipped | Implicit |
| Adapter normalization invariants | `managers/transcription/adapter_normalize.rs` + `transcription-adapter-contract` skill | shipped | None (gate) |
| Accelerator selection (CUDA / TensorRT / CoreML / OpenVINO) | `managers/transcription/accelerators.rs`; settings `WhisperAcceleratorSetting`, `OrtAcceleratorSetting` | partial | Surfaced in code; UI selector retained per `remove-history-and-legacy` BLUEPRINT |
| Model unload timeout (idle watcher) | `managers/transcription/mod.rs:94-100` + `ModelUnloadTimeoutSetting.tsx` | shipped | Models settings |
| Local LLM cleanup post-processing | `managers/cleanup/` + `llm_client.rs` | partial | Settings exist; UI panel **unreachable** (see §3) |
| `transcribe_media_file` end-to-end | `commands/transcribe_file/mod.rs` | shipped | Editor "Transcribe" button |

### Editing

| Capability | Source of truth | State | User-facing surface |
|---|---|---|---|
| Word-level delete / restore / silence / split | `managers/editor/mod.rs:74,98,...` | shipped | TranscriptEditor + context menu |
| Range delete + restore-all | `managers/editor/mod.rs:98` | shipped | Drag-select + context |
| Undo/redo (≤64 snapshots) | `managers/editor/mod.rs` | shipped | Hotkeys + context |
| Backend-authoritative keep-segments | `managers/editor/mod.rs:195` | shipped | Preview + export |
| Timing contract snapshot | `managers/editor/mod.rs` | shipped | Preview consumer |
| Filler-word detection (configurable list + pause threshold) | `managers/filler.rs:106` | shipped | FillerDashboard |
| Filler / pause / duplicate iterative analyzer | `commands/filler.rs analyze_fillers` | shipped | "Analyze" button |
| Pause trim / silence | `commands/filler.rs trim_pauses,silence_pauses` | shipped | FillerDashboard buttons |
| Audio-aware disfluency cleanup | `managers/disfluency.rs` + `commands/disfluency.rs cleanup_duplicates_smart` | shipped | "Smart cleanup" |
| Local-LLM word proposals apply | `managers/editor/mod.rs LocalLlmApplyResult` + `commands/editor.rs editor_apply_local_llm_proposals` | partial | Backend command exists; UI invocation depends on (currently unreachable) post-processing panel |
| Find/replace (case sensitive, delete-all) | `components/editor/FindReplaceBar.tsx` | shipped | Editor toolbar |
| `FillerConfig.auto_delete_fillers`, `auto_silence_pauses` | `managers/filler.rs:41,44` | dead-code-still-present | None |
| `PARAKEET_OUTER_TRIM_US` constant | `managers/editor/mod.rs:40` (`#[allow(dead_code)]`) | dead-code-still-present | None — forward-looking for engine-aware trim |

### Captions

| Capability | Source of truth | State | User-facing surface |
|---|---|---|---|
| Single-source caption layout | `managers/captions/mod.rs:12` `build_blocks` | shipped | Preview overlay + ASS burn-in |
| Bundled font registry | `managers/captions/fonts.rs` | shipped | Font family selector |
| ASS subtitle generation | `managers/captions/ass.rs` | shipped | Export burn-in path |
| Live preview overlay | `components/player/CaptionOverlay.tsx` | shipped | Player |
| Settings preview pane | `components/settings/CaptionSettings.tsx` (per `caption-settings-preview` bundle) | shipped (in `reviewing`) | Caption Settings |
| SRT / VTT sidecar export | `managers/export.rs` + `commands/export.rs` | shipped | Editor toolbar |
| Plain-text "script" export | `managers/export.rs ExportFormat::Script` | shipped | Editor toolbar |
| ASS / SSA sidecar export | n/a | missing | None |
| Caption font/family/radius/padding/max-width settings | `AppSettings` + `components/settings/CaptionSettings.tsx:316-522` | partial | UI sliders update local store; **no backend command handler** for 5 keys (see §3) |

### Export / FFmpeg pipeline

| Capability | Source of truth | State | User-facing surface |
|---|---|---|---|
| FFmpeg-driven preview render with cache | `commands/waveform/mod.rs render_temp_preview_audio`, `preview_cache.rs` | shipped | "Preview edits" toggle |
| FFmpeg-driven edited media export (mp4 H.264 + AAC) | `commands/waveform/mod.rs export_edited_media:562-687` | shipped | Editor "Export" |
| Multi-segment `filter_complex` (trim/atrim/setpts/asetpts/concat) | `commands/waveform/mod.rs:106-279` | shipped | Implicit |
| Seam fades (symmetric 20 ms; 2 ms first-boundary) | `commands/waveform/mod.rs:111-117` + `audio-boundary-eval` | shipped | Implicit |
| Silence gates (`volume=enable='between(t,S,E)':volume=0`) | `commands/waveform/mod.rs:279` | shipped | Implicit |
| Manual export volume dB + global fade in/out | `AppSettings.export_volume_db,export_fade_in_ms,export_fade_out_ms` | shipped | Settings (deep) |
| Loudness normalization (`loudnorm=I=-16:TP=-1.5:LRA=11`) | `commands/waveform/mod.rs:121` gated by `normalize_audio_on_export` | partial | Toggle exists; no measurement / preflight feedback |
| Subtitle burn-in (`subtitles=...` with `fontsdir`) | `commands/waveform/mod.rs:612,656` | shipped | Export-with-captions toggle |
| Zero-crossing energy-biased segment snap | `managers/splice/boundaries.rs snap_segments_energy_biased:118` (called at `commands/waveform/mod.rs:444`) | shipped | Implicit (preview + export parity) |
| EBU R128 deterministic LUFS measurement | `managers/splice/loudness.rs:40` | dead-code-still-present | None — pure infra |
| `experimental_simplify_mode` flag | `settings/types.rs:250` + `commands/waveform/mod.rs:115,143,218,235,393` | undocumented | No UI; gates legacy keep-segment selection |
| Debug FFmpeg edit script generator | `commands/waveform/mod.rs generate_ffmpeg_edit_script` | undocumented | "Generate FFmpeg script" button (debug) |
| Output containers other than mp4 | n/a | missing | None |
| Hardware encoders (NVENC / QSV / VideoToolbox) | n/a | missing | None |
| Audio-only export (mp3 / wav / m4a / opus / flac) | n/a | missing | None — codec wired for AAC only |
| Aspect-ratio reframe / crop (9:16, 1:1) | n/a | missing | None |
| Speed / pitch-preserving time stretch (`atempo`) | n/a | missing | None |
| Per-segment volume / ducking | n/a | missing | None |
| Chapter markers / poster frame / thumbnail | n/a | missing | None |

### Project / persistence

| Capability | Source of truth | State | User-facing surface |
|---|---|---|---|
| `.toaster` JSON project save/load | `managers/project.rs` + `commands/project.rs` | shipped | Editor save/load |
| Auto-save every 30 s | `components/editor/EditorView.tsx` | shipped | Implicit |
| Portable mode (data in `./Data/`) | `portable.rs` | shipped (undocumented in README) | Marker file beside exe |
| Recordings folder + log dir openers | `commands/app_settings.rs` | shipped | About panel |
| Settings sanitizer (loopback-only LLM URL) | `settings/sanitize.rs` + `llm_client.rs is_local_host` | shipped | Defense-in-depth |
| Lock-poison recovery | `lock_recovery.rs` | shipped | Implicit |
| Project history list | n/a (deleted by `remove-history-and-legacy`) | shipped (deletion in `reviewing`) | Removed |

### UX / shell

| Capability | Source of truth | State | User-facing surface |
|---|---|---|---|
| Sidebar navigation: Editor / Models / About | `components/Sidebar.tsx:24-43` | shipped | Persistent left rail |
| Onboarding (download first model) | `components/onboarding/Onboarding.tsx` | shipped | First run |
| Update checker | `components/update-checker/UpdateChecker.tsx` | shipped | Footer |
| Model selector in footer | `components/model-selector/ModelSelector.tsx` | shipped | Footer dropdown |
| 22-locale i18n with parity check | `src/i18n/locales/*` + `scripts/check-translations.ts` | shipped | App language selector |
| RTL language direction | `src/lib/utils/rtl.ts`, `App.tsx:36` | shipped | Implicit |
| Debug-mode toggle (Ctrl+Shift+D) | `App.tsx:67-76` | shipped | Hidden hotkey |
| 800-line file-size cap | `scripts/check-file-sizes.ts` + `scripts/file-size-allowlist.txt` | shipped | CI gate |
| Caption preview pane | `caption-settings-preview` bundle | shipped (in `reviewing`) | Caption Settings top |
| Brand wordmark sizing pass | `brand-title-sizing` bundle | shipped (in `reviewing`) | Sidebar |

### Build & ops

| Capability | Source of truth | State | User-facing surface |
|---|---|---|---|
| Windows monitored launcher | `scripts/launch-toaster-monitored.ps1` | shipped | Dev only |
| Windows env preflight (`Platform`, `CMAKE_GENERATOR_*` strip) | `scripts/setup-env.ps1`, `scripts/migrate/check-cmake-ninja-env.ps1` (per `build-env-ninja-hardening` planned) | partial | Dev only |
| `whisper-rs-sys` cache wipe helper | `scripts/dev/clean-whisper-cache.ps1` | shipped | Dev only |
| Eval harness orchestrator | `scripts/eval/run-eval-harness.ps1` + `eval-harness-runner` agent | shipped | CI |
| Precision / boundary / parity evals | `scripts/eval/eval-edit-quality.ps1`, `eval-audio-boundary.ps1`, `eval-multi-backend-parity.ps1` | shipped | CI / agents |
| Cleanup / disfluency / captions verifier scripts | `scripts/eval/eval-verifier-*.ps1` | shipped | CI / agents |
| Local LLM rollout gate | `scripts/eval/run-local-llm-eval-gate.ps1` | shipped (undocumented in README) | Dev only |
| Feature board (terminal Kanban over `STATE.md`) | `scripts/feature/feature-board.ps1` | shipped | Dev only |
| Coverage gate | `scripts/feature/check-feature-coverage.ps1` | shipped | CI |
| Translation parity gate | `scripts/check-translations.ts` | shipped | CI |
| Nix flake / module variants | `flake.nix`, `nix/`, `.nix/` | partial | Linux power users; no docs in README |
| Code-signed Windows installer | `tauri.conf.json signCommand=""` (`docs/build.md:138-150`) | missing | Unsigned → SmartScreen warning |
| macOS / Linux installers | n/a | partial | `cargo tauri build` works; no docs verifying first-run |

### Acceptance criteria

- AC-001-a — §2 lists ≥ 30 capabilities grouped under all 7 domains
  (Transcription / Editing / Captions / Export / Project / UX / Build)
  with state and source-of-truth file paths.

---

## 3. Undocumented or under-documented capabilities

Each item below is present in code but absent from README, settings
labels, or the running UI. Items are evidence for §4 gaps and §6
roadmap items, not bugs to silently fix.

1. **`experimental_simplify_mode` settings flag** —
   `src-tauri/src/settings/types.rs:250`; consumed at
   `src-tauri/src/commands/waveform/mod.rs:115,143,218,235,393`.
   Never surfaced in any settings panel. Survived the recent
   `remove-history-and-legacy` purge because the waveform path still
   reads it. Either delete or expose as a documented advanced toggle.

2. **`splice::loudness` EBU R128 measurement** —
   `src-tauri/src/managers/splice/loudness.rs:40 measure_loudness`,
   `:7 target_gain_db`. Returns LUFS + true-peak; explicitly
   designed as an alternative to the FFmpeg `loudnorm` filter, but
   no caller uses it. Wiring it in pre-export would let us warn on
   clipping or wildly off-target loudness without re-encoding.

3. **`splice::clarity` spectral analyzer** —
   `src-tauri/src/managers/splice/clarity.rs:60 analyze`. Used
   inside `managers/disfluency.rs` for survivor scoring, never
   surfaced as a confidence/clarity heat-map in the editor.
   Candidate for the §6 "show me where audio is weak" UX.

4. **Unreachable post-processing settings UI** — translation
   namespace `settings.postProcessing` plus full component tree
   under `src/components/settings/post-processing/` and
   `post-processing-api/` (provider select, API key field, base URL
   field, model select, prompt CRUD). Not in
   `src/components/Sidebar.tsx:24-43` `SECTIONS_CONFIG`. Backend
   commands exist (`addPostProcessPrompt`, `setPostProcessProvider`,
   etc.). Either restore the panel or delete the surface.

5. **Unreachable debug settings panel** — components under
   `src/components/settings/debug/` (`DebugPaths`,
   `LogLevelSelector`, `WordCorrectionThreshold`) plus
   `sidebar.debug` translation key. Only `LogDirectory` was
   relocated; the rest are dead until mounted somewhere.

6. **`sidebar.general` translation key** — referenced in 22 locale
   files; no component matches it. Pure orphan.

7. **5 caption styling settings without backend handlers** —
   `caption_font_family`, `caption_radius_px`, `caption_padding_x_px`,
   `caption_padding_y_px`, `caption_max_width_percent` all read/write
   in `components/settings/CaptionSettings.tsx:316-522` and the
   layout engine consumes them, but `settingsStore.ts` lacks
   dedicated `change*Setting` commands for them. They appear to round
   trip through the generic `update_setting` path; the asymmetry is
   confusing and should be reconciled.

8. **`generateFfmpegEditScript` command** —
   `src-tauri/src/commands/waveform/mod.rs generate_ffmpeg_edit_script`,
   wired to a button in `components/editor/EditorToolbar.tsx`. Useful
   debug tool with zero documentation; either gate it behind
   debug-mode or document it.

9. **`overlay` i18n namespace** — present in all 22 locale files,
   referenced nowhere. Handy-era residue missed by
   `remove-history-and-legacy`.

10. **`AppSettings.normalize_audio_on_export` toggle** — exists in
    settings but there is no UI control for it, only the FFmpeg
    consumer at `commands/waveform/mod.rs:121`. Defaults silently.

11. **CLI flags on the launcher** — `src-tauri/src/cli.rs`
    `--start-hidden` and `--debug`. Not in README; only honored when
    the binary is invoked directly (not via `cargo tauri dev`).

12. **`scripts/eval/run-local-llm-eval-gate.ps1`** documented in
    `docs/build.md:84-103` but not linked from README; new
    contributors will not discover it.

13. **`scripts/dev/dump-debug-state.ps1` / `dump-caption-style.ps1`** —
    AGENTS.md mentions them, README does not.

14. **Portable mode** — `src-tauri/src/portable.rs`. Mentioned in
    `update-checker` ("special handling for portable builds"), absent
    from README and `docs/build.md`.

15. **`check_apple_intelligence_available()` stub** —
    `src-tauri/src/commands/app_settings.rs` returns `false`
    unconditionally; only consumer is unreachable post-processing
    UI. Schedule for deletion.

### Acceptance criteria

- AC-002-a — §3 lists ≥ 10 specific undocumented capabilities, each
  with file path (and line number where useful).

---

## 4. Gap analysis to v1.0 launch

Categories: **Blocker** = launch is irresponsible without it;
**Strongly Recommended** = will be the top complaint if absent;
**Nice-to-have** = differentiator, not table-stakes.

### Blockers

- **B1. Code-signed Windows installer.** `docs/build.md:131-150`
  documents what is required; `tauri.conf.json signCommand=""` ships
  unsigned. SmartScreen warning will sink first-run conversion.
- **B2. Cross-platform first-run verification.** No equivalent of the
  Windows monitored launcher for macOS/Linux; no documented
  reproduction of "open installer → onboard → transcribe → export"
  outside Windows-MSVC dev. `docs/build-macos.md` referenced in
  AGENTS.md `Repository layout` is missing from `docs/`
  (`Get-ChildItem c:\git\toaster\docs` shows only `build.md` and
  `testing-kb.md`).
- **B3. Remove unreachable UI surfaces.** Until the
  `sidebar.general/debug/postProcessing` tri-orphan, the
  `overlay` i18n namespace, and the `experimental_simplify_mode`
  flag are either restored or deleted, the app reads as half-built
  on inspection (translators see ghost keys; contributors find dead
  code). Extends the `remove-history-and-legacy` cleanup pass.
- **B4. Loudness preflight before export.** Today export silently
  applies (or doesn't apply) `loudnorm` based on a setting that has
  no UI. A user can ship a 28 LUFS clip with hard clipping and have
  no warning. We have measurement infra (`splice::loudness:40`)
  unused — wire it in.
- **B5. Audio-only export.** Podcasters are explicit target users
  (PRD.md target users include "Podcasters and educators"). They
  cannot export audio-only without losing the video container; the
  pipeline only emits H.264 mp4.

### Strongly Recommended

- **SR1. Hardware encoder selection with safe fallback.** Long-form
  H.264 export on CPU is slow on consumer hardware. NVENC / QSV /
  VideoToolbox via FFmpeg `-c:v` are deterministic to add behind a
  detect-then-fallback selector.
- **SR2. Format / container choice.** mp4/H.264 is the safe default;
  many users want mov (Final Cut), webm (web), m4a (audio-only).
- **SR3. Restore (or formally delete) post-processing UI.** Cleanup
  via local LLM is fully implemented in the backend
  (`managers/cleanup/`, `llm_client.rs` with loopback enforcement).
  Hiding the only entrypoint wastes the work. Either restore the
  Sidebar route or delete the entire surface.
- **SR4. Per-clip / per-segment volume + ducking.** The export
  pipeline already applies a global `volume=` filter; per-segment
  volume gating is the same primitive scoped to time ranges. Useful
  for "boost this quiet question" without re-encoding outside
  Toaster.
- **SR5. Subtitle sidecar formats: ASS export.** ASS is generated for
  burn-in (`captions/ass.rs`) but never offered as an export. Two
  lines of UI.
- **SR6. Speed / pitch-preserving time stretch (`atempo` filter
  chain).** Highly requested for tutorial / talking-head cleanup.
  FFmpeg-only, single setting.
- **SR7. Documentation: README links to all the eval scripts and the
  monitored launcher.** Today they are AGENTS.md-only.
- **SR8. Discoverable keyboard shortcut map.** Editor uses many
  hotkeys (delete, silence, undo, redo, find, debug toggle); no
  surface lists them.

### Nice-to-have

- **NH1. Loudness / clipping warnings inline in editor (live
  `ebur128` panel).** Differentiator vs. simple editors.
- **NH2. Aspect-ratio reframe (9:16 / 1:1) for social.** Opt-in.
- **NH3. Chapter markers from transcript headings.**
- **NH4. Poster-frame / thumbnail generation on export.**
- **NH5. Crossfades at edit seams (in addition to today's symmetric
  fades).** Marginal quality lift; risk of over-engineering the
  splice policy.
- **NH6. Multi-backend ASR exercised by parity eval.** Today only
  Whisper is gate-tested even though six other adapters exist
  (`adapter.rs`).
- **NH7. Confidence / clarity heat-map in transcript using
  `splice::clarity`.**

### Acceptance criteria

- AC-003-a — §4 categorizes every gap as one of {Blocker, Strongly
  Recommended, Nice-to-have} and assigns each gap a stable ID
  (B#, SR#, NH#).

---

## 5. FFmpeg opportunity map

Constraint: every item is local-only (FFmpeg is bundled; no network).
Complexity = engineering days at current velocity. Risk to "simplicity
first" = how much UI / settings surface this adds. **Recommend** is the
discovery call; the human can override in §8.

| # | Capability | Value | Cmplx | UX risk | Recommend | Reasoning |
|---|---|---|---|---|---|---|
| F1 | EBU R128 loudness normalization on export (already toggled, plus preflight measure) | Clean, broadcast-friendly output without per-file knob-twiddling | S | Low | **Include — Foundation** | Filter already wired (`waveform/mod.rs:121`); add UI toggle (label "Normalize loudness to broadcast level (-16 LUFS)") + preflight using `splice::loudness:40` |
| F2 | Per-clip / seam crossfades (extend current 20 ms symmetric fade) | Slightly smoother seams on hard cuts | M | Med | **Defer** | Today's symmetric 20 ms fades + zero-crossing snap + energy-bias already pass `audio-boundary-eval`. Crossfade adds a parameter without obvious win; revisit if seam complaints appear |
| F3 | Burn-in caption parity vs. preview (single-source-of-truth audit) | Eliminates caption preview↔export drift class of bug | S | Low | **Include — Foundation (audit task)** | Already enforced by `captions::build_blocks` consumed by both paths. Roadmap item is a verification eval (`captions-parity-eval`) not new code |
| F4a | Container/codec: mov H.264/ProRes-Proxy | Final Cut / Premiere import friendly | S | Low | **Include — Polish** | One CLI flag swap; safer to start without ProRes (license-free codec only) |
| F4b | Container/codec: webm VP9/Opus | Web-native, no licensing burden | M | Low | **Defer — post-launch** | Slower encode than H.264; user signal needed |
| F4c | Audio-only mp3 / wav / m4a / opus / flac | Podcasters can stop round-tripping through Audacity | S | Low | **Include — Foundation** | Reuse current audio filter chain, drop video stream; presets in a dropdown |
| F5 | Hardware encoder (NVENC / QuickSync / VideoToolbox) with CPU fallback | 3-10x faster long-form export | M | Low | **Include — Polish** | FFmpeg detection at startup (`-encoders`); preference is "auto / cpu / gpu"; falls back on encoder error |
| F6 | Aspect-ratio reframe (9:16 / 1:1 with safe-area crop) | Social-media reuse without leaving the app | M | Med | **Defer — post-launch** | UX risk: needs manual crop alignment to avoid talking-head clipping; ship after seeing real demand |
| F7 | Speed / pitch-preserving time stretch (`atempo`) | Speed up "uhhh" sections to ~1.2x without chipmunking | S | Low | **Include — Polish** | Per-segment scope reuses keep-segments; backend authority preserved. Single number control (1.0x–2.0x) |
| F8 | Per-segment volume / ducking | Quiet-question rescue, music-bed ducking | M | Med | **Defer** | Adds a per-word/per-range setting; risk of overcomplicating editor unless paired with strong UX |
| F9 | Chapter markers from transcript headings | YouTube/podcast chapter timestamps | S | Low | **Include — Polish** | Map paragraph breaks (or user-marked headings) to mp4 chapter atoms / WebVTT chapters |
| F10 | Thumbnail / poster frame generation | Auto-poster for mp4 thumbnail; YouTube preview | S | Low | **Include — Launch-Ready** | Single `-ss N -frames:v 1` extract; pick a frame the user likes from the timeline |
| F11 | Subtitle sidecar export: SRT / VTT (already shipped), add ASS | Pro caption editing in Aegisub | S | Low | **Include — Polish** | `captions/ass.rs` already produces ASS for burn-in; expose a "Save .ass" button |
| F12 | Live `ebur128` panel in editor (analysis only, no re-encode) | Differentiator: see loudness curve and clipping in real time | L | Med | **Defer — post-launch** | Requires a streaming audio-analysis path on the live preview; nice but not a launch blocker |

### Rejected outright

- **None of the above require network access.** Anything that did
  would be auto-rejected per AGENTS.md `Non-negotiable boundaries:
  Local-only inference` (`AGENTS.md:90`).

### Acceptance criteria

- AC-004-a — §5 evaluates every one of the 12 focus areas listed in
  the original brief (loudnorm, crossfades, caption parity, formats,
  hardware encoders, aspect crops, time stretch, per-segment volume,
  chapter markers, thumbnails, sidecar formats, ebur128 panel) with
  one-line value, complexity (S/M/L), risk (Low/Med/High), and an
  Include / Defer / Reject recommendation with reasoning.

---

## 6. Proposed roadmap to v1.0

Sequence within each milestone is the suggested execution order; each
roadmap item will be re-scaffolded as its own `features/<slug>/`
bundle by the human via `pwsh scripts/feature/scaffold-feature.ps1 -Slug
<slug>` then `feature-pm`.

### Milestone 1 — Foundation (clean house, fix asymmetries)

**Closes:** B3, B4, B5, plus parts of SR3, SR7. **Includes FFmpeg:**
F1, F3, F4c.

| Order | Slug (proposed) | One-line scope |
|---|---|---|
| 1.1 | `unreachable-surface-purge` | Delete `sidebar.general`, `sidebar.debug`, `sidebar.postProcessing` orphans, `overlay` i18n namespace, `check_apple_intelligence_available` stub, `experimental_simplify_mode` flag — OR re-mount survivors. Decision per Risk §8 Q1. |
| 1.2 | `caption-settings-handlers` | Add backend command handlers for the 5 caption styling keys; resolve the asymmetry between caption settings that have dedicated commands and the 5 that don't. Re-checks dual-path SST. |
| 1.3 | `loudness-preflight` | Wire `splice::loudness::measure_loudness` (`loudness.rs:40`) into export preflight; surface a "Normalize loudness" toggle with a preflight readout (current LUFS, target LUFS, true-peak). |
| 1.4 | `audio-only-export` | mp3 / wav / m4a / opus presets in the export dialog; reuse current audio filter chain, drop `:v` stream. |
| 1.5 | `caption-parity-eval` | Standing eval (`scripts/eval/eval-verifier-captions.ps1` extension) that asserts preview-rendered caption block geometry equals ASS-burn output to within 1 px / 1 frame. Catches the next caption regression before it ships. |

**Exit criteria:**
- All §3 items 4-6 and 9 (orphan UI / namespace) are either gone or
  reachable from the sidebar.
- `export_edited_media` accepts an output container choice;
  audio-only presets work end-to-end (`launch-toaster-monitored.ps1`
  + manual export of `eval/fixtures/toaster_example.mp4` to all four
  audio presets).
- `audio-boundary-eval` and `transcript-precision-eval` still pass.

### Milestone 2 — Polish (the export experience users will demo)

**Closes:** SR1, SR2, SR4 (deferred), SR5, SR6, plus parts of SR8.
**Includes FFmpeg:** F4a, F5, F7, F9, F11.

| Order | Slug (proposed) | One-line scope |
|---|---|---|
| 2.1 | `export-format-mov` | Add mov container option (H.264 inside mov); same codec, different muxer flag. |
| 2.2 | `hardware-encoder-fallback` | Detect NVENC / QSV / VideoToolbox at startup; expose "Auto / CPU / GPU" preference; on encoder failure, automatically retry with `libx264`. |
| 2.3 | `time-stretch-segments` | Per-keep-segment `atempo` factor; backend authority on the segment list; preview must use identical filter chain. |
| 2.4 | `chapter-markers` | Detect transcript paragraph breaks (gap > N seconds) or user-tagged headings; embed as mp4 chapter atoms + WebVTT chapter sidecar. |
| 2.5 | `ass-sidecar-export` | "Save as .ass" button in EditorToolbar; reuses `captions::ass`. |
| 2.6 | `keyboard-shortcuts-cheatsheet` | "?" hotkey opens a modal listing every editor shortcut; sourced from a single hotkey table. |
| 2.7 | `restore-or-delete-post-processing-ui` | Decision per §8 Q3. If restore: re-mount under `sidebar.postProcessing` and document loopback-only enforcement. If delete: rip the component tree + i18n keys + backend commands per `dep-hygiene`. |

**Exit criteria:**
- Export dialog offers ≥ 4 output choices (mp4 H.264 / mov H.264 /
  audio-only mp3 / audio-only m4a) and resolves correctly with
  hardware encoder when available, CPU when not.
- `cut-drift-fuzzer` passes after time-stretch lands.
- All sidebar entries either go somewhere or are deleted.

### Milestone 3 — Launch-Ready (ship the installer)

**Closes:** B1, B2, SR7. **Includes FFmpeg:** F10.

| Order | Slug (proposed) | One-line scope |
|---|---|---|
| 3.1 | `windows-code-signing` | Wire `signCommand` per `docs/build.md:138-150`; add CI secrets; verify SmartScreen behavior on a clean VM. |
| 3.2 | `macos-build-verify` | Restore the missing `docs/build-macos.md` referenced in AGENTS.md `Repository layout`. Verify `cargo tauri build` on macOS produces a notarizable bundle; document NSPanel / private API gotchas. |
| 3.3 | `linux-build-verify` | Verify Nix flake + standard Tauri Linux bundle on a clean Linux box; add troubleshooting section to `docs/build.md`. |
| 3.4 | `readme-launch-pass` | README links every eval script, the monitored launcher, the local-LLM gate, the dump-debug helpers, and portable-mode. Aligns with `WS3` in PRD.md. |
| 3.5 | `poster-frame-export` | Pick-a-frame UI for thumbnail; embed in mp4. |
| 3.6 | `first-run-smoke-eval` | New eval script that spins up a clean profile, runs onboarding → transcribe a 60s fixture → cleanup → export, and asserts no panic / no orphan dialog / non-zero output. Cross-platform. |

**Exit criteria:**
- Three platform installers exist and have been hand-verified by a
  user other than the implementer.
- README onboarding path matches reality on all three platforms.
- `first-run-smoke-eval` is part of CI.

### Acceptance criteria

- AC-005-a — §6 defines exactly 3 milestones, each with: gaps closed
  (citing §4 IDs), FFmpeg items included (citing §5 IDs), an ordered
  list of slugs, and explicit exit criteria.

---

## 7. Anti-scope (explicitly NOT for v1.0)

| # | Excluded | AGENTS.md justification |
|---|---|---|
| AS1 | Real-time dictation surface (push-to-talk / overlay / tray) | `AGENTS.md:202` `handy-legacy-pruning` skill: Handy-era dictation modules are off the transcript-editor path; extending them is forbidden without justification. |
| AS2 | Hosted transcription / cleanup APIs (anything that talks to a non-loopback host) | `AGENTS.md:90` `Local-only inference`: "Toaster performs all transcription and cleanup locally. No runtime network calls to hosted LLM/transcription/caption APIs." |
| AS3 | Multi-user collaboration / cloud sync | `AGENTS.md:90` (network) + `AGENTS.md:13` `Local-first by default` (PRD principle 1). |
| AS4 | Frontend-owned keep-segment / time-mapping / caption layout (any duplication of backend logic) | `AGENTS.md:89` `Single source of truth for dual-path logic`. |
| AS5 | Swapping the video element source to an audio preview file | `AGENTS.md:88` literal rule. |
| AS6 | New 800-line+ source files in `src/` or `src-tauri/src/` outside the allowlist | `AGENTS.md:141` `File-size cap`. |
| AS7 | Apple-Intelligence cleanup / Apple-only ASR routing | Removed in `remove-history-and-legacy` per `handy-legacy-pruning`; `commands/app_settings.rs check_apple_intelligence_available` returns `false`. Re-introducing it would re-open the Handy surface. |
| AS8 | Streaming / live-recording dictation features | `PRD.md:5-9` — Toaster's product vision is file-based ("Open media → transcribe → edit → preview → export"). Streaming is out of scope. |
| AS9 | Crossfades / per-segment volume / aspect reframe / live ebur128 panel (F2 / F8 / F6 / F12 above) | Deferred per §5; revisit post-launch only on user signal. |

### Acceptance criteria

- AC-006-a — §7 lists ≥ 6 explicit exclusions, each citing the
  AGENTS.md (or PRD.md) line that justifies the exclusion.

---

## 8. Risks & open questions for the human

These are decisions the human needs to make **before** any §6 roadmap
item is scaffolded. Numbering is stable so future bundles can cite
them.

1. **Q1 — `experimental_simplify_mode`: keep, surface, or delete?**
   The flag exists at `settings/types.rs:250` and is consumed by the
   waveform export path. There is no UI. Three options: (a) document
   + expose as an Advanced toggle, (b) wire on permanently and
   delete the flag, (c) wire off permanently and delete the flag.
   Affects roadmap item 1.1.

2. **Q2 — `splice::loudness` integration: preflight only, or also
   replace the current FFmpeg `loudnorm` filter?** `loudness.rs`
   measurement is deterministic; replacing the FFmpeg filter would
   give us identical preview/export numbers. But `loudnorm` is the
   industry-standard implementation and we currently match it.
   Recommend preflight-only. Affects roadmap item 1.3.

3. **Q3 — Restore or delete the post-processing UI?** Backend
   cleanup pipeline is fully functional and gated to loopback. UI
   tree is complete but unmounted. Decision drives roadmap item 2.7.
   If restored, must label "Local LLM only — endpoints must be
   loopback (127.0.0.1 / localhost / ::1)" per `llm_client.rs
   is_local_host`.

4. **Q4 — Hardware encoder default policy.** Should "Auto" prefer
   GPU when available (faster, occasional artifacts on older NVENC
   silicon), or default to CPU (slower, deterministic)? Affects
   roadmap item 2.2.

5. **Q5 — Container set for v1.0 export dropdown.** Confirm the
   minimum set is {mp4 H.264, mov H.264, audio mp3, audio m4a}.
   Adding more (webm, opus, flac) increases the testing surface.

6. **Q6 — Code-signing certificate type.** EV (immediate
   SmartScreen pass, $$$) vs. OV (cheaper, builds reputation over
   time). `docs/build.md:139-141` already enumerates the trade-off.
   Affects roadmap item 3.1.

7. **Q7 — macOS / Linux packaging owner.** B2 requires hands-on
   verification on hardware not currently present in dev. Who runs
   the smoke test on each OS?

8. **Q8 — `caption-settings-handlers` migration: typed commands or
   single generic setter?** The 5 unhandled caption keys round-trip
   today via a generic path. Either add 5 dedicated commands (matches
   the rest of CaptionSettings) or migrate ALL caption settings to
   the generic path. Pick one for symmetry. Affects roadmap item 1.2.

9. **Q9 — Coverage gate exemption for planning-only bundles.** This
   bundle uses `manual` verifiers that point back at the PRD. The
   gate accepts that, but it conflates "the doc says what it
   should" with real verification. Should
   `scripts/feature/check-feature-coverage.ps1` learn a `kind: doc-section`
   or `--planning-only` mode? Documented in `journal.md` as a
   proposed amendment, not made.

10. **Q10 — Multi-backend ASR exposure.** Six adapters
    (Parakeet/Moonshine/SenseVoice/GigaAM/Canary/Cohere) exist but
    only Whisper is gate-tested. For v1.0, do we (a) keep them
    selectable but unsupported, (b) hide them behind debug-mode, or
    (c) extend `eval-multi-backend-parity.ps1` to gate-test all
    seven? Affects model-selector polish in Milestone 3.

### Acceptance criteria

- AC-007-a — §8 lists at least 8 numbered open questions, each
  framed as a binary or short-list decision the human must resolve
  before the matching roadmap item is scaffolded.

---

## 9. Coverage hint forward map (informational)

For each roadmap item proposed in §6, the verifier kind that would
gate its eventual feature bundle. This is not the bundle's own
coverage — that lives in `coverage.json` — it is the forward-looking
hint that proves every roadmap slug has a viable verification path.

See `coverage.json` for the literal AC↔verifier map of THIS planning
bundle. The forward-looking hints are listed there under the
`roadmap_hints` extension key (read-only metadata; the gate ignores
unknown keys).

### Acceptance criteria

- AC-008-a — Every roadmap item in §6 has a forward-looking verifier
  hint in `coverage.json` `roadmap_hints` (skill / agent / script /
  cargo-test / manual). Hints may name "to-be-created when feature is
  scaffolded" if no current verifier fits, but must say so explicitly.

---

## Edge cases & constraints

- Coverage gate accepts `manual` ACs only when they include both
  `command` and a non-empty `steps` array. Each AC verifier in
  `coverage.json` complies.
- Roadmap items deliberately do NOT spawn `tasks/<id>/context.md`
  files — those will be created when each roadmap slug is scaffolded
  individually. `tasks.sql` is intentionally empty.

## Data model

N/A.

## Non-functional requirements

- The PRD must remain ≤ 800 lines (per `AGENTS.md:141` file-size
  cap convention; planning artifacts not technically gated but the
  cap is the project norm).
- All file references must be ASCII paths with optional `:line`
  suffix.
- No smart quotes; ASCII-only punctuation.
