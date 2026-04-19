# Blueprint: poster-frame-export

## Architecture decisions

- **R-001 storage**: add `poster_frame_ms: Option<u64>` to
  `ProjectSettings` in `src-tauri/src/managers/project.rs:34-47`.
  Follows the exact pattern used for `caption_profiles`
  (`src-tauri/src/managers/project.rs:41-46`): `#[serde(default)]`
  with `Option` so older project files deserialize to `None`. Bump
  `PROJECT_VERSION` from `"1.1.0"` to `"1.2.0"`
  (`src-tauri/src/managers/project.rs:17`); the existing rewrite-on-save
  behavior already normalizes older files to the current version
  (`src-tauri/src/managers/project.rs:86-87`).

- **R-001 UI**: add a new entry to the transcript editor's
  word-level context menu. The context-menu plumbing
  (`onContextMenu`, `setContextMenu`, `handleContextMenu`) already
  exists at `src/components/editor/TranscriptEditor.tsx:194-201`
  and `src/components/editor/TranscriptEditor.tsx:360-366`. Re-use
  the same pattern that `handleDeleteSelected`
  (`src/components/editor/TranscriptEditor.tsx:203-224`) and
  `handleRestoreSelected`
  (`src/components/editor/TranscriptEditor.tsx:226-230`) use: an
  action callback that `invoke`s a Tauri command and calls
  `closeContextMenu()`. The clicked word's `start_us` (edit-time
  micros) is divided by 1000 and passed as `poster_frame_ms: u64`
  to a new command `set_poster_frame`.

- **R-002 backend authority**: poster-frame extraction and
  attachment happen entirely inside the existing single export
  entry point, `export_edited_media`
  (`src-tauri/src/commands/waveform/commands.rs:386-535`), and its
  argv builder `build_export_args`
  (`src-tauri/src/commands/waveform/mod.rs`). The frontend never
  constructs FFmpeg flags; it only persists `poster_frame_ms` on
  the project. This matches the "one backend authority, two
  consumers" rule for dual-path logic.

- **R-002 two-pass export**: extracting from the *edited* output
  requires the rendered bytes. The simplest and lowest-risk design
  is a two-step command sequence inside `export_edited_media`:
  1. Run the existing mux end-to-end to produce the final mp4/mov
     at the user's chosen path.
  2. If and only if `poster_frame_ms.is_some()` and the format is
     mp4 or mov, run a second short FFmpeg process: `ffmpeg -ss
     <secs> -i <output> -frames:v 1 <temp.png>` (with `-ss` clamped
     to `duration - frame_period` on overshoot). Then run a third,
     in-place remux: `ffmpeg -i <output> -attach <temp.png>
     -metadata:s:t mimetype=image/png -metadata:s:t
     filename=cover.png -c copy <output.tmp>` and atomically rename
     `output.tmp` over `output`.
  Two passes (extract + remux) are cheap because both run
  `-c copy`/single-frame and write tiny files; they avoid entangling
  attachment flags with the primary filter graph.

- **R-002 no-op path**: when `poster_frame_ms` is `None`, neither
  pass 2 nor pass 3 runs. The argv for pass 1 must remain
  byte-identical to today's argv (AC-002-c); `build_export_args`
  accepts `poster_frame_ms: Option<u64>` but must only affect
  behavior in the `Some(_)` branch.

- **R-002 clamp behavior (AC-002-d)**: before launching pass 2,
  read `duration_us` already computed from the snapped segments
  (`snapped_segments` in
  `src-tauri/src/commands/waveform/commands.rs:482`) and clamp
  `poster_frame_ms` to `min(poster_frame_ms * 1000,
  duration_us - 33_333)` (one 30 fps frame below the end). The
  clamp is silent -- no warning, no error.

- **R-003 audio-only + webm**: the dispatch check is a two-line
  guard before pass 2: `if !effective_has_video || !matches!(ext,
  "mp4" | "mov") { return Ok(()); }` where `effective_has_video`
  already exists at
  `src-tauri/src/commands/waveform/commands.rs:457`. This covers
  the "audio-only ignores", "webm ignores", and "any future
  container ignores" cases uniformly.

- **R-003 temp-file hygiene**: reuse the exact ASS-cleanup pattern
  at `src-tauri/src/commands/waveform/commands.rs:505-508`: bind
  the `Option<PathBuf>` before the FFmpeg calls and
  unconditionally `let _ = std::fs::remove_file(...)` in a single
  cleanup block executed on both success and failure. The temp
  name is `std::env::temp_dir().join(format!(
  "toaster_poster_{}.png", uuid::Uuid::new_v4()))`; the `uuid`
  crate is already a transitive dep (confirm with `cargo tree`
  during implementation -- if not, prefer `format!(
  "toaster_poster_{}_{}.png", std::process::id(), timestamp_ns)`
  to avoid adding a crate).

## Component & module touch-list

