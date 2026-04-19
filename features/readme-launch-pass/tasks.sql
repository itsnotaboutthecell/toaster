-- Task graph for readme-launch-pass.
-- Ingest into the session SQL store with the `sql` tool.
--
-- Schema: todos(id, title, description, status) with status in
-- {pending, in_progress, done, blocked}. todo_deps(todo_id, depends_on).
-- Do not add columns.

INSERT INTO todos (id, title, description, status) VALUES
  ('readme-launch-pass-draft',
   'Draft README.md rewrite per BLUEPRINT section structure',
   'Rewrite README.md end-to-end following the 14-section outline in features/readme-launch-pass/BLUEPRINT.md. Hero tagline, [TODO: screenshot ...] placeholders at the flagged sites, What Toaster does today bullets, Quickstart, Build, Launch protocol (with portable-mode note citing src-tauri/src/commands/mod.rs:26-30), Evals, Platform support, Roadmap (link only, no inlined table), Contributing, License, Acknowledgments (fork-ack footer - ONLY place Handy may appear). Verifiers: AC-001-a, AC-001-b, AC-001-c, AC-001-d per coverage.json.',
   'pending'),
  ('readme-launch-pass-purge-handy',
   'Purge Handy references and badge links to upstream',
   'After the draft lands, confirm zero occurrences of the string "Handy" outside the Acknowledgments footer, and zero image/badge links whose URL contains "Handy" (case-insensitive). Verifiers: AC-002-a, AC-003-a per coverage.json.',
   'pending'),
  ('readme-launch-pass-verify-links',
   'Verify every internal relative link resolves',
   'Run the AC-002-b one-liner from features/readme-launch-pass/coverage.json and fix any unresolved targets by either pointing at the real file or deleting the link. Verifier: AC-002-b per coverage.json.',
   'pending'),
  ('readme-launch-pass-qc-structure',
   'QC: README structure + launch-asset links',
   'Execute the manual verifier commands for AC-001-a, AC-001-b, AC-001-c, AC-001-d from features/readme-launch-pass/coverage.json and confirm each prints OK (or the expected first-three-lines content for AC-001-a).',
   'pending'),
  ('readme-launch-pass-qc-handy-purge',
   'QC: Handy purge + badge hygiene',
   'Execute the manual verifier commands for AC-002-a and AC-003-a from features/readme-launch-pass/coverage.json and confirm each prints the pass tokens described in the steps arrays.',
   'pending'),
  ('readme-launch-pass-qc-links',
   'QC: link integrity',
   'Execute the manual verifier command for AC-002-b from features/readme-launch-pass/coverage.json and confirm the output is exactly OK.',
   'pending'),
  ('readme-launch-pass-feature-qc',
   'Feature QC: coverage + tasks gates',
   'Run pwsh scripts/feature/check-feature-coverage.ps1 -Feature readme-launch-pass and pwsh scripts/feature/check-feature-tasks.ps1 -Feature readme-launch-pass and confirm both print [OK].',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('readme-launch-pass-purge-handy',    'readme-launch-pass-draft'),
  ('readme-launch-pass-verify-links',   'readme-launch-pass-draft'),
  ('readme-launch-pass-qc-structure',   'readme-launch-pass-draft'),
  ('readme-launch-pass-qc-handy-purge', 'readme-launch-pass-purge-handy'),
  ('readme-launch-pass-qc-links',       'readme-launch-pass-verify-links'),
  ('readme-launch-pass-feature-qc',     'readme-launch-pass-qc-structure'),
  ('readme-launch-pass-feature-qc',     'readme-launch-pass-qc-handy-purge'),
  ('readme-launch-pass-feature-qc',     'readme-launch-pass-qc-links');