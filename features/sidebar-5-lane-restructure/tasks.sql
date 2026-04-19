-- sidebar-5-lane-restructure task graph
INSERT INTO todos (id, title, description, status) VALUES
 ('slr-settings', 'Add experimental_enabled to AppSettings', 'Edit src-tauri/src/settings/defaults.rs: add experimental_enabled: bool (default false) to AppSettings + initializer. Regenerate TS bindings. cargo check -p toaster --lib must stay green.', 'pending'),
 ('slr-getter-rust', 'Rust defence-in-depth getter', 'Add pub fn is_experiment_enabled(settings, key) in src-tauri/src/settings/mod.rs that returns false when experimental_enabled == false, else returns stored value. Expose at module root.', 'pending'),
 ('slr-getter-tests', 'Cargo tests for R-006 getter', 'Add tests experiment_getter_returns_false_when_master_disabled and experiment_getter_returns_stored_value_when_master_enabled. cargo test -p toaster --lib experiment_getter must pass.', 'pending'),
 ('slr-hook-ts', 'Frontend useExperiment hook + migration', 'Add src/hooks/useExperiment.ts that applies the same master-gating rule. Migrate every caller currently reading an experiment boolean via useSettings/getSetting to useExperiment.', 'pending'),
 ('slr-advanced-page', 'Compose Advanced with 5 groups', 'AdvancedSettings.tsx renders SettingsGroup for Words, Performance, Captions, Export, Experimental in that order. Reuses existing leaf components. Split sub-groups into co-located files if >800 lines.', 'pending'),
 ('slr-experimental-group', 'Experimental group component with master toggle', 'New ExperimentalGroup.tsx inside src/components/settings/advanced/. Renders a ToggleSwitch bound to experimental_enabled + (when on) the existing per-flag list from @/lib/experiments. Keeps the existing Alert banner when ON.', 'pending'),
 ('slr-models-strip', 'Strip Performance + Captions groups from Models', 'Delete the two SettingsGroup blocks at the bottom of ModelsSettings.tsx. Remove now-unused imports (ModelUnloadTimeoutSetting, CaptionSettings). rg assertion in AC-003-a must pass.', 'pending'),
 ('slr-sidebar-strip', 'Remove export + experimental from Sidebar', 'Delete those two SECTIONS_CONFIG entries in src/components/Sidebar.tsx. Add defensive fallback: when activeSection is not a known key, default to editor. Drop unused icon imports.', 'pending'),
 ('slr-index-cleanup', 'Clean settings/index.ts exports + dep-hygiene', 'Remove ExperimentalSettings and ExportSettings page-level exports if no longer referenced. Run knip/depcheck mentally or via the dep-hygiene skill. Leaf components (CaptionSettings, ExportSettings-as-leaf) stay exported.', 'pending'),
 ('slr-i18n', 'Update 20 locale files', 'Remove sidebar.export and sidebar.experimental from all 20 locales. Add settings.advanced.groups.{words,performance,captions,export,experimental}.{title,description} and settings.advanced.experimentalMaster.{title,description}. Use one scripted pass. bun run scripts/check-translations.ts must exit 0.', 'pending'),
 ('slr-static-gates', 'Run static gates', 'npm run lint, npx tsc --noEmit, cargo check -p toaster --lib, bun run scripts/check-translations.ts - all exit 0.', 'pending'),
 ('slr-live-qc', 'Live-app QC', 'pwsh scripts/launch-toaster-monitored.ps1 -ObservationSeconds 180. Exercise AC-009-a/b/c. Record evidence in journal.md.', 'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
 ('slr-getter-rust', 'slr-settings'),
 ('slr-getter-tests', 'slr-getter-rust'),
 ('slr-hook-ts', 'slr-settings'),
 ('slr-advanced-page', 'slr-hook-ts'),
 ('slr-experimental-group', 'slr-hook-ts'),
 ('slr-advanced-page', 'slr-experimental-group'),
 ('slr-models-strip', 'slr-advanced-page'),
 ('slr-sidebar-strip', 'slr-advanced-page'),
 ('slr-index-cleanup', 'slr-sidebar-strip'),
 ('slr-i18n', 'slr-advanced-page'),
 ('slr-static-gates', 'slr-i18n'),
 ('slr-static-gates', 'slr-getter-tests'),
 ('slr-static-gates', 'slr-models-strip'),
 ('slr-static-gates', 'slr-index-cleanup'),
 ('slr-live-qc', 'slr-static-gates');
