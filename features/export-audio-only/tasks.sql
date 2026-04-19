-- Task graph for export-audio-only.
INSERT INTO todos (id, title, description, status) VALUES
  ('eao-format-enum',
   'Add ExportFormat enum + settings field',
   'Add ExportFormat { Mp4, Mp3, Wav, M4a, Opus } to src-tauri/src/settings/types.rs; default Mp4 in defaults.rs. Verifier: AC-001-a.',
   'pending'),

  ('eao-codec-map',
   'Add export_format_codec_map helper + unit tests',
   'In src-tauri/src/commands/waveform/mod.rs (or sibling if size cap pressures), add export_format_codec_map returning (ext, -c:a, -vn, optional bitrate). Cover with cargo test export_format_codec_map. Verifier: AC-002-a.',
   'pending'),

  ('eao-export-branch',
   'Branch export args on export_format; drop video stream for audio-only',
   'In src-tauri/src/commands/waveform/mod.rs:494-527 area, branch on settings.export_format. For audio-only, append -vn and omit -c:v. Build_audio_post_filters is reused unchanged. Add cargo test export_format_args_no_video_stream. Verifier: AC-002-b, AC-005-a.',
   'pending'),

  ('eao-roundtrip-test',
   'Add audio_only_roundtrip_durations cargo test',
   'New ignored cargo test that exports eval/fixtures/toaster_example.mp4 to each of mp3/wav/m4a/opus, ffprobe-decodes each, asserts duration within 30 ms. Verifier: AC-003-a.',
   'pending'),

  ('eao-format-select',
   'Add Format Select to ExportSettings.tsx',
   'In src/components/settings/export/ExportSettings.tsx (created by Bundle 1), add a Select bound to settings.export_format with 5 options. No new panel. Verifier: AC-001-a, AC-001-b.',
   'pending'),

  ('eao-i18n-keys',
   'Add format Select i18n keys to all 20 locales',
   'Add settings.export.format.label and the 5 option labels (video_mp4, audio_mp3, audio_wav, audio_m4a, audio_opus) to every src/i18n/locales/*/translation.json. Use i18n-pruning skill.',
   'pending'),

  ('eao-qc-picker',
   'QC: Format picker live-app behavior (R-001)',
   'Verifies AC-001-a, AC-001-b.',
   'pending'),

  ('eao-qc-codec-map',
   'QC: codec/muxer mapping (R-002)',
   'Verifies AC-002-a, AC-002-b via cargo test.',
   'pending'),

  ('eao-qc-roundtrip',
   'QC: round-trip duration parity (R-003)',
   'Verifies AC-003-a via cargo test (ignored test, run explicitly).',
   'pending'),

  ('eao-qc-precision',
   'QC: transcript precision on audio-only path (R-004)',
   'Run transcript-precision-eval skill. Verifier: AC-004-a.',
   'pending'),

  ('eao-qc-filter-reuse',
   'QC: audio-filter SSOT (R-005)',
   'Verifies AC-005-a (grep) and AC-005-b (BLUEPRINT doc-section).',
   'pending'),

  ('feature-qc',
   'QC: coverage gate green',
   'Run pwsh scripts/check-feature-coverage.ps1 -Feature export-audio-only and confirm exit 0.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('eao-codec-map', 'eao-format-enum'),
  ('eao-export-branch', 'eao-codec-map'),
  ('eao-roundtrip-test', 'eao-export-branch'),
  ('eao-format-select', 'eao-format-enum'),
  ('eao-format-select', 'eao-i18n-keys'),
  ('eao-qc-picker', 'eao-format-select'),
  ('eao-qc-codec-map', 'eao-codec-map'),
  ('eao-qc-roundtrip', 'eao-roundtrip-test'),
  ('eao-qc-precision', 'eao-export-branch'),
  ('eao-qc-filter-reuse', 'eao-export-branch'),
  ('feature-qc', 'eao-qc-picker'),
  ('feature-qc', 'eao-qc-codec-map'),
  ('feature-qc', 'eao-qc-roundtrip'),
  ('feature-qc', 'eao-qc-precision'),
  ('feature-qc', 'eao-qc-filter-reuse');
