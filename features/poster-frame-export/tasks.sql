-- Task graph for poster-frame-export.
-- Ingest into the session SQL store with the `sql` tool.
--
-- Schema: todos(id, title, description, status). todo_deps(todo_id, depends_on).
-- Allowed status: pending, in_progress, done, blocked.

INSERT INTO todos (id, title, description, status) VALUES
  ('poster-frame-export-project-schema',
   'Add poster_frame_ms to ProjectSettings',
   'Extend src-tauri/src/managers/project.rs: add `poster_frame_ms: Option<u64>` with #[serde(default)] to ProjectSettings; bump PROJECT_VERSION to "1.2.0"; update Default impl. Add unit tests (1) round-trip Some(1234) through save + load, (2) legacy v1.1.0 JSON string deserializes with poster_frame_ms == None. Verifier: AC-001-a (cargo test -p toaster --lib managers::project::tests::poster_frame). See BLUEPRINT.md "Architecture decisions" R-001.',
   'pending'),
  ('poster-frame-export-set-command',
   'Add set_poster_frame Tauri command',
   'Add a Tauri command `set_poster_frame(ms: Option<u64>)` in src-tauri/src/commands/project.rs that mutates the current ToasterProject.settings.poster_frame_ms and triggers the existing save path. Register in the invoke_handler. Regenerate bindings.ts via specta (do NOT hand-edit bindings.ts). Verifier: AC-001-a (round-trip covers persistence of values written by this command; real call path exercised by poster-frame-export-eval fixtures).',
   'pending'),
  ('poster-frame-export-menu-entry',
   'Add "Set as poster frame" context-menu entry',
   'In src/components/editor/TranscriptEditor.tsx, add a new entry to the word-level context menu following the handleDeleteSelected / handleRestoreSelected pattern at lines 203-230. The handler reads the clicked word''s start_us, calls invoke("set_poster_frame", { ms: Math.round(start_us / 1000) }), and calls closeContextMenu(). Add one new i18next key with an English default; mirror the key (same English string) into all 20 files under src/i18n/locales/*/translation.json. Verifier: AC-001-b (bun scripts/check-translations.ts).',
   'pending'),
  ('poster-frame-export-build-args',
   'Thread poster_frame_ms into build_export_args',
   'Extend build_export_args in src-tauri/src/commands/waveform/mod.rs to take `poster_frame_ms: Option<u64>`. When None, produce byte-identical argv to today (guarded by AC-002-c golden). When Some(ms), do NOT inject attachment args into the primary mux; attachment is a separate FFmpeg pass (see BLUEPRINT "R-002 two-pass export"). This task just plumbs the parameter and updates the argv-emitting tests in src-tauri/src/commands/waveform/tests/part2.rs to cover None == identical. Verifier: AC-002-c (pwsh scripts/eval/eval-poster-frame.ps1 -Mode argv-identity -Format mp4).',
   'pending'),
  ('poster-frame-export-extract-and-attach',
   'Implement extract + attach passes in export_edited_media',
   'In src-tauri/src/commands/waveform/commands.rs export_edited_media (lines 386-535): after the primary FFmpeg pass, if poster_frame_ms.is_some() AND effective_has_video AND the output extension is mp4 or mov: (a) compute clamped timestamp against duration_us from snapped_segments; (b) pass 2 = `ffmpeg -ss <clamped_secs> -i <output> -frames:v 1 <temp.png>`; (c) pass 3 = `ffmpeg -i <output> -attach <temp.png> -metadata:s:t mimetype=image/png -metadata:s:t filename=cover.png -c copy <output.tmp>` then atomic rename over <output>. Temp PNG path uses std::env::temp_dir() with a unique name. Unconditional cleanup of temp PNG on both success and failure paths, mirroring the ASS cleanup at lines 505-508. Verifiers: AC-002-a, AC-002-b, AC-002-d, AC-003-a, AC-003-b.',
   'pending'),
  ('poster-frame-export-eval',
   'Implement eval-poster-frame.ps1 and fixtures',
   'Flesh out scripts/eval/eval-poster-frame.ps1 (currently stub, exit 2) to cover all six Modes described in its header: attachment, no-attachment, clamp, audio-only, cleanup, argv-identity. Commit a small fixture under eval/fixtures/ (a short MP4 + a .toaster project JSON with poster_frame_ms pre-seeded to a known millisecond value). For argv-identity, record a golden argv text file next to the fixture. Use ffprobe -show_streams -of json to assert attachment presence/absence. Verifiers: AC-002-a, AC-002-b, AC-002-c, AC-002-d, AC-003-a, AC-003-b.',
   'pending'),
  ('poster-frame-export-qc',
   'QC: coverage + tasks + translations + eval-harness',
   'Run: pwsh scripts/feature/check-feature-coverage.ps1 -Feature poster-frame-export; pwsh scripts/feature/check-feature-tasks.ps1 -Feature poster-frame-export; bun scripts/check-translations.ts; then the eval-harness-runner agent for the poster-frame eval. All must be green. This is the final gate before promote.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('poster-frame-export-set-command',       'poster-frame-export-project-schema'),
  ('poster-frame-export-menu-entry',        'poster-frame-export-set-command'),
  ('poster-frame-export-build-args',        'poster-frame-export-project-schema'),
  ('poster-frame-export-extract-and-attach','poster-frame-export-build-args'),
  ('poster-frame-export-eval',              'poster-frame-export-extract-and-attach'),
  ('poster-frame-export-eval',              'poster-frame-export-menu-entry'),
  ('poster-frame-export-qc',                'poster-frame-export-eval');
