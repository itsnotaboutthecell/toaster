-- Task graph for example-pm-dryrun.
-- Ingest into the session SQL store with the `sql` tool when planning a real
-- feature; for the example we only declare the schema-shape so contributors
-- see what an actual breakdown looks like.

INSERT INTO todos (id, title, description, status) VALUES
  ('example-task-1',
   'Author the example bundle',
   'Create REQUEST/PRD/BLUEPRINT/tasks.sql/coverage.json/STATE.md under features/example-pm-dryrun/. Verifier: AC-001-a, AC-001-b, AC-002-a per coverage.json.',
   'done'),
  ('example-qc',
   'Verify the example passes the coverage gate',
   'Run scripts/check-feature-coverage.ps1 -Feature example-pm-dryrun and confirm exit 0.',
   'done');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('example-qc', 'example-task-1');
