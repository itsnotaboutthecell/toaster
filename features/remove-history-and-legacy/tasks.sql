-- Task graph for remove-history-and-legacy.
-- Ingest into the session SQL store with the `sql` tool.
-- Order: deletion -> i18n cleanup -> dep cleanup -> live-app QC.

INSERT INTO todos (id, title, description, status) VALUES
  ('remove-history-and-legacy-disambiguate',
   'Confirm Q1 + Q2 with the user before any deletion',
   'Block on user answering: (Q1) "history components" = project transcription history (yes/no)? (Q2) Delete Experimental flag and gated controls (yes/no)? Defaults applied in PRD if user does not answer. Update REQUEST.md Q&A section verbatim once answers received.',
   'pending'),

  ('remove-history-and-legacy-baseline',
   'Capture pre-deletion baselines',
   'Run repo-auditor agent and record dead-module count baseline in journal.md. Run rg for HistoryManager / AdvancedSettings / Experimental* references and snapshot counts. Required by AC-003-a.',
   'pending'),

  ('remove-history-and-legacy-delete-backend',
   'Delete history backend (R-001)',
   'Delete src-tauri/src/managers/history.rs, managers/history_tests.rs, commands/history.rs. Edit managers/mod.rs, commands/mod.rs, lib.rs (lines 19, 135-144, 299-307), settings/types.rs, settings/defaults.rs, settings/io.rs, settings/mod.rs. Update comments in managers/cleanup/mod.rs:7, transcription_mock.rs:5, transcription/adapter.rs:109. Verifier: AC-001-a (cd src-tauri && cargo test --no-run).',
   'pending'),

  ('remove-history-and-legacy-delete-advanced',
   'Delete Advanced panel + relocate survivors (R-002)',
   'Delete src/components/settings/advanced/ and src/components/settings/history/ directories, plus HistoryLimit.tsx and RecordingRetentionPeriod.tsx. Edit Sidebar.tsx (drop advanced + history SECTIONS_CONFIG entries; drop History/Cog icon imports). Edit settings/index.ts (drop re-exports). Relocate ModelUnloadTimeoutSetting -> ModelsSettings; DiscardWords + AllowWords -> Editor settings surface; CaptionSettings -> ModelsSettings (temporary host pending caption-settings-preview feature). Edit settingsStore.ts (drop history_limit + recording_retention_period mutators). Verifier: AC-002-b (rg).',
   'pending'),

  ('remove-history-and-legacy-delete-legacy',
   'Delete dictation-era controls (R-003)',
   'Per-file: re-grep before each delete. Default deletes (subject to grep): ExperimentalToggle.tsx, ExperimentalSimplifyModeToggle.tsx, AccelerationSelector.tsx. Drop matching settings struct fields (experimental_enabled, etc.) compiled at execution time. Log any preserved file in journal.md with the consumer that saved it. Verifier: AC-003-b (rg).',
   'pending'),

  ('remove-history-and-legacy-update-agents-md',
   'Update AGENTS.md repo-layout block',
   'Edit AGENTS.md line ~71 to drop the managers/history/ entry. Re-verify the rest of the layout block still matches the actual managers/ tree. Single source of truth: do NOT mirror this change into .github/copilot-instructions.md (canonical-instructions skill).',
   'pending'),

  ('remove-history-and-legacy-i18n-sweep',
   'Drop orphaned i18n keys across 20 locales (R-005)',
   'Compile final key list by re-grepping en/translation.json after the deletion. Remove sidebar.history, sidebar.advanced, settings.advanced.*, settings.history.*, settings.recordingRetention.*, plus any sub-key referenced only by deleted components. Mirror across all 20 locale files in the same commit. Verifier: AC-005-a (npx tsx scripts/check-translations.ts) + AC-005-b (rg).',
   'pending'),

  ('remove-history-and-legacy-dep-cleanup',
   'Drop orphaned crates and npm packages (R-004)',
   'Invoke dep-hygiene skill. Run cargo machete in src-tauri/ and knip + depcheck at repo root. Manual review every flagged dep before removing from Cargo.toml / package.json. Re-run tools clean. Record decisions in journal.md. Verifiers: AC-004-a, AC-004-b.',
   'pending'),

  ('remove-history-and-legacy-qc-deletion',
   'QC: backend deletion compiles + zero stragglers',
   'AC-001-a, AC-001-b, AC-001-c. cd src-tauri && cargo test --no-run; pwsh scripts/eval-verifier.ps1 (deletion patterns) -ExpectZero; npm run build; grep regenerated bindings.ts.',
   'pending'),

  ('remove-history-and-legacy-qc-advanced',
   'QC: Advanced panel gone + survivors reachable',
   'AC-002-a, AC-002-b, AC-002-c. Live launch confirms no Advanced entry; rg confirms zero AdvancedSettings refs; manual reach-test of relocated children with screenshots to journal.md.',
   'pending'),

  ('remove-history-and-legacy-qc-legacy',
   'QC: legacy controls gone + repo-auditor clean',
   'AC-003-a, AC-003-b. Run repo-auditor agent; compare to pre-deletion baseline. rg for ExperimentalToggle / ExperimentalSimplifyMode / experimental_enabled returns zero.',
   'pending'),

  ('remove-history-and-legacy-qc-i18n',
   'QC: locale parity + zero i18n stragglers',
   'AC-005-a, AC-005-b. npx tsx scripts/check-translations.ts exits 0; rg for removed key prefixes returns zero.',
   'pending'),

  ('remove-history-and-legacy-qc-deps',
   'QC: dep-hygiene clean',
   'AC-004-a, AC-004-b. cargo machete clean; knip + depcheck clean.',
   'pending'),

  ('remove-history-and-legacy-qc-live',
   'QC: live app launches + editor undo/redo + eval gates pass',
   'AC-006-a, AC-006-b, AC-006-c, AC-006-d. Monitored launch reaches Vite ready with no panics; manual edit + undo + redo cycle recorded with timestamp + initials in journal.md; eval-edit-quality.ps1 + eval-audio-boundary.ps1 exit 0.',
   'pending'),

  ('remove-history-and-legacy-feature-qc',
   'Feature-level QC: coverage gate + eval-harness-runner',
   'pwsh scripts/check-feature-coverage.ps1 -Feature remove-history-and-legacy (exit 0). Then invoke eval-harness-runner agent for the full eval bundle.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('remove-history-and-legacy-baseline',         'remove-history-and-legacy-disambiguate'),
  ('remove-history-and-legacy-delete-backend',   'remove-history-and-legacy-baseline'),
  ('remove-history-and-legacy-delete-advanced',  'remove-history-and-legacy-baseline'),
  ('remove-history-and-legacy-delete-legacy',    'remove-history-and-legacy-delete-advanced'),
  ('remove-history-and-legacy-update-agents-md', 'remove-history-and-legacy-delete-backend'),
  ('remove-history-and-legacy-qc-deletion',      'remove-history-and-legacy-delete-backend'),
  ('remove-history-and-legacy-qc-deletion',      'remove-history-and-legacy-delete-advanced'),
  ('remove-history-and-legacy-qc-deletion',      'remove-history-and-legacy-delete-legacy'),
  ('remove-history-and-legacy-i18n-sweep',       'remove-history-and-legacy-qc-deletion'),
  ('remove-history-and-legacy-qc-i18n',          'remove-history-and-legacy-i18n-sweep'),
  ('remove-history-and-legacy-dep-cleanup',      'remove-history-and-legacy-qc-deletion'),
  ('remove-history-and-legacy-qc-deps',          'remove-history-and-legacy-dep-cleanup'),
  ('remove-history-and-legacy-qc-advanced',      'remove-history-and-legacy-qc-deletion'),
  ('remove-history-and-legacy-qc-legacy',        'remove-history-and-legacy-qc-deletion'),
  ('remove-history-and-legacy-qc-live',          'remove-history-and-legacy-qc-i18n'),
  ('remove-history-and-legacy-qc-live',          'remove-history-and-legacy-qc-deps'),
  ('remove-history-and-legacy-qc-live',          'remove-history-and-legacy-qc-advanced'),
  ('remove-history-and-legacy-qc-live',          'remove-history-and-legacy-qc-legacy'),
  ('remove-history-and-legacy-feature-qc',       'remove-history-and-legacy-qc-live');
