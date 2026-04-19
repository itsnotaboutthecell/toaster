-- Task graph for advanced-menu-restoration.
-- Ingest into the session SQL store with the `sql` tool.

INSERT INTO todos (id, title, description, status) VALUES
  ('advanced-menu-restoration-i18n',
   'Add settings.advanced.* i18n keys across all 22 locales',
   'Invoke the i18n-pruning skill. Add settings.advanced.title and settings.advanced.description (plus any nav-label key dictated by the Settings router) to every src/i18n/locales/*/translation.json. English strings: "Advanced" / "Configured-once controls (allow / discard word lists, etc.)". Verifier: AC-003-a, AC-003-b per coverage.json.',
   'pending'),

  ('advanced-menu-restoration-advanced-page',
   'Create AdvancedSettings page',
   'Add src/components/settings/advanced/AdvancedSettings.tsx modelled on src/components/settings/experimental/ExperimentalSettings.tsx. Single SettingsGroup titled settings.advanced.title, child rows render <DiscardWords grouped /> and <AllowWords grouped />. Wire into the Settings router the same way ExperimentalSettings is wired (locate the wiring site by reading the existing Settings router/nav before editing). Verifier: AC-002-a, AC-002-b, AC-002-c per coverage.json.',
   'pending'),

  ('advanced-menu-restoration-strip-editor',
   'Remove inline word-list block from EditorView.tsx',
   'In src/components/editor/EditorView.tsx delete (a) the SettingsGroup wrapper titled editor.sections.words at lines ~574-578, (b) the imports of DiscardWords and AllowWords at lines 26-27. Confirm via grep that DiscardWords/AllowWords are not referenced anywhere under src/components/editor/. Verifier: AC-001-a, AC-001-b per coverage.json.',
   'pending'),

  ('advanced-menu-restoration-audit',
   'Audit current settings + write placement heuristic doc',
   'Append an "Audit" section to features/advanced-menu-restoration/journal.md containing (a) the frequency-of-use heuristic in one paragraph and (b) a markdown table covering every component under src/components/settings/ (excluding experimental/ and debug/) with columns: Component | Setting key | Recommended placement | Rationale. Create docs/settings-placement.md restating the heuristic in user-readable form and linking to the audit table. Verifier: AC-004-a, AC-004-b per coverage.json.',
   'pending'),

  ('advanced-menu-restoration-qc',
   'QC: live launch + i18n gate + lint',
   'Run (1) bun run scripts/check-translations.ts; expect exit 0. (2) npm run lint; expect exit 0. (3) pwsh scripts/launch-toaster-monitored.ps1 -ObservationSeconds 120; open the editor view (no inline word lists), open Settings -> Advanced (both components render and accept add/remove). Append results to journal.md.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('advanced-menu-restoration-advanced-page', 'advanced-menu-restoration-i18n'),
  ('advanced-menu-restoration-strip-editor',  'advanced-menu-restoration-advanced-page'),
  ('advanced-menu-restoration-audit',         'advanced-menu-restoration-strip-editor'),
  ('advanced-menu-restoration-qc',            'advanced-menu-restoration-audit');
