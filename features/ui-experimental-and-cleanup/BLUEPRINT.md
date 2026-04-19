# Blueprint: UI Experimental Pattern + Cleanup

## Architecture decisions

- **R-001 (Experimental pattern)**: new
  `src/lib/experiments.ts` exporting
  `export const experiments: readonly Experiment[]`. New panel
  `src/components/settings/experimental/ExperimentalSettings.tsx`
  iterates the registry and renders one row per experiment plus a
  banner at the top. Reuses the `useSettings` hook and the existing
  `Toggle` control. Sidebar entry uses a new
  `sidebar.experimental` i18n key.
- **R-002 (post-processing re-mount)**: new wrapper component
  `src/components/settings/post-processing/PostProcessingSettings.tsx`
  composes the existing `PostProcessingSettingsPrompts.tsx` and
  the `post-processing-api/*` provider components. Placement is
  **standalone panel under `sidebar.postProcessing`**, not nested
  under Editor. Justification: post-processing is a coherent
  sub-product (provider + prompts + API key) deserving its own
  panel; nesting under Editor would push the Editor panel beyond
  the AGENTS.md "Settings UI contract" complexity bar.
- **R-003 (Apple stub purge)**: deletions are mechanical and
  scoped — no new code:
  - `src-tauri/src/commands/mod.rs:126` — remove function.
  - `src-tauri/src/lib.rs:231` — remove registration entry.
  - `src-tauri/src/settings/mod.rs:13` — remove constant.
  - `src/bindings.ts:342` — auto-removed by codegen on next build
    (or hand-removed if codegen output is committed).
  - `src/components/settings/post-processing-api/
    usePostProcessProviderState.ts:32` and any apple branches in
    that file — remove.
  - `src/i18n/locales/*/translation.json` — remove
    `appleIntelligence` keys + provider option labels.
- **R-004 (orphan-i18n purge)**: candidates from
  `product-map-v1` PRD §3 are: `sidebar.general` (orphan),
  `sidebar.debug` (orphan after restore?), `overlay` namespace
  (orphan), `appleIntelligence` (deleted in 5c), various
  post-processing description keys. Re-mounting in 5b restores
  `sidebar.postProcessing` + the post-processing.* subtree.
  Final remove list is computed after 5a + 5b land and is recorded
  in this BLUEPRINT's Migration section before the orphan-purge
  task closes.
- **R-005 (handy-legacy-pruning)**: the apple stub is on the
  Handy-era list per the skill (`apple_intelligence.rs`). This
  bundle invokes `handy-legacy-pruning` for the deletion and
  records the answer in `journal.md`.

## Component & module touch-list

| File | Change |
|------|--------|
| `src/lib/experiments.ts` (new) | `Experiment` type + `experiments` registry. First occupant: `experimental_simplify_mode`. |
| `src/components/settings/experimental/ExperimentalSettings.tsx` (new) | Panel: banner + iterate registry + Toggle per experiment. |
| `src/components/settings/post-processing/PostProcessingSettings.tsx` (new) | Wrapper composing the existing prompt + provider components. |
| Sidebar registration (existing settings index, e.g. `src/components/settings/index.ts`) | Add Experimental and Post Process entries. |
| `src-tauri/src/commands/mod.rs:126` | Delete `check_apple_intelligence_available`. |
| `src-tauri/src/lib.rs:231` | Remove registration. |
| `src-tauri/src/settings/mod.rs:13` | Remove `APPLE_INTELLIGENCE_PROVIDER_ID`. |
| `src/bindings.ts:342` | Removed (auto-regenerated). |
| `src/components/settings/post-processing-api/usePostProcessProviderState.ts:32` | Remove `APPLE_PROVIDER_ID` and any apple branches. |
| `src/i18n/locales/*/translation.json` (20 files) | Add `sidebar.experimental`, `experiments.simplifyMode.label/description`, `experiments.feedbackLink`, `settings.experimental.banner`; remove `appleIntelligence` keys; remove still-orphaned keys from §3. |

## Single-source-of-truth placement

- **Experiments registry**: `src/lib/experiments.ts` is the
  single source. `ExperimentalSettings.tsx` consumes it; no other
  consumer.
