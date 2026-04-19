# PRD: UI Experimental Pattern + Cleanup

## Problem & Goals

Toaster's settings surface today has three orthogonal hygiene
problems:

1. `experimental_simplify_mode`
   (`src-tauri/src/settings/types.rs:250`) is a working backend
   flag with no UI. It gates a real keep-segment behavior but is
   invisible to users.
2. The post-processing settings tree
   (`src/components/settings/post-processing/`,
   `src/components/settings/post-processing-api/`) and its
   loopback-enforced backend (`src-tauri/src/managers/cleanup/`)
   are functional but unmounted; the `sidebar.postProcessing`
   translation key already exists as an orphan.
3. The Apple Intelligence stub
   (`src-tauri/src/commands/mod.rs:126`,
   `src-tauri/src/settings/mod.rs:13`,
   `src/bindings.ts:342`,
   `src/components/settings/post-processing-api/
   usePostProcessProviderState.ts:32`) is a no-op on the desktop
   target; Toaster does not ship to mobile or Apple-OS-only
   surfaces.

This bundle (i) introduces an Experimental Features pattern that
hosts `experimental_simplify_mode` as its first occupant, (ii)
re-mounts the post-processing UI as a standalone panel, and (iii)
deletes the Apple Intelligence stub and any i18n keys that remain
orphaned after restoration.

## Scope

### In scope

- 5a Experimental panel + `experiments.ts` registry.
- 5b Post-processing UI re-mount.
- 5c Apple stub deletion + post-restoration orphan-i18n purge.

### Out of scope (explicit)

- Removing `experimental_simplify_mode` setting.
- Editing audio / caption / export / time-mapping / keep-segments
  code (other bundles own).
- Editing `tauri.conf.json` (Bundle 4 owns).
- Deleting transcript edit history.
- Removing other Handy-era modules (covered by
  `remove-history-and-legacy` bundle separately).

## Requirements

### R-001 — Experimental Features pattern (5a)

- Description: a settings panel "Experimental" with a top banner
  ("These features are under active development and may change or
  be removed."). Lists experiments from a TS registry. First
  occupant: `experimental_simplify_mode`.
- Acceptance Criteria
  - AC-001-a — In the live app, opening Settings shows an
    "Experimental" sidebar entry; clicking it opens a panel whose
    first element is the warning banner with the exact copy above
    (or its localized i18n equivalent).
  - AC-001-b — In the live app, the Experimental panel lists
    `experimental_simplify_mode` with a label, a one-line
    description, and a link to a feedback issue template URL.
  - AC-001-c — Toggling the simplify_mode switch persists across
    app restart (verified by closing the app, relaunching, and
    re-opening the panel).
  - AC-001-d — `src/lib/experiments.ts` exists and exports a
    `readonly` array typed `Experiment[]`. Adding a hypothetical
    second experiment requires editing only `experiments.ts` and
    the i18n files; no other source file changes.

### R-002 — Restore post-processing UI (5b)

- Description: re-mount
  `src/components/settings/post-processing/`,
  `src/components/settings/post-processing-api/` under a new
  standalone Settings panel referenced via the existing
  `sidebar.postProcessing` translation key.
- Acceptance Criteria
  - AC-002-a — In the live app, opening Settings shows a "Post
    Process" sidebar entry; clicking it opens the
    PostProcessingSettings panel.
  - AC-002-b — In the live app, configuring a local OpenAI-
    compatible loopback provider (e.g. `http://localhost:1234/v1`),
    saving, and running cleanup on a transcript fixture produces
    cleaned output (loopback enforcement: `managers/cleanup/`
    backend rejects non-loopback URLs).
  - AC-002-c — `BLUEPRINT.md` "Architecture decisions" justifies
    the standalone-panel placement.

### R-003 — Apple stub + apple_intelligence symbol purge (5c)

- Description: delete `check_apple_intelligence_available` command,
  its registration in `lib.rs`, the `bindings.ts` wrapper, the
  `APPLE_INTELLIGENCE_PROVIDER_ID` and `APPLE_PROVIDER_ID`
  constants, the `appleIntelligence` i18n keys, and any
  apple-only branches in `usePostProcessProviderState.ts`.
- Acceptance Criteria
  - AC-003-a — `cargo test` and `cargo check` pass after the
    backend deletions (verifies registration removal is consistent
    with the command deletion).
  - AC-003-b — `rg -n
    "check_apple_intelligence_available|APPLE_INTELLIGENCE_PROVIDER_ID|APPLE_PROVIDER_ID|apple_intelligence|appleIntelligence"
    src src-tauri` returns zero matches outside vendored / build
    artifacts.
  - AC-003-c — In the live app, the post-processing provider
    Select no longer offers an Apple Intelligence option; selecting
    a previously-stored apple provider falls back to the default
    provider on next load (no crash).

### R-004 — Orphan-i18n purge (5c, post-restoration)

- Description: identify which keys from
  `product-map-v1` PRD §3 (sidebar.general, sidebar.debug,
  sidebar.postProcessing, overlay namespace, appleIntelligence
  block, etc.) are *still* orphaned after 5a (which adds
  sidebar.experimental + experiments.* keys) and 5b (which
  re-uses sidebar.postProcessing and the post-processing key
  subtree). Remove only the still-orphaned keys, across all 20
  locales.
- Acceptance Criteria
  - AC-004-a — `bun run scripts/check-translations.ts` exits 0
    after the bundle.
  - AC-004-b — `i18n-pruning` skill returns pass; no key is
    present in some locales but absent in others.
  - AC-004-c — `BLUEPRINT.md` "Migration / compatibility"
    enumerates the exact key list removed (filled in during
    execution after restoration).

### R-005 — handy-legacy-pruning gate on touched stub files

- Description: any source file touched in 5c that matches the
  Handy-era list (per `handy-legacy-pruning` skill) gets the
  skill's "is this still on the transcript-editor path?" question
  answered explicitly in the journal before deletion.
- Acceptance Criteria
  - AC-005-a — `journal.md` contains an "Apple stub deletion
    rationale" entry confirming `handy-legacy-pruning` was applied
    and no transcript-editor path is broken.

## Edge cases & constraints

- A user who previously selected the apple provider must land on
  the default provider, not crash.
- Removing the `appleIntelligence` i18n keys must succeed even if
  the post-processing UI references them; the references must be
  removed first.
- `sidebar.postProcessing` is re-used (not added); confirm value
  text still matches the current product wording before re-mount.
- ASCII-only changes; 800-line cap; no hosted inference.

## Data model (if applicable)

- `Experiment = { id: string, settingsKey: keyof Settings,
  labelKey: string, descriptionKey: string, feedbackUrl: string }`
  in `src/lib/experiments.ts`.

## Non-functional requirements

- AGENTS.md "Settings UI contract": Experimental panel + restored
  post-processing panel each follow the label + one-line
  description rule.
- AGENTS.md "Verified means the live app, not `cargo check`": every
  R-NNN has at least one live-app AC.
- AGENTS.md "Local-only inference": post-processing already
  enforces loopback URLs in `managers/cleanup/`; this bundle does
  not weaken that.
