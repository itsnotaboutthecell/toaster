-- Task graph for ass-sidecar-export.
-- Ingest into the session SQL store with the `sql` tool.

-- Schema: todos(id TEXT, title TEXT, description TEXT, status TEXT).
-- Allowed status values: 'pending', 'in_progress', 'done', 'blocked'.
-- todo_deps schema: (todo_id TEXT, depends_on TEXT).
INSERT INTO todos (id, title, description, status) VALUES
  ('ass-sidecar-export-setting',
   'Add export_ass_sidecar_enabled setting + typed handler',
   'Add export_ass_sidecar_enabled: bool to AppSettings (settings/types.rs near normalize_audio_on_export, line 269), default false in settings/defaults.rs (line 541), typed setter change_export_ass_sidecar_setting in commands/app_settings.rs (mirror change_normalize_audio_setting at 487-492), register in lib.rs (near 223). Add a Rust unit test round-tripping the field. Verifier: AC-001-a (cargo-test).',
   'pending'),
  ('ass-sidecar-export-refactor',
   'Extract build_export_ass_doc helper (single-site blocks_to_ass)',
   'Refactor commands/waveform/commands.rs:430-443 so the ASS document is built by a single helper build_export_ass_doc taking (&words, &segments, &settings, frame_size) and returning Option<String> (None for non-video / audio-only). The burn branch and the new sidecar branch both consume this Option. blocks_to_ass must appear exactly once inside src-tauri/src/commands/waveform/. Commit the stub scripts/feature/check-ass-generator-singleton.ps1 (exit 2 until implemented). Verifier: AC-003-a (script).',
   'pending'),
  ('ass-sidecar-export-sidecar-write',
   'Write the sidecar file in export_edited_media',
   'When AppSettings.export_ass_sidecar_enabled is true AND has_video AND !export_format.is_audio_only(), write the Option<String> document to Path::new(&output_path).with_extension("ass") via std::fs::write. Happens before the FFmpeg spawn so write failures short-circuit. Propagate errors as Result::Err(String). Verifier: AC-003-b, AC-004-a..d (manual fixture task).',
   'pending'),
  ('ass-sidecar-export-frontend',
   'Add the toolbar toggle bound to the persisted setting',
   'In src/components/editor/EditorView.tsx, add a new toggle adjacent to the burn captions toggle at 538-548. Read AppSettings.export_ass_sidecar_enabled via the existing settings hook (not a local useState). On click, invoke commands.changeExportAssSidecarSetting and re-read settings. Use the new i18n key editor.saveAssSidecar. Verifier: AC-002-a (manual live app).',
   'pending'),
  ('ass-sidecar-export-i18n',
   'Add editor.saveAssSidecar to all 20 locales',
   'Add the key editor.saveAssSidecar to each src/i18n/locales/*/translation.json adjacent to editor.burnCaptions at line 654. English: "Also save .ass subtitle sidecar". Other locales: provide accurate translations or leave as the English string until a translator review pass (follow the convention used for other recent additions). Run bun run scripts/check-translations.ts and confirm it passes. Verifier: AC-002-b (script).',
   'pending'),
  ('ass-sidecar-export-fixture',
   'Manual fixture runs for all orthogonality combinations',
   'Run the four orthogonality combinations (burn off+sidecar off, off+on, on+off, on+on) plus the audio-only gate against eval/fixtures/toaster_example.mp4 and record SHA-256 hashes + observations in journal.md. Verifier: AC-003-b, AC-004-a, AC-004-b, AC-004-c, AC-004-d (manual).',
   'pending'),
  ('ass-sidecar-export-qc',
   'QC: run coverage + tasks gates + eval-harness-runner',
   'Run `pwsh scripts/feature/check-feature-coverage.ps1 -Feature ass-sidecar-export` and `pwsh scripts/feature/check-feature-tasks.ps1 -Feature ass-sidecar-export`. Then invoke the eval-harness-runner agent to confirm caption-parity-eval and transcript-precision-eval still pass. All three must exit 0.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('ass-sidecar-export-sidecar-write', 'ass-sidecar-export-setting'),
  ('ass-sidecar-export-sidecar-write', 'ass-sidecar-export-refactor'),
  ('ass-sidecar-export-frontend',      'ass-sidecar-export-setting'),
  ('ass-sidecar-export-i18n',          'ass-sidecar-export-frontend'),
  ('ass-sidecar-export-fixture',       'ass-sidecar-export-sidecar-write'),
  ('ass-sidecar-export-fixture',       'ass-sidecar-export-frontend'),
  ('ass-sidecar-export-fixture',       'ass-sidecar-export-i18n'),
  ('ass-sidecar-export-qc',            'ass-sidecar-export-fixture');
