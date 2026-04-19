-- Task graph for export-loudness.
INSERT INTO todos (id, title, description, status) VALUES
  ('el-loudness-target-enum',
   'Add LoudnessTarget enum + build_loudnorm_filter helper',
   'In src-tauri/src/managers/splice/loudness.rs, add LoudnessTarget enum (Off/Podcast_-16/Streaming_-14) and build_loudnorm_filter helper returning Option<String>. Cover with cargo test build_loudnorm_filter. Verifier: AC-003-a.',
   'pending'),

  ('el-settings-field',
   'Add loudness_target settings field + migration',
   'Add loudness_target field to src-tauri/src/settings/types.rs:262 area; default Off in defaults.rs. Add migrate_loudness_setting in settings/mod.rs that maps legacy normalize_audio_on_export. Cover with cargo test migrate_loudness_setting. Verifier: AC-004-a, AC-004-b.',
   'pending'),

  ('el-loudnorm-rewire',
   'Replace inline loudnorm string in waveform/mod.rs:121',
   'Replace the literal "loudnorm=I=-16:TP=-1.5:LRA=11" at src-tauri/src/commands/waveform/mod.rs:121 with a call to splice::loudness::build_loudnorm_filter(settings.loudness_target). No string literal remains. Verifier: AC-003-b, AC-003-c.',
   'pending'),

  ('el-preflight-command',
   'Add loudness_preflight Tauri command',
   'In src-tauri/src/commands/waveform/commands.rs, add loudness_preflight command that walks keep-segments to decode post-edit PCM and calls splice::loudness::measure_loudness. Returns LoudnessPreflight DTO. Register in lib.rs. Add cargo test loudness_preflight_roundtrip on eval/fixtures/toaster_example.mp4. Verifier: AC-002-a.',
   'pending'),

  ('el-export-panel',
   'Create ExportSettings panel + sidebar entry',
   'Create src/components/settings/export/ExportSettings.tsx with the LoudnessTarget Select wired to useSettings. Add sidebar entry sidebar.export. Bundle 2 and 3 will extend this same panel. Verifier: AC-001-a, AC-001-b.',
   'pending'),

  ('el-export-dialog-readout',
   'Wire preflight readout into export dialog',
   'In the export dialog component, add a Run preflight button that invokes loudness_preflight and renders integrated_lufs/true_peak_dbtp/lra. Show >12 LU off-target warning. No arithmetic on the floats; only formatters. Verifier: AC-002-b, AC-002-c, AC-006-a.',
   'pending'),

  ('el-i18n-keys',
   'Add Export panel + preflight i18n keys to all 20 locales',
   'Add sidebar.export, settings.export.loudness.* (target labels + descriptions), dialog.export.preflight.* (heading, button, lufs/dbtp/lra labels, off-target warning) to every src/i18n/locales/*/translation.json. Use i18n-pruning skill.',
   'pending'),

  ('el-qc-panel',
   'QC: Export panel scaffolding (R-001)',
   'Verifies AC-001-a (live-app) and AC-001-b (BLUEPRINT doc-section).',
   'pending'),

  ('el-qc-preflight',
   'QC: preflight readout + warning (R-002)',
   'Verifies AC-002-a (cargo test), AC-002-b/c (live app).',
   'pending'),

  ('el-qc-filter-authority',
   'QC: single-source-of-truth for loudnorm parameters (R-003)',
   'Verifies AC-003-a (cargo test), AC-003-b (grep), AC-003-c (live-app round-trip).',
   'pending'),

  ('el-qc-migration',
   'QC: legacy boolean -> enum migration (R-004)',
   'Verifies AC-004-a (cargo test), AC-004-b (live app).',
   'pending'),

  ('el-qc-boundary',
   'QC: audio-boundary-eval after filter refactor (R-005)',
   'Run audio-boundary-eval skill on eval/fixtures/toaster_example.mp4. Verifier: AC-005-a.',
   'pending'),

  ('el-qc-backend-authority',
   'QC: backend authority for loudness math (R-006)',
   'Verifies AC-006-a (grep) and AC-006-b (BLUEPRINT doc-section).',
   'pending'),

  ('feature-qc',
   'QC: coverage gate green',
   'Run pwsh scripts/check-feature-coverage.ps1 -Feature export-loudness and confirm exit 0.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('el-loudnorm-rewire', 'el-loudness-target-enum'),
  ('el-loudnorm-rewire', 'el-settings-field'),
  ('el-preflight-command', 'el-loudness-target-enum'),
  ('el-export-panel', 'el-settings-field'),
  ('el-export-panel', 'el-i18n-keys'),
  ('el-export-dialog-readout', 'el-preflight-command'),
  ('el-export-dialog-readout', 'el-i18n-keys'),
  ('el-qc-panel', 'el-export-panel'),
  ('el-qc-preflight', 'el-export-dialog-readout'),
  ('el-qc-filter-authority', 'el-loudnorm-rewire'),
  ('el-qc-filter-authority', 'el-export-panel'),
  ('el-qc-migration', 'el-settings-field'),
  ('el-qc-migration', 'el-export-panel'),
  ('el-qc-boundary', 'el-loudnorm-rewire'),
  ('el-qc-backend-authority', 'el-export-dialog-readout'),
  ('feature-qc', 'el-qc-panel'),
  ('feature-qc', 'el-qc-preflight'),
  ('feature-qc', 'el-qc-filter-authority'),
  ('feature-qc', 'el-qc-migration'),
  ('feature-qc', 'el-qc-boundary'),
  ('feature-qc', 'el-qc-backend-authority');
