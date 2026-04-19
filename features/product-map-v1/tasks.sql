-- Task graph for product-map-v1.
--
-- This bundle produces NO implementation tasks. It is a discovery /
-- planning artifact whose deliverable is features/product-map-v1/PRD.md.
-- Each roadmap item in that PRD's Section 6 will scaffold its OWN
-- features/<slug>/ bundle (with its own tasks.sql) when the human is
-- ready to execute it -- see features/product-map-v1/BLUEPRINT.md
-- "How subsequent feature bundles branch off this map".
--
-- The single QC todo below exists only so the coverage gate can record
-- a verification step; it does not produce code.

INSERT INTO todos (id, title, description, status) VALUES
  ('product-map-v1-doc-review',
   'Doc review: confirm PRD sections 1-8 are complete',
   'Walk PRD.md sections 1-8 and confirm each AC-001-a..AC-008-a in coverage.json passes by inspection. No implementation. See features/product-map-v1/coverage.json for per-AC steps.',
   'pending'),
  ('product-map-v1-qc',
   'QC: run coverage gate',
   'Run scripts/check-feature-coverage.ps1 -Feature product-map-v1 and confirm exit 0. Document any deviation in journal.md.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('product-map-v1-qc', 'product-map-v1-doc-review');
