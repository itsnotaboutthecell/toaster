# Toaster - Product Requirements Document

## 1. Product Vision

**Toaster** is a local-first desktop application for transcript-driven editing of spoken video and audio. Users open a recording, generate a word-level transcript, and edit the media by editing text. The product should feel close to **OBS Studio** in architecture and workspace behavior, and closer to **Audiate** in day-to-day editing flow.

### Core Principle

> "Edit spoken media by editing text, with native desktop speed and OBS-style docked workflow."

### Product Stance

1. **Local-first.** No cloud service required for core editing.
2. **Deterministic cleanup.** Filler detection, pause handling, silence, and restore work without LLMs.
3. **OBS-like and compliant.** Native menu bar, dockable panels, reusable shared core, and safe OBS host integration.
4. **Non-destructive first.** Users can delete, silence, shorten, split, and restore without losing original media.
5. **One shared engine.** Standalone app and OBS host integration consume the same C API.

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
| 1 | ⬜ | Open audio or video file and preview it locally | Needs FFmpeg decoder plugin |
| 2 | ⬜ | Transcribe media into word-level text with timestamps | Needs ASR plugin |
| 3 | ✅ | Click transcript words to seek playback | |
| 4 | ✅ | Select transcript words and Delete, Silence, or Restore them | |
| 5 | ✅ | Save/load a project file that preserves transcript, edits, and settings | |
| 6 | ⬜ | Export cleaned media to a new file | Needs FFmpeg exporter plugin |
| 7 | ✅ | Export SRT/VTT captions and plain-text script | |
| 8 | ⚠️ | Search transcript and run find/replace | Find-next works; find-and-replace not wired |
| 9 | ⚠️ | Show waveform and keep transcript/playhead/waveform selection in sync | Display works; waveform generation needs media loading |
| 10 | ⬜ | Support undo/redo for all edit actions | No edit history API in libtoaster |

### P2 - Suggested Edits Without LLM (Phase 2 - Deterministic Cleanup)

| # | Status | Feature | Notes |
|---|--------|---------|-------|
| 1 | ✅ | Detect filler words from built-in dictionaries and repeated-word heuristics | |
| 2 | ⚠️ | Let users Silence/Delete Filler Words in batch | API works; batch UI not built |
| 3 | ⚠️ | Detect pauses using audio energy and duration thresholds | Gap-based detection works; no RMS/energy analysis yet |
| 4 | ⚠️ | Let users Silence/Delete/Shorten Pauses in batch | API works; batch UI not built |
| 5 | ⬜ | Support custom filler lists and ignore lists | Static dictionaries only |
| 6 | ✅ | Support restore markers for deleted spans | |
| 7 | ⬜ | Support transcript-only correction mode for captions/scripts | |

### P3 - Precision Editing (Phase 3 - Polish and Hardening)

| # | Status | Feature |
|---|--------|---------|
| 1 | ⬜ | Split word at playhead |
| 2 | ⬜ | Edit transcription timing with handles |
| 3 | ⬜ | Drag boundary markers |
| 4 | ⬜ | Roll adjacent word boundaries |
| 5 | ⬜ | Ripple edit selected spans |
| 6 | ⬜ | Snap to zero crossings, nearby words, and optional time grid |
| 7 | ⬜ | Apply seam smoothing on edited joins during export |
| 8 | ⬜ | Support keyboard nudge for fine timing control |

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
| Qt 6 Widgets | native desktop UI and docks |
| CMake | build |
| Local ASR engine plugin | transcription without Python service |
| OBS Studio SDK | OBS host integration (Phase 4) |

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
| ⬜ | Confirm end-to-end media load, transcription import/transcribe flow, and export-path ownership | Needs FFmpeg decoder + ASR plugin |

**Parallel workstreams after the foundation is stable**

| Status | Track | Notes |
|--------|-------|-------|
| ✅ | **Core/session:** transcript state transitions, project persistence polish, keep-segment correctness | |
| ⚠️ | **Media/export:** preview reliability, waveform loading, export correctness, caption/script outputs | Caption/script done; media load/export needs plugins |
| ⚠️ | **Frontend workflow:** selection/edit actions, inspector clarity, transcript search/navigation, dock cohesion | Selection/edit/inspector done; find-replace partial |
| ⚠️ | **Quality:** test coverage, automation smoke flow, and repeatable Windows build/run validation | Core tests pass; frontend e2e blocked on media loading |

**Phase exit:** the Windows standalone app can reliably perform the P1 workflow without timeline-first editing.

### Phase 2 - Deterministic Cleanup and Guided Editing

**Goal:** deliver the offline cleanup workflow that makes the product meaningfully better than
manual editing (all P2 items).

**Workstreams**

| Status | Track | Notes |
|--------|-------|-------|
| ⚠️ | **Filler engine:** dictionaries, repeated-word handling, ignore lists, review reasons | Detection done; custom lists not started |
| ⚠️ | **Pause engine:** move from gap-only toward audio-energy-aware detection | Gap-based done; energy analysis not started |
| ⬜ | **Review UX:** suggestion-list quality, batch-apply flows, transcript-only correction mode | |
| ✅ | **Recovery:** restore markers, reversible cleanup behavior, regression cases | |

**Phase exit:** users can run deterministic filler and pause cleanup with reviewable batch actions and reversible results.

### Phase 3 - Precision Editing and Release Hardening

**Goal:** improve timing control, polish, and release confidence (all P3 items).

**Workstreams**

| Status | Track |
|--------|-------|
| ⬜ | Boundary editing, split/roll/ripple behavior, and timing repair |
| ⬜ | Seam smoothing and edited-join quality during export |
| ⬜ | Undo/redo depth, shortcut completeness, dock persistence, and workflow polish |
| ⬜ | Packaging, crash handling expectations, logging, and repeatable release validation |

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
