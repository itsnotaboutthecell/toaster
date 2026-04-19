-- Task graph for keyboard-shortcuts-cheatsheet.
-- Ingest into the session SQL store with the `sql` tool.
-- Schema: todos(id, title, description, status) and
-- todo_deps(todo_id, depends_on). Status values: pending,
-- in_progress, done, blocked.

INSERT INTO todos (id, title, description, status) VALUES
  ('keyboard-shortcuts-cheatsheet-registry',
   'Build the shortcut registry and platform helpers',
   'Create src/lib/shortcuts/{registry.ts, platform.ts, index.ts}. Define ShortcutDefinition and ShortcutContext types per BLUEPRINT.md. Populate `shortcuts` with every chord currently wired at src/components/editor/EditorView.tsx:79-160, src/components/editor/TranscriptEditor.tsx:161-173, and src/App.tsx:67-89. Add the ?/Ctrl+/ triggers and a placeholder global Esc gated by cheatsheetOpen. Export groupShortcuts and formatChord. Verifier: AC-001-a per coverage.json.',
   'pending'),
  ('keyboard-shortcuts-cheatsheet-dispatcher',
   'Build the keydown dispatcher hook',
   'Create src/lib/shortcuts/dispatcher.ts with useShortcutDispatcher(ctx). Single window.keydown listener, focus-in-input guard honouring ignoreInEditable, short-circuit on cheatsheetOpen. Do not rewire any call sites yet. Verifier: AC-001-a per coverage.json.',
   'pending'),
  ('keyboard-shortcuts-cheatsheet-store',
   'Add cheatsheetOpen flag + setters to the editor store',
   'Extend the editor store (see existing modal flags for pattern) with cheatsheetOpen: boolean, openCheatsheet(), closeCheatsheet(). No persisted state. Verifier: AC-001-b per coverage.json.',
   'pending'),
  ('keyboard-shortcuts-cheatsheet-modal',
   'Build the ShortcutsCheatsheet modal component',
   'Create src/components/modals/ShortcutsCheatsheet.tsx. Read shortcuts from src/lib/shortcuts/registry.ts, render grouped list via groupShortcuts, format chords via formatChord(). Mirror src/components/ui/Alert.tsx styling + the outside-click pattern in src/components/editor/TranscriptEditor.tsx Find overlay. Own Esc via capture-phase listener so it runs before the editor global Esc. Mount once in the top-level layout. Verifier: AC-001-b, AC-001-c, AC-002-a per coverage.json.',
   'pending'),
  ('keyboard-shortcuts-cheatsheet-refactor-callsites',
   'Refactor the three existing keydown sites onto the dispatcher',
   'Replace the inline keydown useEffect blocks at src/components/editor/EditorView.tsx:79-160, src/components/editor/TranscriptEditor.tsx:161-173, and src/App.tsx:67-89 with calls to useShortcutDispatcher(ctx). All existing chord semantics must remain identical. After this task, `rg "addEventListener\\(.keydown" src/` returns exactly one hit, inside dispatcher.ts. Verifier: AC-001-a, AC-001-c per coverage.json.',
   'pending'),
  ('keyboard-shortcuts-cheatsheet-i18n',
   'Add shortcuts.* i18n keys across all 20 locales',
   'Add shortcuts.groups.{editor,playback,navigation} and shortcuts.descriptions.<id> to every src/i18n/locales/*/translation.json. Seed non-English values with the English string per repo policy. Run `bun scripts/check-translations.ts` and confirm exit 0. Verifier: AC-002-b per coverage.json.',
   'pending'),
  ('keyboard-shortcuts-cheatsheet-qc',
   'QC: run coverage + translations + manual walkthrough',
   'Run pwsh scripts/feature/check-feature-coverage.ps1 -Feature keyboard-shortcuts-cheatsheet (must exit 0), pwsh scripts/feature/check-feature-tasks.ps1 -Feature keyboard-shortcuts-cheatsheet (must exit 0), bun scripts/check-translations.ts (must exit 0), and execute the manual steps for AC-001-a, AC-001-b, AC-001-c, AC-002-a, AC-003-a exactly as listed in coverage.json.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('keyboard-shortcuts-cheatsheet-dispatcher', 'keyboard-shortcuts-cheatsheet-registry'),
  ('keyboard-shortcuts-cheatsheet-modal', 'keyboard-shortcuts-cheatsheet-registry'),
  ('keyboard-shortcuts-cheatsheet-modal', 'keyboard-shortcuts-cheatsheet-store'),
  ('keyboard-shortcuts-cheatsheet-refactor-callsites', 'keyboard-shortcuts-cheatsheet-dispatcher'),
  ('keyboard-shortcuts-cheatsheet-refactor-callsites', 'keyboard-shortcuts-cheatsheet-modal'),
  ('keyboard-shortcuts-cheatsheet-i18n', 'keyboard-shortcuts-cheatsheet-registry'),
  ('keyboard-shortcuts-cheatsheet-qc', 'keyboard-shortcuts-cheatsheet-refactor-callsites'),
  ('keyboard-shortcuts-cheatsheet-qc', 'keyboard-shortcuts-cheatsheet-i18n');
