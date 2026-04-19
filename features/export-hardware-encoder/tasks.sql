-- Task graph for export-hardware-encoder.
INSERT INTO todos (id, title, description, status) VALUES
  ('ehe-detect-module',
   'Create managers/export/encoders.rs with detect_encoders + parser',
   'New module src-tauri/src/managers/export/encoders.rs. detect_encoders runs ffmpeg -encoders with a 1s timeout and returns EncoderAvailability. Add parse_ffmpeg_encoders_output cargo test against a bundled fixture snapshot. Verifier: AC-001-a.',
   'pending'),

  ('ehe-init-wire',
   'Run detection at app init; store on AppHandle; expose Tauri command',
   'In src-tauri/src/lib.rs, call detect_encoders at startup and stash EncoderAvailability on managed state. Register get_encoder_availability command. Verifier: AC-001-b, AC-001-c.',
   'pending'),

  ('ehe-encoder-enum',
   'Add VideoEncoder enum + settings field',
   'In src-tauri/src/settings/types.rs add VideoEncoder enum (Auto, Cpu, Nvenc, Qsv, VideoToolbox, Vaapi). Default Auto in defaults.rs. Verifier: AC-002-a.',
   'pending'),

  ('ehe-codec-for',
   'Add codec_for + resolve_encoder helpers',
   'In managers/export/encoders.rs add codec_for(encoder, role) and resolve_encoder(setting, availability). Auto priority NVENC > QSV > VideoToolbox > VAAPI > libx264. Verifier: AC-005-a.',
   'pending'),

  ('ehe-fallback-wrapper',
   'Add run_export_with_fallback wrapper',
   'New src-tauri/src/managers/export/run.rs with run_export_with_fallback that runs FFmpeg, classifies encoder-init errors, retries libx264 once, emits toast event on retry. Cover with cargo tests hardware_encoder_fallback and hardware_encoder_fallback_no_loop. Verifier: AC-003-a, AC-003-b.',
   'pending'),

  ('ehe-waveform-rewire',
   'Replace -c:v libx264 literal with codec_for + run_export_with_fallback',
   'In src-tauri/src/commands/waveform/mod.rs:522 area, route the video codec choice through codec_for and the FFmpeg invocation through run_export_with_fallback. Verifier: AC-002-a, AC-002-b, AC-003-c, AC-005-a.',
   'pending'),

  ('ehe-encoder-select',
   'Add Encoder Select to ExportSettings.tsx',
   'In src/components/settings/export/ExportSettings.tsx add an Encoder Select wired to settings.video_encoder, populated from get_encoder_availability. Verifier: AC-001-b, AC-002-a.',
   'pending'),

  ('ehe-i18n-keys',
   'Add encoder Select + fallback toast i18n keys to all 20 locales',
   'Add settings.export.encoder.label, .auto, .cpu, .nvenc, .qsv, .videotoolbox, .vaapi, and toast.export.encoderFallback to every src/i18n/locales/*/translation.json. Use i18n-pruning skill.',
   'pending'),

  ('ehe-qc-detect',
   'QC: encoder detection (R-001)',
   'Verifies AC-001-a (cargo test), AC-001-b/c (live app).',
   'pending'),

  ('ehe-qc-picker',
   'QC: encoder picker live-app behavior (R-002)',
   'Verifies AC-002-a, AC-002-b via live-app ffprobe inspection.',
   'pending'),

  ('ehe-qc-fallback',
   'QC: safe fallback (R-003)',
   'Verifies AC-003-a, AC-003-b (cargo tests), AC-003-c (live app on driverless VM).',
   'pending'),

  ('ehe-qc-parity',
   'QC: cut-drift-fuzzer + audio-boundary-eval after encoder switch (R-004)',
   'Run cut-drift-fuzzer agent and audio-boundary-eval skill. Verifier: AC-004-a, AC-004-b.',
   'pending'),

  ('ehe-qc-ssot',
   'QC: SSOT for encoder choice + codec strings (R-005)',
   'Verifies AC-005-a (grep) and AC-005-b (BLUEPRINT doc-section).',
   'pending'),

  ('feature-qc',
   'QC: coverage gate green',
   'Run pwsh scripts/check-feature-coverage.ps1 -Feature export-hardware-encoder and confirm exit 0.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('ehe-init-wire', 'ehe-detect-module'),
  ('ehe-codec-for', 'ehe-detect-module'),
  ('ehe-codec-for', 'ehe-encoder-enum'),
  ('ehe-fallback-wrapper', 'ehe-codec-for'),
  ('ehe-waveform-rewire', 'ehe-fallback-wrapper'),
  ('ehe-waveform-rewire', 'ehe-init-wire'),
  ('ehe-encoder-select', 'ehe-init-wire'),
  ('ehe-encoder-select', 'ehe-encoder-enum'),
  ('ehe-encoder-select', 'ehe-i18n-keys'),
  ('ehe-qc-detect', 'ehe-init-wire'),
  ('ehe-qc-detect', 'ehe-encoder-select'),
  ('ehe-qc-picker', 'ehe-waveform-rewire'),
  ('ehe-qc-picker', 'ehe-encoder-select'),
  ('ehe-qc-fallback', 'ehe-fallback-wrapper'),
  ('ehe-qc-fallback', 'ehe-waveform-rewire'),
  ('ehe-qc-parity', 'ehe-waveform-rewire'),
  ('ehe-qc-ssot', 'ehe-waveform-rewire'),
  ('feature-qc', 'ehe-qc-detect'),
  ('feature-qc', 'ehe-qc-picker'),
  ('feature-qc', 'ehe-qc-fallback'),
  ('feature-qc', 'ehe-qc-parity'),
  ('feature-qc', 'ehe-qc-ssot');
