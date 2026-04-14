# Toaster - Product Requirements Document

## 1. Product Vision

**Toaster** is a local-first desktop application for transcript-driven editing of spoken video and audio. Users open a recording, generate a word-level transcript using offline AI models, and edit the media by editing text. The product prioritizes a clean, icon-driven interface inspired by **Handy** while maintaining a pure-C core (`libtoaster`) that can be consumed by any frontend.

### Core Principle

> "Edit spoken media by editing text, with native desktop speed and offline AI transcription."

### Product Stance

1. **Local-first.** No cloud service required for core editing or transcription.
2. **Deterministic cleanup.** Filler detection, pause handling, silence, and restore work without LLMs.
3. **Clean architecture.** Native UI with icon sidebar and panel layout; pure-C core with stable ABI.
4. **Non-destructive first.** Users can delete, silence, shorten, split, and restore without losing original media.
5. **Offline AI models.** Multiple whisper.cpp model sizes with a built-in model selector and download manager.
6. **One shared engine.** Standalone app and future integrations consume the same C API.

---

## 2. Competitive Direction

### Keep from CutScript

- Transcript-first editing
- Fast open -> transcribe -> edit -> export flow
- Word-level transcript with waveform and preview
- Project save/load
- Caption export and optional burn-in
- Show long-running pipeline state clearly during slow operations (loading, transcribing, exporting)
- Make export modes explicit: fast copy, full re-encode, sidecar captions, burn-in captions

### Learn from Handy

- Offline model selector with multiple size/accuracy tiers
- Download manager with progress bar and status pills (green/yellow/orange/red)
- Icon-driven sidebar navigation instead of deep menu trees
- Clean panel layout with warm, approachable visual design
- Mascot integration for personality and empty states

### Learn from Audiate

- Suggested Edits as first-class workflow
- Silence vs Delete as separate actions
- Pause shortening as batch operation
- Editing Mode vs Transcript/Caption Mode
- Waveform editor for precise timing
- Restore markers and reversible edits
- Fast keyboard-first cleanup

### Learn from OBS

- Native menu bar and dock system
- Layout persistence and dock reset/lock behavior
- Profiles and reusable presets
- Stable crash handling and safe-mode expectations
- Shared core used by both standalone shell and host/plugin integration

---

## 3. Target Users

- YouTube creators editing talking-head videos, tutorials, and explainers
- Podcast and interview producers cleaning speech-heavy audio/video
- OBS users who want transcript-aware cleanup and offline cut/export workflow
- Educators and trainers producing narrated lessons and demos

---

## 4. Platform and Language Strategy

### 4.1 Target Platforms

1. **Windows** first
2. **macOS** second
3. **Linux** third

### 4.2 Programming Languages and Runtime

| Layer | Choice | Why |
|---|---|---|
| Core edit/session/export engine | **C17** | Stable ABI, plugin-friendly, easy OBS/shared-library integration |
| Desktop frontend | **C++20 + Qt 6 Widgets** | Native desktop menus, docks, waveform/preview UI, OBS alignment |
| OBS host integration | **C++20 + Qt 6 + OBS frontend/plugin APIs** | Dock reuse, native plugin compliance |
| Media I/O | **FFmpeg** | Proven decode/encode path and broad format support |
| Local transcription plugin | **C/C++ local ASR plugin** | Avoid Python service dependency; allow whisper.cpp or equivalent |
| Build | **CMake** | Cross-platform and OBS-aligned |

### 4.3 Human Languages

#### Launch

- **UI language:** English
- **Transcript cleanup language:** English
- **Caption export language:** whatever transcription returns, but English-first QA

#### Expansion Packs

Recommended next spoken-language packs: Spanish, Portuguese (Brazil), French, German, Japanese.

Each language pack must include:

- ASR support
- filler dictionary
- ignore dictionary
- repeated-word rules
- punctuation/caption spacing rules

---

## 5. UX Layout

### 5.1 Primary Workspace

- **Center-left:** video preview or audio preview surface
- **Right:** transcript editor
- **Bottom:** waveform/timeline
- **Optional right dock rail:** suggested edits, inspector, export, project info

