# Feature request: UI Experimental Pattern + Cleanup

## 1. Problem & Goals

Closes PRD `product-map-v1` Blocker B3 plus orchestrator decisions
1, 2, and 4 from the planning session:

- (1) **Keep `experimental_simplify_mode`** but surface it via a
  first-class **Experimental Features** settings section with a
  warning banner. This becomes a reusable registry pattern so the
  next experiment is a one-file add.
- (2) **Restore the post-processing UI**. The unmounted tree under
  `src/components/settings/post-processing/` plus
  `src-tauri/src/managers/cleanup/` backend is fully functional and
  loopback-enforced; placement to be justified.
- (4) **Delete the `check_apple_intelligence_available` stub** and
  any `apple_intelligence` plumbing. Toaster is desktop-only;
  Apple Intelligence is mobile/Apple-OS-bound and not in scope.

A constrained orphan-i18n purge runs alongside, but only deletes
keys that remain orphaned after 5a + 5b restoration.

## 2. Desired Outcome & Acceptance Criteria

- Settings has an "Experimental" section with a banner ("These
  features are under active development and may change or be
  removed.") and one occupant: `experimental_simplify_mode`
  (label, description, link to a feedback issue template).
- A small TS registry (`src/lib/experiments.ts`) is the single
  source for experiment metadata; the next experiment is a one-file
  add.
- The post-processing settings UI is reachable from the sidebar and
  functional end-to-end (loopback-only enforced by the existing
  `managers/cleanup/` backend).
- `check_apple_intelligence_available` command and any
  `apple_intelligence` symbols are deleted; backend recompiles
  cleanly.
- The orphan i18n keys identified in `product-map-v1` PRD §3 that
  are *still orphaned after 5a + 5b* are removed across all 20
  locales.
- `bun run scripts/check-translations.ts` passes.
- `cargo check` / `cargo test` pass after the IPC handler removal.

## 3. Scope Boundaries

### In scope

- New `src/lib/experiments.ts` registry.
- New `src/components/settings/experimental/ExperimentalSettings.tsx`
  panel + sidebar entry under `sidebar.experimental`.
- Re-mount of the post-processing settings tree
  (`src/components/settings/post-processing/`,
  `src/components/settings/post-processing-api/`) under
  `sidebar.postProcessing`. Placement: standalone panel (not nested
  under Editor) — justified in BLUEPRINT.
- Deletion of `commands::check_apple_intelligence_available`
  (`src-tauri/src/commands/mod.rs:126`), its registration
  (`src-tauri/src/lib.rs:231`), the `bindings.ts` invocation
  (`src/bindings.ts:342`), the `APPLE_INTELLIGENCE_PROVIDER_ID`
  constant (`src-tauri/src/settings/mod.rs:13`), and the
  `APPLE_PROVIDER_ID` constant
  (`src/components/settings/post-processing-api/
  usePostProcessProviderState.ts:32`) plus any apple-intelligence
  i18n keys.
- Orphan-i18n purge limited to keys *still* orphaned after 5a + 5b.
- `handy-legacy-pruning` skill applied to any 5c file touched.
- `i18n-pruning` skill applied across all 20 locales.

### Out of scope (explicit)

- Removing `experimental_simplify_mode` itself (orchestrator
  decision: keep it).
- Touching the audio path, time-mapping, keep-segments, captions, or
  export pipelines (Bundles 1-3 own those).
- Touching `tauri.conf.json` (Bundle 4 owns).
- Deleting transcript edit history (orchestrator decision: keep).
- Removing real Handy-era modules beyond the apple_intelligence
  stub (covered by `remove-history-and-legacy` bundle separately).

## 4. References to Existing Code

- `src-tauri/src/settings/types.rs:250` — `experimental_simplify_mode`
  setting (kept).
- `src-tauri/src/commands/mod.rs:126` —
  `check_apple_intelligence_available()` stub (deleted).
- `src-tauri/src/lib.rs:231` — registration of the above (deleted).
- `src/bindings.ts:342` — invoke wrapper (regenerated/removed).
- `src-tauri/src/settings/mod.rs:13` —
  `APPLE_INTELLIGENCE_PROVIDER_ID` constant (deleted).
- `src/components/settings/post-processing-api/
  usePostProcessProviderState.ts:32` — `APPLE_PROVIDER_ID` constant
  (deleted; provider list pruned).
- `src/components/settings/post-processing/
  PostProcessingSettingsPrompts.tsx` — to remount.
- `src/components/settings/post-processing-api/*.tsx` — provider
  selection UI; remount with apple option removed.
- `src-tauri/src/managers/cleanup/` — backend; **not modified**.
- `src/i18n/locales/*/translation.json` — 20 files; orphan purge.
- `scripts/check-translations.ts` — gate.
- `features/product-map-v1/PRD.md:218,253-294,449` — provenance.

## 5. Edge Cases & Constraints

- The `sidebar.postProcessing` translation key already exists as an
  orphan (`product-map-v1` §3 item 6); this bundle re-uses it after
  verifying the value text still applies.
- Removing `apple_intelligence` from the provider list must not
  break a user who previously selected it; on settings load, an
  unknown provider falls back to the default provider.
- The Experimental panel must visually warn before listing
  experiments; missing the banner is a regression.
- ASCII-only changes; 800-line cap.
- No hosted inference.
- `handy-legacy-pruning` skill required for 5c files (the apple
  stub).

## 6. Data Model (optional)

- `experiments: readonly Experiment[]` in `src/lib/experiments.ts`:
  `Experiment = { id: string, settingsKey: keyof Settings,
  labelKey: string, descriptionKey: string, feedbackUrl: string }`.
- No backend schema change.

## Q&A

Pre-answered:

- Q: Delete or keep `experimental_simplify_mode`?
  - A: Keep, surface in an Experimental panel with a warning banner.
- Q: Restore post-processing UI under Editor or as its own panel?
  - A: Standalone panel (not under Editor). The cleanup feature is
    a coherent sub-product (provider, prompts, API key) deserving
    its own panel; nesting under Editor would push the Editor panel
    beyond the AGENTS.md "Settings UI contract" complexity bar.
- Q: Apple Intelligence?
  - A: Delete. Desktop-only; not in scope.
- Q: Which orphan i18n keys to remove?
  - A: Only those *still* orphaned after 5a + 5b. PRD enumerates
    candidates; final list determined post-restoration.
