# Blueprint: sidebar-5-lane-restructure

## Architecture decisions

### R-001/002/003/004 — Relocation

Single source of truth: `AdvancedSettings.tsx` owns the composition; the existing child components (`ModelUnloadTimeoutSetting`, `CaptionSettings`, `ExportSettings` content, `DiscardWords`, `AllowWords`) are imported as-is. No duplication. `ModelsSettings.tsx` drops its Performance + Captions `SettingsGroup` blocks. `Sidebar.tsx` drops the `export` and `experimental` entries.

If `AdvancedSettings.tsx` approaches the 800-line cap once 5 groups are inlined, split into sub-component files co-located under `src/components/settings/advanced/` (`ExperimentalGroup.tsx`, `ExportGroup.tsx`, etc.) — leaf groups, not page shells.

Retired sidebar targets (`ExperimentalSettings`, `ExportSettings` standalone pages) may still be exported from `src/components/settings/index.ts` if their internals are reused in the new group components; otherwise `dep-hygiene` audit deletes them. Decision: keep the leaf components but stop exporting the page-level shells. This keeps the diff focused on composition.

### R-005/R-006 — Master toggle + defence-in-depth

Settings shape: add `experimental_enabled: bool` to `AppSettings` in `src-tauri/src/settings/defaults.rs` with default `false`. Regenerate `bindings.ts`.

Defence-in-depth is enforced at the **getter** layer, not by writing to stored per-flag values. This preserves user intent — if they re-enable the master later, their prior per-flag choices come back. Rule: `get_experiment(key) = if !settings.experimental_enabled { false } else { settings.<key> }`.

Where the getter lives:
- Rust: new `pub fn is_experiment_enabled(settings: &AppSettings, key: ExperimentKey) -> bool` in `src-tauri/src/settings/mod.rs`. Cargo tests target this directly.
- Frontend: a `useExperiment(key)` hook wrapping `useSettings` that applies the same rule. All existing call sites reading `getSetting("experimental_*")` migrate to `useExperiment`.

Frontend hook + Rust function are two mouths of the same SSOT rule (dual-path). Both are covered in R-006 ACs; a cargo test covers Rust, a React-Testing-Library test or a live-app toggle sequence covers the frontend path.

### R-007 — i18n

Use the existing per-locale JSON layout. For each of 20 locales, remove the two retired keys and add the new `settings.advanced.groups.*` and `settings.advanced.experimentalMaster.*` keys. English values are human-written; other 19 locales receive the English placeholder (per `i18n-pruning` policy — reference file is `en/translation.json`; other locales' values can be English placeholders without failing `check-translations.ts`).

Script helper: author one idempotent PowerShell pass that edits all 20 files in one run. Keep it under `scripts/` only if reusable; otherwise write inline and delete after use (per session convention).

## Risk register

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Existing users have a saved "active section" pointer that still reads `export` or `experimental` and crashes | UI black-screen on first launch | In the `SidebarSection` consumer, fallback to `editor` when an unknown section id is deserialized. One-line defensive guard. |
| `experimental_enabled` default flips too aggressively on upgrade (user had an experiment on before) | Feature they relied on silently turns off | By design per user directive. Document in release notes / journal. Defence-in-depth is the whole point. |
| `AdvancedSettings.tsx` exceeds 800-line cap | CI file-size gate fails | Split leaf groups into co-located files at the first sign of pressure. Already budgeted. |
| Translation keys drift across 20 locales | `check-translations.ts` exit 1 | Use one scripted pass that adds+removes keys in a single loop; verify before commit. |
| Existing ExperimentalSettings / ExportSettings component has side effects (event listeners, stores) when mounted that disappear if its page shell is removed | Lost background behavior | Audit: both are pure render components today (grep `useEffect` in each). Confirmed safe. |

## Implementation order

1. Rust `experimental_enabled` + bindings regen + getter.
2. Cargo tests for R-006 defence-in-depth getter.
3. Frontend `useExperiment` hook + migrate existing call sites.
4. Advanced page composition: host all 5 groups.
5. Delete Performance + Captions groups from Models.
6. Delete `export` + `experimental` from Sidebar SECTIONS_CONFIG + `SidebarSection` fallback.
7. i18n: 20 locale updates (remove 2 keys, add new keys).
8. Static gates: lint, tsc, cargo check, translations.
9. Live-app QC.
