# Feature request: sidebar-5-lane-restructure

## 1. Problem & Goals

The sidebar has grown to 7 entries (editor, models, postProcessing, export, experimental, advanced, about). User feedback flagged this as clutter and a design regression. Advanced-class settings (Performance, Captions) currently live on the Models page, violating the "configured-once" heuristic codified in `docs/settings-placement.md`. Experimental features surface as a top-level sidebar lane with no master gate, so a stale per-flag toggle can remain on silently.

## 2. Desired outcome & acceptance criteria

- Sidebar has exactly 5 items: Editor, Models, Post-Process, Advanced, About.
- Advanced renders 5 groups in order: Words, Performance, Captions, Export, Experimental.
- Models page is pure catalog again (cards + language filter). Performance and Captions groups removed from it.
- Export content relocates unchanged into Advanced.
- A new `experimental_enabled: bool` (default `false`) gates the per-flag list. When OFF, the whole per-flag block is hidden AND every individual experiment boolean returns `false` through the getter (defence-in-depth).
- All 20 locale JSONs stay in sync; `check-translations.ts` exits 0; lint + tsc + `cargo check -p toaster --lib` green; live-app QC passes.

## 3. Scope boundaries

In scope: UI relocation, sidebar config change, new master setting + defence-in-depth getter, i18n add/remove across 20 locales.

Out of scope: keep-segment, export FFmpeg pipeline, caption rendering logic, per-experiment behavior, Post-Process provider logic. No backend manager changes beyond adding one bool field and its getter.

## 4. References to existing code

- `src/components/Sidebar.tsx:28-71` — SECTIONS_CONFIG
- `src/components/settings/models/ModelsSettings.tsx:367-375` — Performance + Captions groups to relocate
- `src/components/settings/advanced/AdvancedSettings.tsx` — target host for 5 groups
- `src/components/settings/export/ExportSettings.tsx` — component to relocate into Advanced
- `src/components/settings/experimental/ExperimentalSettings.tsx` — pattern to adapt into a gated group
- `src/lib/experiments.ts` — experiment registry; getter must be wrapped
- `src-tauri/src/settings/defaults.rs` — add `experimental_enabled: bool`
- `src/i18n/locales/*/translation.json` — 20 files

## 5. Edge cases & constraints

- Existing users upgrading from pre-change builds: settings migration must preserve any already-set per-flag experiment booleans but force-hide them when master is OFF (defence-in-depth getter handles this without migration rewriting stored values).
- `check-translations.ts` compares English as reference; other 19 locales need matching keys.
- `SECTIONS_CONFIG` is `as const satisfies ...` — removing keys changes the `SidebarSection` union type; any persisted "active section" in UI state must be migrated if it pointed at `export` or `experimental`.
- File-size cap 800 lines — `AdvancedSettings.tsx` will gain 4 groups; extract sub-components if it breaches.

## 6. Data model

```rust
// AppSettings add:
experimental_enabled: bool, // default false
```

Translation keys:
- REMOVE: `sidebar.export`, `sidebar.experimental`
- ADD: `settings.advanced.groups.{words,performance,captions,export,experimental}.{title,description}`
- ADD: `settings.advanced.experimentalMaster.{title,description}`

## Q&A (resolved)

- Export placement: Advanced (user confirmed).
- Experimental gating style: gated — master OFF hides the whole per-flag list (user confirmed).
