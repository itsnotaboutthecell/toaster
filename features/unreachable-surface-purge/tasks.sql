-- Task graph for unreachable-surface-purge.
-- Schema: todos(id, title, description, status) + todo_deps(todo_id, depends_on).
-- Five implementation chunks (A..E) each followed by a QC task, with a
-- final feature-qc that runs the coverage gate.

INSERT INTO todos (id, title, description, status) VALUES
  ('usp-experimental-delete',
   'Chunk A: delete experimental_simplify_mode end-to-end',
   'Remove the field at src-tauri/src/settings/types.rs:259, any default in src-tauri/src/settings/defaults.rs, the helper settings_experimental_simplify_mode_enabled at src-tauri/src/commands/waveform/mod.rs:299, and the flag parameter + dead branch in canonical_keep_segments_for_media_with_options and select_raw_keep_segments_for_media (lines 331, 343, 352, 357, 363, 366). Regenerate src/bindings.ts if the project has a regen step. Verifiers: AC-001-a, AC-001-b, AC-001-c.',
   'pending'),

  ('usp-i18n-orphans',
   'Chunk B: purge sidebar.general and overlay namespace across 20 locales',
   'Using the i18n-pruning skill, remove the sidebar.general key (if present) and the entire overlay top-level namespace (if present) from every src/i18n/locales/*/translation.json file. Touch only the sidebar.* subtree to avoid clobbering settings.general.*. Verifiers: AC-002-a, AC-002-b, AC-002-c.',
   'pending'),

  ('usp-apple-residue',
   'Chunk C: verify + mop up check_apple_intelligence_available residue',
   'Recon already shows the Rust stub is deleted. Run the rg audits from coverage.json (AC-003-a, AC-003-c). If any residue remains in src/ (frontend invoke caller, bindings entry), delete it and regenerate src/bindings.ts. Verifiers: AC-003-a, AC-003-b, AC-003-c.',
   'pending'),

  ('usp-postprocessing-label',
   'Chunk D: add Local-LLM-only label to post-processing panel and verify mount',
   'Add a loopback-advisory label at the top of src/components/settings/post-processing/PostProcessingSettings.tsx citing src-tauri/src/llm_client.rs is_local_host. Reuse an existing i18n key if one fits; otherwise add settings.postProcessing.localOnlyNotice to all 20 locales per i18n-pruning. Verify sidebar.postProcessing remains in SECTIONS_CONFIG (src/components/Sidebar.tsx:39-44) and the UMC ModelsSettings embed still renders. Verifiers: AC-004-a, AC-004-b, AC-004-c.',
   'pending'),

  ('usp-debug-gated',
   'Chunk E: re-mount sidebar.debug gated by settings.debug_mode',
   'Create src/components/settings/debug/DebugSettings.tsx that composes DebugPaths, LogLevelSelector, WordCorrectionThreshold, and LogDirectory. Export it from src/components/settings/index.ts. Add a debug entry to SECTIONS_CONFIG in src/components/Sidebar.tsx with enabled() returning settings.debug_mode === true. Reuse the existing Ctrl+Shift+D handler at src/App.tsx:67-89. Rely on the existing defensive fallback at src/components/Sidebar.tsx:59+ when debug mode turns off. Verifiers: AC-005-a, AC-005-b, AC-005-c.',
   'pending'),

  ('usp-qc-experimental-delete',
   'QC: experimental_simplify_mode deletion (R-001)',
   'Confirms AC-001-a (rg gate), AC-001-b (cargo test workspace), AC-001-c (live-app relaunch tolerates legacy settings.json).',
   'pending'),

  ('usp-qc-i18n-orphans',
   'QC: i18n orphan purge (R-002)',
   'Confirms AC-002-a (bun run scripts/check-translations.ts), AC-002-b (rg sidebar.general), AC-002-c (rg overlay).',
   'pending'),

  ('usp-qc-apple-residue',
   'QC: apple stub purge complete (R-003)',
   'Confirms AC-003-a (rg check_apple_intelligence_available), AC-003-b (cargo test workspace), AC-003-c (rg appleIntelligence constants).',
   'pending'),

  ('usp-qc-postprocessing-label',
   'QC: post-processing reachable with loopback label (R-004)',
   'Confirms AC-004-a (live label visible), AC-004-b (ModelsSettings embed mounts), AC-004-c (is_local_host enforcement point present).',
   'pending'),

  ('usp-qc-debug-gated',
   'QC: debug sidebar gated by debug_mode (R-005)',
   'Confirms AC-005-a (debug entry hidden when debug_mode false), AC-005-b (Ctrl+Shift+D reveals and panel mounts all four components), AC-005-c (toggling off falls back without error).',
   'pending'),

  ('feature-qc',
   'QC: feature coverage + tasks gates green',
   'Run pwsh scripts/feature/check-feature-coverage.ps1 -Feature unreachable-surface-purge and pwsh scripts/feature/check-feature-tasks.ps1 -Feature unreachable-surface-purge; both must exit 0.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('usp-qc-experimental-delete', 'usp-experimental-delete'),
  ('usp-qc-i18n-orphans',        'usp-i18n-orphans'),
  ('usp-qc-apple-residue',       'usp-apple-residue'),
  ('usp-qc-postprocessing-label','usp-postprocessing-label'),
  ('usp-qc-debug-gated',         'usp-debug-gated'),
  ('feature-qc',                 'usp-qc-experimental-delete'),
  ('feature-qc',                 'usp-qc-i18n-orphans'),
  ('feature-qc',                 'usp-qc-apple-residue'),
  ('feature-qc',                 'usp-qc-postprocessing-label'),
  ('feature-qc',                 'usp-qc-debug-gated');
