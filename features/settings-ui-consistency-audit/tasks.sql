-- settings-ui-consistency-audit task graph
INSERT INTO todos (id, title, description, status) VALUES
 ('sua-testid-decorate', 'Decorate SettingContainer + CaptionPreviewPane with audit attributes', 'Add data-testid and data-setting-role attributes to src/components/ui/SettingContainer.tsx (row/label/control/description) and src/components/settings/captions/CaptionProfileShared.tsx (caption-preview-pane). Non-behavioural; no render changes. Satisfies AC-003-a foundation, AC-004-a/b/c foundation.', 'pending'),
 ('sua-settings-outer', 'Tag settings outer container on all 5 pages', 'Add data-testid="settings-outer" to the outer <main>/div of AboutSettings, ModelsSettings, PostProcessSettings, AdvancedSettings, and the captions tab wrapper. Non-behavioural. Enables AC-002-a baseline extraction.', 'pending'),
 ('sua-playwright-spec', 'Create tests/settingsUIAudit.spec.ts', 'Playwright spec with hardcoded ROUTES, VIEWPORTS, RULES. Implements padding baseline extraction, two-column rule, preview clamp, contract compliance, color HSL analysis. Emits violations via a custom reporter that writes raw.json. Satisfies AC-001-a/b, AC-002-a/b, AC-003-a/b/c, AC-004-a/b/c, AC-005-a/b/c/d.', 'pending'),
 ('sua-wrapper-script', 'Create scripts/audit-settings-ui.ps1', 'PowerShell wrapper: preflight dev server, run Playwright, post-process raw.json (sort), write audit.json + audit.md, copy screenshots, enforce 50 MB artefact cap, exit 0/1 on critical count, assert 120s runtime budget. Satisfies AC-001-c, AC-006-a/b/c.', 'pending'),
 ('sua-docs-appendix', 'Append Layout invariants section to docs/settings-placement.md', 'Section names the four rules (padding, two-column, preview-clamp, contract) and links to scripts/audit-settings-ui.ps1 + tests/settingsUIAudit.spec.ts. Append-only; does not modify existing content. AC-008-a.', 'pending'),
 ('sua-baseline-run', 'Run the audit once and commit the baseline report', 'Invoke scripts/audit-settings-ui.ps1 against the current tree; commit the resulting audit.json + audit.md + screenshots under features/settings-ui-consistency-audit/audit-report/. This is the reference baseline the follow-on fix feature consumes.', 'pending'),
 ('sua-static-gates', 'Static gates', 'cargo check -p toaster --lib, bun run build, bun run scripts/check-translations.ts, bun run lint, bun run check:file-sizes. All must exit 0. AC-007-a/b/c.', 'pending'),
 ('sua-coverage-gate', 'Run feature coverage gate', 'pwsh scripts/check-feature-coverage.ps1 -Feature settings-ui-consistency-audit must exit 0. No manual action; just confirms the bundle is well-formed before executor agents dispatch.', 'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
 ('sua-playwright-spec', 'sua-testid-decorate'),
 ('sua-playwright-spec', 'sua-settings-outer'),
 ('sua-wrapper-script', 'sua-playwright-spec'),
 ('sua-baseline-run', 'sua-wrapper-script'),
 ('sua-baseline-run', 'sua-docs-appendix'),
 ('sua-static-gates', 'sua-testid-decorate'),
 ('sua-static-gates', 'sua-settings-outer'),
 ('sua-coverage-gate', 'sua-static-gates'),
 ('sua-coverage-gate', 'sua-baseline-run');
