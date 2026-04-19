-- Task graph for {{SLUG}}.
-- Ingest into the session SQL store with the `sql` tool.

-- Schema: todos(id TEXT, title TEXT, description TEXT, status TEXT).
-- Allowed status values: 'pending', 'in_progress', 'done', 'blocked'.
-- Do not invent columns (no estimate_minutes, no owner, etc).
-- todo_deps schema: (todo_id TEXT, depends_on TEXT). No predecessor/successor.
INSERT INTO todos (id, title, description, status) VALUES
  ('{{SLUG}}-task-1',
   '<Task title>',
   '<Task description. Cite verifier: AC-NNN-x per coverage.json.>',
   'pending'),
  ('{{SLUG}}-qc',
   'QC: run coverage gate',
   'Run scripts/check-feature-coverage.ps1 -Feature {{SLUG}} and confirm exit 0.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('{{SLUG}}-qc', '{{SLUG}}-task-1');