### 5.2 Editing Modes

1. **Edit Mode** — deleting words deletes media; silencing keeps duration but removes sound; pause shortening changes effective duration
2. **Transcript Mode** — edits transcript text for captions/script only; media timing unchanged
3. **Boundary Mode** — trim, roll, ripple, split, keyboard nudge, timing repair

---

## 6. Menu Bar and Dock Model

Toaster uses a native desktop menu bar with OBS-like structure.

### 6.1 Top-Level Menus

| Menu | Purpose | Required Entries |
|---|---|---|
| **File** | media/project lifecycle | New Project, Open Media, Open Project, Save Project, Save Project As, Import Transcript, Export Media, Export Captions, Export Script, Exit |
| **Edit** | direct editing actions | Undo, Redo, Delete, Silence, Unsilence, Restore, Split at Playhead, Edit Timing, Find, Find and Replace, Select All Fillers, Select All Pauses |
| **View** | interface visibility | Show/Hide Transcript, Waveform, Inspector, Suggested Edits, Status Bar, Always on Top, Reset UI, Zoom controls |
| **Docks** | OBS-style panel control | Transcript Dock, Preview Dock, Waveform Dock, Inspector Dock, Suggested Edits Dock, Export Dock, Logs Dock, Lock Docks, Reset Docks |
| **Project** | edit-session settings | Project Settings, Relink Media, Language, Caption Options, Rebuild Transcript Cache |
| **Profiles** | reusable presets | Cleanup Profiles, Export Profiles, Shortcut Profiles |
| **Tools** | analysis and utilities | Transcribe, Re-transcribe, Suggested Edits, Filler Dictionary, Pause Detection, Audio Cleanup, Batch Export, OBS Integration Tools |
| **Help** | support and diagnostics | Keyboard Shortcuts, Documentation, Release Notes, Logs, Crash Reports, Check for Updates, About |

### 6.2 Docking Rules

- Every major panel must be dockable.
- Dock visibility must be toggled from **Docks** menu.
- Layout must persist per user.
- Users must be able to **Reset Docks** and **Lock Docks**.

### 6.3 OBS Host Rules

When hosted inside OBS, all OBS-specific constraints are defined here:

- Toaster surfaces must appear as **dock(s)** and **tool actions**.
- Toaster must not replace OBS main window behavior.
- Destructive cut/ripple/export work must stay outside render/audio callbacks.
- Non-destructive tagging is allowed inside host; destructive cleanup/export is queued to offline workflow.
- OBS integration must be crash-safe and safe-mode-aware.
- The standalone app and OBS host both consume the same `libtoaster` C API with no divergence.

---

## 7. Core User Stories

Section 7 is the **single source of truth** for feature status. The implementation plan
(section 14) references these P-numbers rather than restating features.

**Usable standalone state** = all P1 items complete + P2 batch cleanup reviewable.

### P1 - Must Have (Phase 1 - Native Core)

| # | Status | Feature | Notes |
|---|--------|---------|-------|
| 1 | ✅ | Open audio or video file and preview it locally | Qt QMediaPlayer; plugin decode is a Phase 3+ concern |
| 2 | ✅ | Transcribe media into word-level text with timestamps | Via whisper-cli.exe; integrated ASR plugin is Phase 3+ |
| 3 | ✅ | Click transcript words to seek playback | |
| 4 | ✅ | Select transcript words and Delete, Silence, or Restore them | |
| 5 | ✅ | Save/load a project file that preserves transcript, edits, and settings | |
| 6 | ✅ | Export cleaned media to a new file | Via ffmpeg.exe; plugin export is Phase 3+ |
| 7 | ✅ | Export SRT/VTT captions and plain-text script | |
| 8 | ✅ | Search transcript and run find/replace | |
| 9 | ✅ | Show waveform and keep transcript/playhead/waveform selection in sync | Waveform generated via ffmpeg.exe |
| 10 | ✅ | Support undo/redo for all edit actions | Snapshot-based undo/redo with 64-deep history |