| File | Change |
|------|--------|
| `src-tauri/src/managers/project.rs` | +1 field on `ProjectSettings`, +1 default init, bump `PROJECT_VERSION` to `"1.2.0"`. |
| `src-tauri/src/managers/project.rs` (tests) | +1 round-trip test: serialize `Some(1234)`, deserialize, assert equal; +1 legacy-load test: deserialize a v1.1.0 JSON string, assert `poster_frame_ms == None`. |
| `src-tauri/src/commands/project.rs` | +1 Tauri command `set_poster_frame(ms: Option<u64>)` that mutates the current project's settings and triggers a save on the existing project-state manager. |
| `src-tauri/src/commands/waveform/mod.rs` | Thread `poster_frame_ms: Option<u64>` into `build_export_args`'s signature; no argv change when `None`. |
| `src-tauri/src/commands/waveform/commands.rs` | In `export_edited_media`: compute temp PNG path, run pass-2 extract + pass-3 remux when `poster_frame_ms.is_some()` and `effective_has_video` and format is mp4/mov; unconditional temp cleanup. |
| `src/components/editor/TranscriptEditor.tsx` | +1 context-menu item "Set as poster frame"; +1 handler that reads the clicked word's `start_us` and calls `invoke("set_poster_frame", { ms: Math.round(start_us / 1000) })`. |
| `src/bindings.ts` | Regenerated (specta) -- do NOT hand-edit. |
| `src/i18n/locales/*/translation.json` (x20) | +1 new key, mirrored verbatim in every locale. Use the English string and let translators update in a later pass. |
| `eval/fixtures/` | +1 small fixture project (JSON) with a known `poster_frame_ms`, referenced by the new precision-eval script. |
| `scripts/eval/eval-poster-frame.ps1` | New script that runs the export pipeline against the fixture for mp4, mov, and audio-only formats; asserts attachment presence/absence via `ffprobe -show_streams -of json`. |

## Single-source-of-truth placement

- **Backend authority**: `build_export_args`
  (`src-tauri/src/commands/waveform/mod.rs`) and `export_edited_media`
  (`src-tauri/src/commands/waveform/commands.rs`) are the sole
  sites that translate `poster_frame_ms` into FFmpeg behavior. The
  frontend never constructs FFmpeg flags.
- **Frontend consumer**: `TranscriptEditor.tsx` only persists the
  value via a Tauri command and renders the menu label. It does
  not read or derive anything else from the field.
- **Project schema authority**: `ProjectSettings` in
  `src-tauri/src/managers/project.rs` is the one definition; TS
  types flow through specta-generated `bindings.ts`.

## Data flow

```
user right-clicks word in editor
  -> handleContextMenu selects word and opens menu
     -> user clicks "Set as poster frame"
        -> invoke("set_poster_frame", { ms: start_us / 1000 })
           -> commands::project::set_poster_frame mutates current
              ToasterProject.settings.poster_frame_ms = Some(ms)
              and triggers project save
                 -> ProjectSettings serialized with new field

user clicks Export
  -> export_edited_media(...)
     -> pass 1: existing ffmpeg mux -> output.mp4
     -> if Some(ms) && format in {mp4, mov} && has_video:
          pass 2: ffmpeg -ss clamp(ms) -i output.mp4 -frames:v 1
                  temp_poster.png
          pass 3: ffmpeg -i output.mp4 -attach temp_poster.png
                  -metadata:s:t mimetype=image/png -c copy
                  output.tmp.mp4  &&  rename -> output.mp4
     -> cleanup: remove temp_poster.png (success or failure)
```

## Migration / compatibility

- v1.0.0 and v1.1.0 `.toaster` files deserialize cleanly: the
  missing field defaults to `None` via `#[serde(default)]`.
- On next save, the file is rewritten as v1.2.0 with
  `"poster_frame_ms": null` present. There is no data loss and no
  one-way migration.
- No change to exports produced by older versions -- when the field
  is `None`, the argv for the primary pass is byte-identical to
  today's (guarded by AC-002-c golden).

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Attachment step disturbs mp4 mux (fragment order, `moov` box moved) | Use `-c copy` in the remux pass; attachment is a separate stream, not a re-encode. | AC-002-a, AC-002-b |
| Timestamp overshoot crashes extraction | Clamp to `duration - frame_period` before calling FFmpeg; never pass a raw out-of-range `-ss`. | AC-002-d |
| `poster_frame_ms` accidentally changes argv for unselected case | Wrap poster logic strictly in `if let Some(ms) = poster_frame_ms`; golden argv comparison in eval. | AC-002-c |
| Temp PNG leaks on FFmpeg failure | Cleanup block runs on both success and failure, matching the existing ASS-cleanup pattern. | AC-003-b |
| Audio-only export gets an attachment | Guard against `!effective_has_video`; also guards webm. | AC-003-a |
| i18n key drift | `bun scripts/check-translations.ts` run as QC step; blocks if any of 20 locales is missing the key. | AC-001-b |
| Older projects fail to load after version bump | Rely on `#[serde(default)]`; explicit legacy-load unit test in `project.rs` tests. | AC-001-a |
| Two extra FFmpeg invocations slow export perceptibly | Both are single-frame or `-c copy`; acceptable for a Launch-Ready polish feature. Monitor via eval runtime. | n/a (non-functional) |
