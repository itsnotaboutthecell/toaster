-- Task graph for brand-title-sizing.
-- Ingest into the session SQL store with the `sql` tool.

INSERT INTO todos (id, title, description, status) VALUES
  ('brand-title-sizing-sidebar-wordmark',
   'Enlarge wordmark in Sidebar.tsx',
   'Edit src/components/Sidebar.tsx line 76: change className from "w-[120px] m-4" to "w-[144px] mx-0 my-4". Asset import on line 4 is unchanged. Verifies AC-001-a, AC-001-b, AC-001-c, AC-003-c per coverage.json.',
   'pending'),

  ('brand-title-sizing-app-padding',
   'Drop horizontal padding on main content wrapper',
   'Edit src/App.tsx line 205: change className from "flex flex-col items-center p-4 gap-4" to "flex flex-col items-center py-4 gap-4". Verifies AC-002-b per coverage.json.',
   'pending'),

  ('brand-title-sizing-editor-cap',
   'Raise editor max-width cap',
   'Edit src/components/editor/EditorView.tsx line 400: change "max-w-4xl" to "max-w-6xl". Keep "w-full mx-auto space-y-6". Verifies AC-002-a, AC-003-b, AC-004-a per coverage.json.',
   'pending'),

  ('brand-title-sizing-settings-caps',
   'Raise settings panes max-width caps',
   'Edit max-w-3xl -> max-w-5xl on: src/components/settings/about/AboutSettings.tsx:31, src/components/settings/advanced/AdvancedSettings.tsx:20, src/components/settings/history/HistorySettings.tsx:242, src/components/settings/models/ModelsSettings.tsx:200 AND :209. Keep "w-full mx-auto" intact on each. Verifies AC-002-c, AC-004-b per coverage.json.',
   'pending'),

  ('brand-title-sizing-qc-visual',
   'QC: visual sizing checks (R-001, R-002)',
   'Run pwsh scripts/launch-toaster-monitored.ps1 at 1280x800 and follow the manual steps in coverage.json for AC-001-a, AC-001-b, AC-001-c, AC-002-a, AC-002-b, AC-002-c.',
   'pending'),

  ('brand-title-sizing-qc-responsive',
   'QC: responsive checks (R-003)',
   'Run pwsh scripts/launch-toaster-monitored.ps1 and follow the manual steps in coverage.json for AC-003-a (720x800), AC-003-b (1920x1080), AC-003-c (720x800).',
   'pending'),

  ('brand-title-sizing-qc-regression',
   'QC: editor + settings regression (R-004)',
   'Live-app smoke per coverage.json AC-004-a, AC-004-b. Then run bun run test:e2e and confirm 0 failures (AC-004-c).',
   'pending'),

  ('brand-title-sizing-qc-gates',
   'QC: discipline gates (R-005)',
   'Run bun scripts/check-file-sizes.ts (AC-005-a) and the git-diff color-literal grep per coverage.json AC-005-b.',
   'pending'),

  ('brand-title-sizing-feature-qc',
   'Feature QC: coverage gate',
   'Run pwsh scripts/check-feature-coverage.ps1 -Feature brand-title-sizing and confirm exit 0.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  -- visual QC depends on the four implementation tasks
  ('brand-title-sizing-qc-visual',     'brand-title-sizing-sidebar-wordmark'),
  ('brand-title-sizing-qc-visual',     'brand-title-sizing-app-padding'),
  ('brand-title-sizing-qc-visual',     'brand-title-sizing-editor-cap'),
  ('brand-title-sizing-qc-visual',     'brand-title-sizing-settings-caps'),
  -- responsive QC depends on the same implementation set
  ('brand-title-sizing-qc-responsive', 'brand-title-sizing-sidebar-wordmark'),
  ('brand-title-sizing-qc-responsive', 'brand-title-sizing-app-padding'),
  ('brand-title-sizing-qc-responsive', 'brand-title-sizing-editor-cap'),
  ('brand-title-sizing-qc-responsive', 'brand-title-sizing-settings-caps'),
  -- regression QC depends on editor + settings caps + app padding
  ('brand-title-sizing-qc-regression', 'brand-title-sizing-app-padding'),
  ('brand-title-sizing-qc-regression', 'brand-title-sizing-editor-cap'),
  ('brand-title-sizing-qc-regression', 'brand-title-sizing-settings-caps'),
  -- discipline gates run after all four edits
  ('brand-title-sizing-qc-gates',      'brand-title-sizing-sidebar-wordmark'),
  ('brand-title-sizing-qc-gates',      'brand-title-sizing-app-padding'),
  ('brand-title-sizing-qc-gates',      'brand-title-sizing-editor-cap'),
  ('brand-title-sizing-qc-gates',      'brand-title-sizing-settings-caps'),
  -- feature QC waits on every group QC
  ('brand-title-sizing-feature-qc',    'brand-title-sizing-qc-visual'),
  ('brand-title-sizing-feature-qc',    'brand-title-sizing-qc-responsive'),
  ('brand-title-sizing-feature-qc',    'brand-title-sizing-qc-regression'),
  ('brand-title-sizing-feature-qc',    'brand-title-sizing-qc-gates');