### P2 - Suggested Edits Without LLM (Phase 2 - Deterministic Cleanup)

| # | Status | Feature | Notes |
|---|--------|---------|-------|
| 1 | ✅ | Detect filler words from built-in dictionaries and repeated-word heuristics | |
| 2 | ✅ | Let users Silence/Delete Filler Words in batch | Checkable suggestion list with Delete/Silence choice |
| 3 | ✅ | Detect pauses using audio energy and duration thresholds | Gap-based detection implemented; RMS/energy analysis deferred |
| 4 | ✅ | Let users Silence/Delete/Shorten Pauses in batch | Configurable thresholds; checkable batch apply |
| 5 | ✅ | Support custom filler lists and ignore lists | Core API done; frontend configuration UI not built |
| 6 | ✅ | Support restore markers for deleted spans | |
| 7 | ⬜ | Support transcript-only correction mode for captions/scripts | |

### P3 - Precision Editing (Phase 3 - Polish and Hardening)

| # | Status | Feature |
|---|--------|---------|
| 1 | ✅ | Split word at playhead |
| 2 | ✅ | Edit transcription timing with handles | Drag handles on waveform word boundaries |
| 3 | ✅ | Drag boundary markers | Waveform shows draggable boundary markers with hit-test |
| 4 | ✅ | Roll adjacent word boundaries | Auto-detected when dragging shared boundary; C API + UI |
| 5 | ✅ | Ripple edit selected spans | Ctrl+Shift+Delete; shifts subsequent timestamps |
| 6 | ⚠️ | Snap to zero crossings, nearby words, and optional time grid | Word and grid snap done; zero-crossing requires audio sample data |
| 7 | ✅ | Apply seam smoothing on edited joins during export | 5ms fade-in/fade-out at segment boundaries |
| 8 | ✅ | Support keyboard nudge for fine timing control | Alt+[ / Alt+] nudge ±10ms |

### P4 - OBS Workflow (Phase 4 - OBS Integration)

| # | Status | Feature |
|---|--------|---------|
| 1 | ⬜ | Provide OBS dock with transcript, suggested edits, and project status |
| 2 | ⬜ | Allow safe non-destructive tagging inside OBS host |
| 3 | ⬜ | Queue destructive cleanup/export to offline workflow |
| 4 | ⬜ | Preserve OBS-friendly dock behavior and crash-safe startup |

---

## 8. Deterministic Cleanup Engine

### 8.1 Filler Detection

Inputs:

- transcript words
- timestamps
- per-word confidence
- optional speaker ID

Rules:

1. Exact filler lexicon
2. Phrase lexicon (`you know`, `kind of`, `sort of`)
3. Repeated-word detection (`I I I`, `the the`)
4. Sentence-initial soft fillers (`so`, `well`, `actually`) behind confidence rules
5. User ignore list
6. Per-language dictionaries

Outputs:

- suggested delete list
- suggested silence list
- reviewable reason tags

### 8.2 Pause Detection

Inputs:

- audio RMS / energy envelope
- optional breath/noise classifier
- transcript gap timing

Rules:

1. Configurable minimum pause duration
2. Configurable silence threshold
3. Preserve breaths when user wants natural cadence
4. Separate **silence** from **shorten**

Outputs:

- pause spans
- suggested shorten targets
- reviewable batch actions

---

## 9. Keyboard Model

Default shortcuts should be simple and editor-friendly:

- Space - Play/Pause
- J / K / L - Shuttle backward, pause, shuttle forward
- Left / Right - Seek small step
- Shift+Left / Shift+Right - Seek larger step
- Delete / Backspace - Delete selection
- Ctrl+Delete - Silence selection
- Ctrl+Z / Ctrl+Shift+Z - Undo / Redo
- Ctrl+F - Find
- Ctrl+Alt+F - Find and Replace
- Ctrl+T - Split word at playhead
- Ctrl+Shift+T - Edit transcription timing
- Ctrl+1 - Toggle waveform
- Ctrl+E - Export
- Alt+[ / Alt+] - Boundary nudge earlier/later

---

## 10. Architecture

