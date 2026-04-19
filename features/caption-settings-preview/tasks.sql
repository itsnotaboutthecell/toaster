-- Task graph for caption-settings-preview.
-- Ingest into the session SQL store with the `sql` tool.

INSERT INTO todos (id, title, description, status) VALUES
  ('csp-extract-pill',
   'Extract CaptionPill named export from CaptionOverlay',
   'In src/components/player/CaptionOverlay.tsx, extract the inner caption-pill JSX into a new named export CaptionPill. CaptionOverlay continues to default-export and renders <CaptionPill ... /> internally. Preserve byte-identical player render output. Verifier: AC-004-a, AC-004-b per coverage.json. See tasks/csp-extract-pill/context.md.',
   'pending'),

  ('csp-i18n-keys',
   'Add five preview-pane i18n keys to all 20 locales',
   'Add settings.captions.preview.heading, settings.captions.preview.sampleLegend, and settings.captions.preview.sample.{short,twoLine,long} to every src/i18n/locales/*/translation.json. English short value is the literal "looking crispy" (AC-003-a). Use the i18n-pruning skill to keep scripts/check-translations.ts green. Verifier: AC-003-a, AC-003-b per coverage.json. See tasks/csp-i18n-keys/context.md.',
   'pending'),

  ('csp-preview-pane',
   'Add CaptionPreviewPane to CaptionSettings.tsx',
   'In src/components/settings/CaptionSettings.tsx, add CaptionPreviewPane as the first child of the caption section. Import CaptionPill from src/components/player/CaptionOverlay.tsx. Add selectedSampleKey state and a Select for sample text. Sticky position, aspect-ratio 16:9, contain-fit. Background: bundled first-frame PNG of eval/fixtures/toaster_example.mp4; fallback flat #1a1a1a on load error. Read settings via the existing useSettings hook only. No debounce/throttle/setTimeout/requestIdleCallback. Verifier: AC-001-a, AC-002-a, AC-002-b, AC-002-c, AC-003-a, AC-003-b, AC-003-c, AC-004-a, AC-004-c, AC-005-a, AC-005-b, AC-005-c per coverage.json. See tasks/csp-preview-pane/context.md.',
   'pending'),

  ('csp-qc-static',
   'QC: static-vs-camera (R-001) live-app check',
   'Manual live-app verification for AC-001-a (no camera prompt; static frame visible). Use scripts/launch-toaster-monitored.ps1 and follow steps in coverage.json AC-001-a.',
   'pending'),

  ('csp-qc-scope',
   'QC: scope verification (R-001) - camera deferral documented',
   'Confirm PRD.md and BLUEPRINT.md both mark camera-based preview out of scope and name the follow-up slug. Verifier: AC-001-b per coverage.json.',
   'pending'),

  ('csp-qc-placement',
   'QC: placement & geometry live-app check',
   'Manual live-app verification for AC-002-a, AC-002-b, AC-002-c. Use scripts/launch-toaster-monitored.ps1 and follow steps in coverage.json.',
   'pending'),

  ('csp-qc-text',
   'QC: placeholder text + fixture fallback live-app check',
   'Manual live-app verification for AC-003-a, AC-003-b, AC-003-c. Includes the rename-asset step for AC-003-c.',
   'pending'),

  ('csp-qc-reuse',
   'QC: single-source-of-truth source-tree assertions',
   'Source-tree verification for AC-004-a, AC-004-b, AC-004-c. Confirm CaptionPill is imported from CaptionOverlay.tsx, no new Caption*Render* file under src/, and no parallel Zustand store.',
   'pending'),

  ('csp-qc-latency',
   'QC: live-update latency live-app + grep check',
   'Manual live-app verification for AC-005-a, AC-005-c plus the grep assertion for AC-005-b (no debounce/throttle/setTimeout/requestIdleCallback in the wiring).',
   'pending'),

  ('csp-qc-player-regression',
   'QC: player caption rendering unchanged after CaptionPill extraction',
   'A/B compare player caption rendering on eval/fixtures/toaster_example.mp4 before and after csp-extract-pill. Confirm zero visible difference. Risk register: extraction subtly changes player render output.',
   'pending'),

  ('feature-qc',
   'QC: coverage gate green',
   'Run pwsh scripts/check-feature-coverage.ps1 -Feature caption-settings-preview and confirm exit 0.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  -- preview-pane depends on the pill extraction and the i18n keys
  ('csp-preview-pane', 'csp-extract-pill'),
  ('csp-preview-pane', 'csp-i18n-keys'),

  -- live-app QC tasks depend on the preview pane being implemented
  ('csp-qc-static',     'csp-preview-pane'),
  ('csp-qc-placement',  'csp-preview-pane'),
  ('csp-qc-text',       'csp-preview-pane'),
  ('csp-qc-latency',    'csp-preview-pane'),

  -- reuse QC depends on extraction + preview-pane wiring
  ('csp-qc-reuse', 'csp-extract-pill'),
  ('csp-qc-reuse', 'csp-preview-pane'),

  -- scope QC only depends on PRD/BLUEPRINT being authored (already true)
  -- (no production-code dep)

  -- player regression depends on the extraction
  ('csp-qc-player-regression', 'csp-extract-pill'),

  -- final coverage gate depends on all QC tasks
  ('feature-qc', 'csp-qc-static'),
  ('feature-qc', 'csp-qc-scope'),
  ('feature-qc', 'csp-qc-placement'),
  ('feature-qc', 'csp-qc-text'),
  ('feature-qc', 'csp-qc-reuse'),
  ('feature-qc', 'csp-qc-latency'),
  ('feature-qc', 'csp-qc-player-regression');
