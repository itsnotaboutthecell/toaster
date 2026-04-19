# Feature request: keyboard shortcuts cheatsheet

## 1. Problem & Goals

Toaster's editor exposes a growing set of keyboard shortcuts for
selection, deletion, silencing, splitting, undo/redo, seek,
play/pause, find, and debug-toggle. Today these are discoverable
only by reading the source or AGENTS.md (see
`src/components/editor/EditorView.tsx:79-155` and
`src/components/editor/TranscriptEditor.tsx:161-173`,
`src/App.tsx:67-89`). New users miss fast paths; returning users
forget chords they only use occasionally.

Ship a `?` overlay that lists every wired shortcut, grouped and
platform-correct, sourced from the same registry the keydown
handlers consume so the cheatsheet cannot drift from reality.

Closes part of SR8 in `features/product-map-v1/PRD.md:366-368`
under Milestone 2 (table row 2.6, PRD.md:476).

## 2. Desired Outcome & Acceptance Criteria

See PRD.md for the normative ACs. Summary:

- Pressing `?` (Shift+/) or `Ctrl+/` opens a modal listing every
  currently-wired shortcut grouped into Editor / Playback /
  Navigation.
- Esc or outside-click dismiss.
- Shortcut chords render with Cmd on macOS and Ctrl on
  Windows/Linux, resolved from `navigator.platform` at render
  time.
- Group headings and human descriptions are i18n-keyed across
  all 20 locales; chord strings are not translated.
- There is exactly one registry of shortcut definitions; the
  keydown handler and the cheatsheet both read from it. Removing
  a registry entry removes it from both the handler and the
  cheatsheet with no other edits.

## 3. Scope Boundaries

### In scope

- New shortcut registry module (TypeScript) under
  `src/lib/shortcuts/` with a typed definition shape.
- Cheatsheet modal component reading from the registry.
- Refactor the three existing keydown sites to consume the
  registry:
  - `src/components/editor/EditorView.tsx:79-160`
  - `src/components/editor/TranscriptEditor.tsx:161-173`
  - `src/App.tsx:67-89` (Ctrl+Shift+D debug toggle)
- i18n keys for group headings + per-shortcut descriptions,
  added to all 20 locales under `src/i18n/locales/*/translation.json`
  (non-English seeded with the English string per repo policy).
- A modal component following the patterns already used by
  `src/components/ui/Alert.tsx` / the Find overlay in
  `TranscriptEditor.tsx` (no new dialog library).

### Out of scope (explicit)

- Remapping / user-editable shortcut bindings.
- Conflict detection across bindings.
- Touch / mobile affordances.
- Any backend change — this is entirely a frontend feature.
- Changes to the dictation-era surfaces flagged by
  `handy-legacy-pruning`.

## 4. References to Existing Code

- `src/components/editor/EditorView.tsx:79-160` — canonical
  pattern: `useEffect` + `window.addEventListener('keydown', ...)`.
  Refactor target; must continue to call the same store actions
  (`deleteWord`, `deleteRange`, `silenceWord`, `splitWord`,
  `undo`, `redo`, `selectWord`, `setSelectionRange`,
  `clearHighlights`, `setPlaying`, `seekTo`).
- `src/components/editor/TranscriptEditor.tsx:161-173` — Ctrl/Cmd+F
  find overlay. Must keep working after refactor.
- `src/App.tsx:67-89` — Ctrl/Cmd+Shift+D debug toggle. Must keep
  working after refactor.
- `src/components/ui/Alert.tsx`, `src/components/ui/Tooltip.tsx` —
  UI primitives to mirror for styling + a11y.
- `src/i18n/locales/en/translation.json` — existing i18n key tree;
  add `shortcuts.*` subtree. 20 locales total (see
  `Get-ChildItem src/i18n/locales` = 20).
- `scripts/check-translations.ts` — key-parity gate across all
  locales.

## 5. Edge Cases & Constraints

- `?` is Shift+/ on US layouts; other layouts may differ. Bind
  both `event.key === '?'` and `event.key === '/' && ctrl/meta`
  to cover layouts without a dedicated `?`.
- When the cheatsheet itself is open, Esc must close it without
  also clearing editor selection (today Esc clears selection in
  `EditorView.tsx:145-148`). The registry dispatcher must know
  when a modal owns Esc.
- When focus is inside a text input (e.g., the Find box), the
  `?` trigger must not fire.
- Ctrl+/ is not currently bound; verified via grep of
  `src/**/*.ts?`.
- Platform detection via `navigator.platform` at render time; do
  not cache at module load (Toaster runs under Tauri webview but
  platform detection must stay in JS to avoid an IPC round-trip
  per render).
- 800-line cap per file (AGENTS.md). Keep the registry, the
  modal, and the dispatcher in separate files.
- i18n parity gate (`scripts/check-translations.ts`) must stay
  green.
- No hosted-inference dependency (AGENTS.md non-negotiable).

## 6. Data Model

Registry entry shape (TypeScript):

```
type ShortcutGroup = 'editor' | 'playback' | 'navigation';

interface ShortcutDefinition {
  id: string;                       // stable, kebab-case
  group: ShortcutGroup;
  keys: { mac: string; other: string }; // chord label, literal
  match: (e: KeyboardEvent) => boolean;  // hit test
  action: (e: KeyboardEvent, ctx: ShortcutContext) => void;
  descriptionKey: string;           // i18n key under shortcuts.*
  // Optional: when true, dispatcher skips this entry while a
  // text input owns focus.
  ignoreInEditable?: boolean;
}
```

Context shape (`ShortcutContext`) is whatever the handlers need
(store accessors, modal open/close setters). Authored in the
blueprint.

## Q&A

Inputs to Phase 5 were pre-answered in the originating REQUEST
from the controller. No user-facing Q&A round was run.

- Trigger key: `?` (Shift+/) globally; `Ctrl+/` alias for
  layouts without a `?` key.
- Dismiss: Esc or click outside the modal.
- Scope of shortcuts shown: every editor + sidebar shortcut
  currently implemented; platform variants (Cmd vs Ctrl)
  resolve per `navigator.platform` at render time.
- Grouping: three groups — Editor / Playback / Navigation —
  derived from the registry's `group` field.
- i18n: group headings and each description are i18n-keyed;
  key-chord strings are literal and not translated.