```text
Standalone Qt App                OBS Host
-----------------                -----------------
Menu bar + docks                 OBS dock + tool entry
Preview + transcript             Transcript dock
Waveform + inspector             Non-destructive actions
        \                              /
         \                            /
          ---------- libtoaster ----------
          session model | edit engine
          filler rules  | pause detector
          project I/O   | exporter API
          local ASR     | plugin registry
                   |
             FFmpeg / local ASR / caption writer
```

### Architectural Rules

1. `libtoaster` must know nothing about Qt.
2. Frontend and OBS host both consume same C API.
3. Plugin ABI remains stable and native.

---

## 11. Data Model

Project file must preserve:

- source media path
- transcript words and segments
- speaker IDs when available
- deleted spans
- silenced spans
- pause edits
- split points
- boundary edits
- export settings
- selected language and cleanup profile
- dock/workspace layout metadata

---

## 12. Dependencies

| Dependency | Purpose |
|---|---|
| FFmpeg | decode, seek, export |
| Qt 6 Widgets | native desktop UI and panels |
| CMake | build |
| whisper.cpp | offline transcription (linked as C library) |
| WinHTTP | model downloads on Windows |

---

## 12.1 Offline Model Management

### Model Catalog

Toaster ships a hardcoded catalog of whisper.cpp GGML models hosted on HuggingFace:

| ID | Name | Size | Languages | Accuracy | Speed | Recommended |
|---|---|---|---|---|---|---|
| tiny.en | Whisper Tiny (English) | 75 MB | English | ★☆☆ | ★★★ | ✅ Quick drafts |
| small | Whisper Small | 465 MB | 99 | ★★☆ | ★★☆ | |
| medium-q4 | Whisper Medium (Q4) | 469 MB | 99 | ★★★ | ★★☆ | |
| turbo | Whisper Large v3 Turbo | 1549 MB | 99 | ★★★★ | ★★☆ | ✅ Production |
| large-q5 | Whisper Large v3 (Q5) | 1031 MB | 99 | ★★★★★ | ★☆☆ | |

### Download Flow

1. User selects model from Model Selector UI
2. WinHTTP streams the model file from HuggingFace with progress callbacks
3. File is written to `%APPDATA%/Toaster/models/{filename}.partial` during download
4. On completion, `.partial` is renamed to final filename
5. Download is cancellable at any point; partial file is cleaned up

### C API (`libtoaster`)

```c
toaster_model_catalog_count()          // Number of available models
toaster_model_catalog_get(index, &info) // Get model info by index
toaster_model_catalog_find(id, &info)  // Find model by ID
toaster_model_is_downloaded(id)        // Check if model file exists locally
toaster_model_get_active()             // Currently selected model ID
toaster_model_set_active(id)           // Change active model
toaster_model_get_path(id)             // Full path to downloaded model file
toaster_model_download(id, cb, data)   // Download model with progress callback
toaster_model_cancel_download()        // Cancel in-progress download
toaster_model_delete(id)               // Remove downloaded model file
```

### Word Metadata

Each word in the transcript carries:
- `float confidence` — transcription confidence (0.0–1.0, or -1.0 if unavailable)
- `int speaker_id` — speaker diarization label (-1 if unavailable)

These fields are saved/loaded in project files (backward-compatible with older files).

---

## 13. Non-Goals

These constraints apply globally. They are stated once here and not repeated elsewhere.

- No LLM-required filler detection
- No prompt-based clip suggestion in MVP
- No Electron frontend
- No Python backend service or child process
- No localhost HTTP bridge
- No mandatory cloud transcription or cloud API key management in core UX
- No unsafe live destructive edits on active OBS render/audio path

---

## 14. Implementation Plan

This section is the implementation-facing roadmap. Feature status is tracked in section 7;
this section covers sequencing, parallelization, and exit criteria per phase.

### Phase 1 - Usable Standalone Foundation

**Goal:** complete the minimum end-to-end standalone editing loop (all P1 items).

**Sequential foundation work**

