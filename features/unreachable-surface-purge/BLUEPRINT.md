# Blueprint: Unreachable Surface Purge

## Architecture decisions

- R-001 (`experimental_simplify_mode` delete): follow the pattern of
  `remove-history-and-legacy` -- remove the field from
  `src-tauri/src/settings/types.rs`, drop any serde default, and
  inline `false` at each of the five consumer sites in
  `src-tauri/src/commands/waveform/mod.rs:299-366` before deleting the
  helper `settings_experimental_simplify_mode_enabled` (line 299) and
  any `experimental_simplify_mode: bool` parameter in
  `canonical_keep_segments_for_media_with_options` and
  `select_raw_keep_segments_for_media`. The `if
  experimental_simplify_mode` branch at line 366 is deleted entirely
  (dead branch after flag deletion). Authority for keep-segment
  selection stays on the backend; this change only removes the
  always-false alternate path.
- R-002 (i18n orphans): follow the `i18n-pruning` skill. Scope every
  edit to the `sidebar.*` subtree; leave `settings.general.*`
  untouched. Apply the same key removal to all 20 locale files
  (`src/i18n/locales/*/translation.json`).
- R-003 (apple stub): recon showed the stub is already absent from
  `src-tauri/src/commands/app_settings.rs`. Chunk C is
  verification-first: run the `rg` gates listed in coverage.json, and
  only touch files if residue surfaces. Any frontend `invoke(
  'check_apple_intelligence_available')` caller is deleted; regenerate
  `src/bindings.ts` with `cargo test -p tauri-bindings` (or the
  project's binding-regen task) if needed.
- R-004 (post-processing reachable): `sidebar.postProcessing` is
  already present in `src/components/Sidebar.tsx:39-44`
  `SECTIONS_CONFIG`. The work is: (a) add a small "Local LLM only"
  label component at the top of
  `src/components/settings/post-processing/PostProcessingSettings.tsx`
  citing `src-tauri/src/llm_client.rs is_local_host`; (b) confirm the
  UMC `ModelsSettings` embed mount (landed by
  `umc-post-process-embed`) still renders.
- R-005 (debug gated): extend `SECTIONS_CONFIG` in
  `src/components/Sidebar.tsx:26-57` with a `debug` entry whose
  `enabled()` returns `settings.debug_mode === true`. The panel is a
  new wrapper `src/components/settings/debug/DebugSettings.tsx` that
  composes the four existing components in
  `src/components/settings/debug/`. Reuse the `Ctrl+Shift+D` toggle
  that already lives in `src/App.tsx:67-89`. The existing
  defensive-fallback helper (`src/components/Sidebar.tsx:59+`) handles
  the case where the active section becomes disabled.

## Component & module touch-list

- `src-tauri/src/settings/types.rs:259` - remove field.
- `src-tauri/src/settings/defaults.rs` - remove default if present.
- `src-tauri/src/commands/waveform/mod.rs:299-366` - remove helper,
  parameters, and dead branch.
- `src/bindings.ts` - regenerate.
- `src/i18n/locales/*/translation.json` (20 files) - purge
  `sidebar.general` (if any) and `overlay` namespace (if any).
- `src/components/Sidebar.tsx` - add `debug` to `SECTIONS_CONFIG`;
  verify `postProcessing` entry unchanged.
- `src/components/settings/post-processing/PostProcessingSettings.tsx`
  - add loopback label.
- `src/components/settings/debug/DebugSettings.tsx` (new) - compose
  the four existing components.
- `src/components/settings/index.ts` - export `DebugSettings`.

## Single-source-of-truth placement

- Loopback enforcement authority: `src-tauri/src/llm_client.rs`
  `is_local_host`. The new UI label in the post-processing panel is a
  consumer-level advisory; it cites the Rust enforcement point but
  does NOT duplicate the policy logic in TypeScript.
- Keep-segment selection: backend
  `src-tauri/src/commands/waveform/mod.rs` remains authority; the
  frontend consumes the already-normalized segments. R-001 removes a
  backend-local alt path, so there is still exactly one code path
  after this change.
- Sidebar routing: `src/components/Sidebar.tsx` `SECTIONS_CONFIG` is
  the single registry of reachable settings sections.

## Data flow

- Settings read: `settings.debug_mode` (already exists) -> Sidebar
  `enabled()` predicate -> React re-render toggles the Debug entry.
- Keep-segment flow after R-001: transcript -> waveform command ->
  `canonical_keep_segments_for_media` -> frontend (unchanged shape).

## Migration / compatibility

- R-001: older user settings.json may still contain
  `"experimental_simplify_mode": true`. Our serde should silently
  ignore unknown fields (confirm with a test load in AC-001-c).
- R-002 / R-004 / R-005 use only pre-existing i18n keys; no net-new
  keys added for any label that already exists. If the new loopback
  label needs a string, reuse a settings.* key where available;
  otherwise add exactly one new key
  (`settings.postProcessing.localOnlyNotice`) to all 20 locales per
  the `i18n-pruning` skill. Chunk D records the chosen string.
- `ui-experimental-and-cleanup` contained an alternate plan for
  `experimental_simplify_mode` (surface under an Experimental panel).
  This bundle supersedes that R-001; any in-flight work on that
  bundle must rebase on the deletion outcome.

## Hard constraints

- Local-only inference (AGENTS.md "Non-negotiable boundaries"): no
  hosted LLM, ASR, or captioning calls introduced. Post-processing
  panel keeps the loopback policy owned by
  `src-tauri/src/llm_client.rs`.
- Single source of truth (AGENTS.md "Repository layout"): sidebar
  registry lives in `src/components/Sidebar.tsx` only; locale files
  are the single source of UI strings; keep-segment selection stays
  on the backend.
- File-size cap 800 lines per source file (`bun run
  check:file-sizes`).
- AGENTS.md canonical-instructions: no rule duplication into other
  AI-instruction files.
- i18n-pruning skill invariants: every locale file moves together;
  `scripts/check-translations.ts` stays green.
- Non-negotiable handy-legacy-pruning gate applies to any surviving
  Handy-era module touched; this bundle touches none (apple stub is
  already gone; experimental_simplify_mode is Toaster-era dead code
  not Handy-era).

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Old settings.json with `experimental_simplify_mode` fails to load after flag removal | Confirm serde silently drops unknown fields; add AC-001-c live-app relaunch test | AC-001-c |
| i18n key deletion accidentally hits `settings.general.*` | Scope edits to the `sidebar.*` subtree only; call out in BLUEPRINT Architecture decisions | AC-002-a, AC-002-b |
| Post-processing panel loopback label drifts from `llm_client.rs` policy | Label text cites the function name; AC-004-c asserts `is_local_host` still exists | AC-004-c |
| Debug sidebar entry persists in production builds | `enabled()` returns `settings.debug_mode === true` which defaults to false | AC-005-a |
| Active section becomes unreachable when debug mode turns off | Rely on existing Sidebar defensive fallback at line 59+ | AC-005-c |
| `check_apple_intelligence_available` residue left in generated `src/bindings.ts` | Chunk C regenerates bindings if needed | AC-003-a |