- **Post-processing loopback authority**: unchanged; remains in
  `src-tauri/src/managers/cleanup/`. Frontend remounted but does
  not duplicate the loopback URL check.
- **Settings**: `experimental_simplify_mode` already lives in
  `src-tauri/src/settings/types.rs:250`; the UI is a passive
  consumer via `useSettings`.

## Data flow

```
ExperimentalSettings.tsx
  -> import experiments from src/lib/experiments.ts
  -> for each experiment: read settings[experiment.settingsKey]
  -> Toggle onChange -> useSettings setter -> backend
  -> banner i18n: settings.experimental.banner
  -> feedback link: experiment.feedbackUrl

PostProcessingSettings.tsx (re-mounted)
  -> compose existing PostProcessingSettingsPrompts + provider UI
  -> provider invokes managers/cleanup loopback-only backend
```

## Migration / compatibility

- Settings: no backend schema change. Apple provider in stored
  settings falls back to the default provider on load.
- i18n keys removed (final list filled during execution after
  5a/5b land):
  - `sidebar.general` (orphan in 22 locales).
  - `overlay` namespace (orphan).
  - `postProcessingPanel.appleIntelligence.*`,
    `placeholderApple`, `descriptionApple` (apple stub purge).
  - Any other §3 candidate that did not become reachable via 5a /
    5b. The execution task `uec-final-orphan-list` writes the
    confirmed list back into this section before the orphan-purge
    task closes.
- `bindings.ts` regenerated on next `cargo build`; CI must
  regenerate or commit the regeneration in the same PR.

## Sequencing & conflict-avoidance

- **Position**: bundle 4 of 5 in execution order (parallel-able
  with the export track per orchestrator decision; explicit
  no-touch fences ensure disjoint file scope).
- **Files this bundle owns**: everything in
  `src/components/settings/experimental/`,
  `src/components/settings/post-processing/`,
  `src/components/settings/post-processing-api/`, the apple-stub
  deletions named above, and the i18n locale files for the keys
  enumerated.
- **Files this bundle agrees not to touch**:
  - Any file under `src-tauri/src/managers/export/` (Bundle 3 may
    create this).
  - Any file under `src/components/settings/export/` (Bundle 1
    creates; Bundles 2 + 3 extend).
  - `src-tauri/src/commands/waveform/` (export pipeline).
  - `src-tauri/src/managers/splice/loudness.rs` (Bundle 1).
  - `src-tauri/tauri.conf.json` (Bundle 4 — release-code-signing
    — owns).
  - `.github/workflows/release.yml` (Bundle 4 owns).
  - Any file under `src-tauri/src/managers/captions/`,
    `managers/editor/`, `managers/transcription/`, `managers/
    cleanup/` (backend authority preserved; cleanup backend stays
    untouched).
- **Parallel safety**: this bundle and the export track touch
  disjoint trees, confirming the orchestrator's "5 in parallel
  with 1-3" plan.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Removing `experimental_simplify_mode` by accident | PRD R-001 keeps it; AC-001-c verifies persistence | AC-001-c |
| Deleting an i18n key still referenced by 5a/5b | Final remove list computed after 5a/5b land; `i18n-pruning` skill catches asymmetric removals | AC-004-a, AC-004-b |
| Stored settings with apple provider crash on load | Fallback-to-default branch in provider load; AC-003-c live test | AC-003-c |
| `bindings.ts` not regenerated in PR; runtime invocation 404 | CI regenerates as part of build; reviewer enforces | AC-003-a |
| Touching files outside the no-touch fence | Sequencing section + reviewer | (process) |
| Handy-era expansion via apple_intelligence remnant | `handy-legacy-pruning` applied; rationale captured in journal | AC-005-a |
| Banner missing or wrong copy → users miss the warning | AC-001-a inspects banner copy in live app | AC-001-a |
| Restored post-processing UI accidentally enables non-loopback providers | Backend `managers/cleanup/` loopback enforcement is untouched; AC-002-b confirms cleanup runs against loopback only | AC-002-b |
| `sidebar.postProcessing` value text drifts from product wording | Reviewer step before re-mount; not gated by an AC (low risk) | n/a |