| Status | Task | Notes |
|--------|------|-------|
| ✅ | Finalize transcript / project / export contract expectations in `libtoaster` | |
| ✅ | Confirm end-to-end media load, transcription import/transcribe flow, and export-path ownership | Works via QMediaPlayer + external ffmpeg/whisper tools |

**Parallel workstreams after the foundation is stable**

| Status | Track | Notes |
|--------|-------|-------|
| ✅ | **Core/session:** transcript state transitions, project persistence polish, keep-segment correctness | |
| ✅ | **Media/export:** preview reliability, waveform loading, export correctness, caption/script outputs | All working via QMediaPlayer + external tools |
| ✅ | **Frontend workflow:** selection/edit actions, inspector clarity, transcript search/navigation, dock cohesion | Selection/edit/search/replace/undo/redo all wired |
| ⚠️ | **Quality:** test coverage, automation smoke flow, and repeatable Windows build/run validation | Core tests pass (96/96); frontend e2e not automated |

**Phase exit:** the Windows standalone app can reliably perform the P1 workflow without timeline-first editing.

### Phase 2 - Deterministic Cleanup and Guided Editing

**Goal:** deliver the offline cleanup workflow that makes the product meaningfully better than
manual editing (all P2 items).

**Workstreams**

| Status | Track | Notes |
|--------|-------|-------|
| ✅ | **Filler engine:** dictionaries, repeated-word handling, ignore lists, review reasons | Detection, custom lists API, and batch UI done |
| ⚠️ | **Pause engine:** move from gap-only toward audio-energy-aware detection | Gap-based done; energy analysis not started |
| ⚠️ | **Review UX:** suggestion-list quality, batch-apply flows, transcript-only correction mode | Batch apply done; transcript-only mode not started |
| ✅ | **Recovery:** restore markers, reversible cleanup behavior, regression cases | |

**Phase exit:** users can run deterministic filler and pause cleanup with reviewable batch actions and reversible results.

### Phase 3 - Precision Editing and Release Hardening

**Goal:** improve timing control, polish, and release confidence (all P3 items).

**Workstreams**

| Status | Track |
|--------|-------|
| ✅ | Boundary editing, split/roll/ripple behavior, and timing repair |
| ✅ | Seam smoothing and edited-join quality during export |
| ✅ | Undo/redo depth, shortcut completeness, dock persistence, and workflow polish |
| ⚠️ | Packaging, crash handling expectations, logging, and repeatable release validation | Logging done; packaging/crash handling not started |

**Phase exit:** standalone Toaster feels coherent, recoverable, and ready for real-world speech-editing sessions.

### Phase 4 - OBS-safe Integration

**Goal:** reuse the shared core in OBS without weakening host safety (all P4 items).

**Workstreams**

| Status | Track |
|--------|-------|
| ⬜ | OBS dock/tool entry design |
| ⬜ | Non-destructive tagging inside host |
| ⬜ | Offline destructive cleanup/export handoff |
| ⬜ | Crash-safe and safe-mode-aware host behavior |
| ⬜ | Shared-core parity checks between standalone and OBS flows |

**Phase exit:** OBS integration adds tagging and handoff value without duplicating the edit core or moving destructive work into unsafe host paths.

### Parallelization Summary

| Workstream | Depends on | Can run in parallel with |
|---|---|---|
| Shared transcript/project/export contracts | none | little; this is the main sequential foundation |
| Standalone media/export pipeline | contract freeze | transcript UI and quality tracks |
| Transcript/editor UX | contract freeze | media/export and quality tracks |
| Deterministic cleanup engines | stable transcript/timing model | cleanup review UX and recovery work |
| Automation, regression, packaging | each active phase | almost all tracks, but it blocks phase exit |
| OBS integration | stable standalone product and export handoff model | limited parallelism; mostly late-phase sequential work |

---

## 15. Success Criteria

1. User can clean up a spoken clip without touching a traditional timeline for common edits.
2. Filler and pause cleanup works offline with no API keys and no prompt calls.
3. UI feels native and dockable, not browser-like.
4. Standalone app and OBS integration share one edit core.
5. Exported media, captions, and script remain synchronized after edits.
