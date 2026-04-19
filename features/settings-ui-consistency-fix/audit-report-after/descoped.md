# Descoped in this bundle

## R-005-missing-description (84 rows remain)

**Authoritative status**: descoped pending user triage.

Per user feedback on the PM review:

> I don't have enough information to make an informed decision yet.

The 84 `R-005-missing-description` violations are all rows where the
underlying control does not yet have an i18n-backed one-line
description. A blanket auto-generated description would violate the
Settings UI contract ("human-readable label and one-line description")
by introducing low-signal filler text. Writing the right descriptions
requires per-row product review (what does the control actually do, in
one sentence, in the user's voice) that is out of scope for this fix.

### Next step

Open a follow-up feature bundle (e.g. `settings-descriptions-pass`)
to:

1. Walk every `R-005-missing-description` row in
   `audit-report/audit.json`.
2. For each, add an i18n key pair (label + description) and wire it
   into the corresponding `SettingContainer`.
3. Re-run `scripts/migrate/audit-settings-ui.ps1` targeting the same report
   path; this bundle's `coverage.json` is the template.

Until that bundle lands, this feature is considered complete with
`critical=0` and all structural / contrast / slider / export-layout
clusters resolved.

## Raw counts

- Critical: 0 (was 1)
- Major: 84 (was 130; all remaining are `R-005-missing-description`)
- Minor: 0
