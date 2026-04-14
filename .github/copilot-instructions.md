# Project Guidelines

Toaster is a text-based video/audio editor ("edit video by editing text"). A pure-C core engine (`libtoaster/`) with a Qt6 frontend (`frontend/`). See [PRD.md](../PRD.md) for full product requirements and phased roadmap.

## Architecture

```
libtoaster/          Pure C library — edit model, signals, analysis, project I/O
  toaster.h          Single public header (all API declarations)
  toaster.c          Core: transcript, undo/redo, split, startup/shutdown
  analysis.c         Filler detection, pause detection, suggestion generation
  project.c          JSON project save/load
  callback/
    callback.c       Signal handler implementation (connect/emit/disconnect)
  CMakeLists.txt     Builds shared library "toaster"

frontend/            Qt6/C++ GUI
  main.cpp           Entry point, calls toaster_startup()
  MainWindow.h/cpp   Monolithic UI: transcript table, playback, waveform, editing
  WaveformView.h/cpp Audio waveform display widget
  CMakeLists.txt     Builds "toaster-app" executable

test/                CLI test harness (no framework)
  test-edit.c        Edit model, undo/redo, split word
  test-signals.c     Signal/callback system
  test-analysis.c    Filler and pause detection
  test-project.c     Project save/load round-trip
  test-timeline.c    Keep-segment calculation after deletions
  test-export.c      Script, SRT, and VTT caption export

scripts/             PowerShell helpers for app launching and automation
```

**Hard boundary**: `libtoaster` has zero knowledge of Qt or any UI. The frontend is a consumer of the C API in `toaster.h`.

## Build and Test

Requires MSYS2 MinGW-w64 environment with Qt6.

```bash
export PATH="/c/Program Files/CMake/bin:/c/msys64/mingw64/bin:$PATH"
cd /c/git/toaster/build
cmake .. -G "MinGW Makefiles" -DCMAKE_PREFIX_PATH=/c/msys64/mingw64
mingw32-make -j4
```

Run tests: `./bin/test-edit.exe`, `./bin/test-signals.exe`, `./bin/test-analysis.exe`, `./bin/test-project.exe`, `./bin/test-timeline.exe`, `./bin/test-export.exe`
Run app: `./bin/toaster-app.exe`

**Gotcha**: Kill old `toaster-app.exe` before rebuilding — DLL locks cause link failures on Windows.

## Code Style

### C (libtoaster, tests)
- `toaster_` prefix for all public symbols; `_t` suffix for types; `snake_case` everywhere
- `TOASTER_API` macro on public functions (controls dllexport/visibility)
- `bool` returns for success/failure (true = success); no exceptions
- `calloc()` for zero-init; destructors always null-check first
- Array growth: `cap ? cap * 2 : initial_size` (exponential doubling)
- All timestamps in **microseconds**
- Forward declarations over transitive includes

### C++ (frontend)
- Qt conventions: `m_` prefix for member variables, camelCase methods
- **Always** `blockSignals(true)` around programmatic QTextEdit/QTableWidget content changes to prevent re-entrant signal loops

## Test Conventions

Tests are standalone C executables using a simple PASS/FAIL macro pattern:

```c
static int failures = 0;
#define PASS(name) printf("  PASS: %s\n", name)
#define FAIL(name, msg) do { printf("  FAIL: %s — %s\n", name, msg); failures++; } while (0)
```

Each test calls `toaster_startup()` at entry and `toaster_shutdown()` at exit. Return `failures ? 1 : 0`.

## Known Pitfalls

- **DLL lock on Windows**: Kill old `toaster-app.exe` before rebuilding — link failures otherwise
- **Signal re-entrancy**: `blockSignals(true)` around programmatic widget content changes
- **Timestamp mapping**: Deleted words reduce effective duration; playback must sum deleted segments to map edit-time → source-time
- **Undo snapshot timing**: Always call `toaster_transcript_save_snapshot()` *before* mutating the transcript

## Planned Architecture (Not Yet Implemented)

The following are planned for later phases (see PRD.md) but have **no code yet**:
- `plugins/` directory with loadable modules (FFmpeg decoder/exporter, filters)
- Plugin registration system (`toaster_{type}_info_t` + `{name}_load()`)
- FFmpeg integration for media decode and export
- OBS Studio source filter plugin (Phase 4)

## Optional Terse Skills

- `caveman` is opt-in. Use it only when the user explicitly asks for caveman mode, fewer tokens, or very terse answers.
- `caveman-commit` is opt-in for terse conventional commit messages.
- `caveman-review` is opt-in for terse paste-ready review comments.
- Clarity overrides terseness for security warnings, irreversible actions, onboarding explanations, and multi-step debugging procedures.
- These skills complement the project guidance in this file; they do not replace it.
