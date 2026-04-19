-- Task graph for ui-experimental-and-cleanup.
INSERT INTO todos (id, title, description, status) VALUES
  ('uec-experiments-registry',
   '5a: create src/lib/experiments.ts registry',
   'New file with Experiment type + readonly experiments array. First occupant: experimental_simplify_mode (settingsKey, labelKey, descriptionKey, feedbackUrl). Verifier: AC-001-d.',
   'pending'),

  ('uec-experimental-panel',
   '5a: add ExperimentalSettings panel + sidebar entry',
   'New src/components/settings/experimental/ExperimentalSettings.tsx with banner + iterate registry + Toggle. Add sidebar.experimental entry. Verifier: AC-001-a, AC-001-b, AC-001-c.',
   'pending'),

  ('uec-experimental-i18n',
   '5a: add Experimental panel i18n keys to all 20 locales',
   'Add sidebar.experimental, settings.experimental.banner, experiments.simplifyMode.label/description, experiments.feedbackLink to every src/i18n/locales/*/translation.json. Use i18n-pruning skill.',
   'pending'),

  ('uec-postprocessing-wrapper',
   '5b: create PostProcessingSettings wrapper',
   'New src/components/settings/post-processing/PostProcessingSettings.tsx composing the existing PostProcessingSettingsPrompts + post-processing-api/* provider components. Verifier: AC-002-a, AC-002-c.',
   'pending'),

  ('uec-postprocessing-mount',
   '5b: re-mount post-processing under sidebar.postProcessing',
   'Wire the wrapper into the settings sidebar using the existing sidebar.postProcessing key (verify the value text still applies before re-using). Verifier: AC-002-a, AC-002-b.',
   'pending'),

  ('uec-handy-legacy-gate',
   '5c: invoke handy-legacy-pruning skill on apple-stub files',
   'Apply handy-legacy-pruning skill to src-tauri/src/commands/mod.rs:126, src-tauri/src/settings/mod.rs:13. Record the rationale in journal.md under "## Apple stub deletion rationale". Verifier: AC-005-a.',
   'pending'),

  ('uec-apple-stub-delete',
   '5c: delete apple_intelligence stub + plumbing',
   'Delete check_apple_intelligence_available command (src-tauri/src/commands/mod.rs:126) + registration (src-tauri/src/lib.rs:231) + APPLE_INTELLIGENCE_PROVIDER_ID constant (src-tauri/src/settings/mod.rs:13) + APPLE_PROVIDER_ID + apple branches in src/components/settings/post-processing-api/usePostProcessProviderState.ts. Regenerate src/bindings.ts. Verifier: AC-003-a, AC-003-b, AC-003-c.',
   'pending'),

  ('uec-apple-i18n-purge',
   '5c: remove appleIntelligence i18n keys',
   'Remove appleIntelligence keys + placeholderApple + descriptionApple from every src/i18n/locales/*/translation.json. Use i18n-pruning skill.',
   'pending'),

  ('uec-final-orphan-list',
   '5c: compute final still-orphaned key list and remove',
   'After 5a + 5b have landed, run scripts/check-translations.ts and rg to enumerate keys in product-map-v1 PRD §3 that are still orphaned (sidebar.general, overlay namespace, etc.). Update BLUEPRINT.md "Migration / compatibility" with the confirmed list. Remove only those keys across all 20 locales. Use i18n-pruning skill. Verifier: AC-004-a, AC-004-b, AC-004-c.',
   'pending'),

  ('uec-qc-experimental',
   'QC: Experimental pattern (R-001)',
   'Verifies AC-001-a/b/c (live app) and AC-001-d (source-tree).',
   'pending'),

  ('uec-qc-postprocessing',
   'QC: post-processing UI restored (R-002)',
   'Verifies AC-002-a/b (live app) and AC-002-c (BLUEPRINT doc-section).',
   'pending'),

  ('uec-qc-apple-purge',
   'QC: apple stub deleted (R-003)',
   'Verifies AC-003-a (cargo test/workspace), AC-003-b (grep), AC-003-c (live app fallback).',
   'pending'),

  ('uec-qc-i18n',
   'QC: orphan-i18n purge + translations gate (R-004)',
   'Verifies AC-004-a (check-translations.ts), AC-004-b (i18n-pruning skill), AC-004-c (BLUEPRINT doc-section).',
   'pending'),

  ('uec-qc-handy-legacy',
   'QC: handy-legacy-pruning rationale recorded (R-005)',
   'Verifies AC-005-a (journal.md doc-section).',
   'pending'),

  ('feature-qc',
   'QC: coverage gate green',
   'Run pwsh scripts/check-feature-coverage.ps1 -Feature ui-experimental-and-cleanup and confirm exit 0.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('uec-experimental-panel', 'uec-experiments-registry'),
  ('uec-experimental-panel', 'uec-experimental-i18n'),
  ('uec-postprocessing-mount', 'uec-postprocessing-wrapper'),
  ('uec-apple-stub-delete', 'uec-handy-legacy-gate'),
  ('uec-apple-stub-delete', 'uec-postprocessing-mount'),
  ('uec-apple-i18n-purge', 'uec-apple-stub-delete'),
  ('uec-final-orphan-list', 'uec-experimental-panel'),
  ('uec-final-orphan-list', 'uec-postprocessing-mount'),
  ('uec-final-orphan-list', 'uec-apple-i18n-purge'),
  ('uec-qc-experimental', 'uec-experimental-panel'),
  ('uec-qc-postprocessing', 'uec-postprocessing-mount'),
  ('uec-qc-apple-purge', 'uec-apple-stub-delete'),
  ('uec-qc-i18n', 'uec-final-orphan-list'),
  ('uec-qc-handy-legacy', 'uec-handy-legacy-gate'),
  ('feature-qc', 'uec-qc-experimental'),
  ('feature-qc', 'uec-qc-postprocessing'),
  ('feature-qc', 'uec-qc-apple-purge'),
  ('feature-qc', 'uec-qc-i18n'),
  ('feature-qc', 'uec-qc-handy-legacy');
