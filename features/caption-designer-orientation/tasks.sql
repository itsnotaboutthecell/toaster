-- Task graph for caption-designer-orientation.
-- Ingest into the session SQL store with the `sql` tool.

INSERT INTO todos (id, title, description, status) VALUES
  ('caption-designer-orientation-mock-frame',
   'Build CaptionMockFrame vector component',
   'Create src/components/settings/captions/CaptionMockFrame.tsx. Single SVG sized to fill container; props: orientation. Renders rounded-rect outline (stroke #EEEEEE, radius proportional to short side), one horizontal + one vertical centerline (dashed low-opacity), 4 double-headed axis arrows along outer edges. NO text/pixel labels. Reference design: eval/fixtures/caption-mock-h-and-w.png (style only, do NOT replicate pixel labels). Verifier: AC-001-a, AC-001-b, AC-001-c per coverage.json.',
   'pending'),

  ('caption-designer-orientation-toggle',
   'Add Orientation select to CaptionSettings',
   'In src/components/settings/CaptionSettings.tsx: (a) add useState<"horizontal"|"vertical">("horizontal"); (b) add a <Select> control (mirroring the sample-text picker at lines 247-275) labelled settings.captions.preview.orientation.label; (c) compute aspectRatio = orientation === "horizontal" ? 16/9 : 9/16; (d) update the scale formula in CaptionPreviewPane (line 220) to use the SHORT axis denominator so pill visual size is comparable across orientations; (e) replace the <img src={captionPreviewFrame}> block at lines 286-296 with <CaptionMockFrame orientation={orientation} />; (f) remove the line-9 import. Verifier: AC-002-a, AC-002-b, AC-002-c, AC-002-d per coverage.json.',
   'pending'),

  ('caption-designer-orientation-i18n',
   'Add caption orientation i18n keys to all 22 locales',
   'Invoke the i18n-pruning skill. Add settings.captions.preview.orientation.label, settings.captions.preview.orientation.horizontal, settings.captions.preview.orientation.vertical to every src/i18n/locales/*/translation.json. English defaults: "Preview orientation" / "Horizontal" / "Vertical". Verifier: AC-004-b per coverage.json.',
   'pending'),

  ('caption-designer-orientation-asset-cleanup',
   'Delete caption-preview-frame.png and verify no remaining references',
   'After the new frame ships and live-launch is verified, delete src/assets/caption-preview-frame.png. Run rg "caption-preview-frame" src and confirm zero matches. Invoke the dep-hygiene skill (knip / depcheck) to confirm no orphaned imports remain. Verifier: AC-004-a per coverage.json.',
   'pending'),

  ('caption-designer-orientation-ssot-audit',
   'Confirm orientation never crosses the Tauri command boundary',
   'Code-review pass: confirm no new change_caption_*_setting Tauri command added; orientation lives only in React useState. Run cd src-tauri; cargo test caption_layout -- --nocapture; expect existing tests pass with zero source-code changes to src-tauri/src/managers/captions/. Verifier: AC-003-a, AC-003-b per coverage.json.',
   'pending'),

  ('caption-designer-orientation-qc',
   'QC: live launch in both orientations + lint + i18n gate',
   'Run (1) bun run scripts/check-translations.ts; expect exit 0. (2) npm run lint; expect exit 0. (3) cd src-tauri; cargo test caption_layout; expect exit 0. (4) pwsh scripts/launch-toaster-monitored.ps1 -ObservationSeconds 180; open Settings -> Captions, confirm vector frame replaces photo, toggle orientation H -> V -> H, verify pill stays inside frame at default + extreme settings (font 72, max-width 100%). Append results to journal.md.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('caption-designer-orientation-toggle',         'caption-designer-orientation-mock-frame'),
  ('caption-designer-orientation-toggle',         'caption-designer-orientation-i18n'),
  ('caption-designer-orientation-asset-cleanup',  'caption-designer-orientation-toggle'),
  ('caption-designer-orientation-ssot-audit',     'caption-designer-orientation-toggle'),
  ('caption-designer-orientation-qc',             'caption-designer-orientation-asset-cleanup'),
  ('caption-designer-orientation-qc',             'caption-designer-orientation-ssot-audit');
