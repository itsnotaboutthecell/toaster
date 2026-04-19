# Blueprint: Keyboard shortcuts cheatsheet

## Architecture decisions

- **R-001 — Central registry + dispatcher hook.**
  Introduce three files under `src/lib/shortcuts/`:
  - `registry.ts` — exports the typed `shortcuts` array, the
    `ShortcutDefinition` / `ShortcutContext` types, and a
    helper `groupShortcuts(shortcuts)` that returns
    `{ editor: [...], playback: [...], navigation: [...] }`.
  - `dispatcher.ts` — exports `useShortcutDispatcher(ctx)`
    hook. Internally runs one `useEffect` that attaches a
    single `window.addEventListener('keydown', ...)` and
    iterates the registry, calling the first entry whose
    `match(e)` returns true (after the focus-in-input
    guard). Mirrors the pattern at
    `src/components/editor/EditorView.tsx:79-160` — same
    `useEffect` + same deps shape — but replaces the
    inline if/else chain with registry iteration.
  - `platform.ts` — exports `isMacLike()` using
    `navigator.platform` and a `formatChord(keys)` helper
    that picks mac or other at call time.
- **R-001 — Three existing handlers collapse into one
  consumer.**
  - `src/components/editor/EditorView.tsx` replaces its
    keydown `useEffect` (lines 79-160) with a call to
    `useShortcutDispatcher(ctx)` where `ctx` wraps the store
    accessors (`deleteWord`, `deleteRange`, `silenceWord`,
    `splitWord`, `undo`, `redo`, `selectWord`,
    `setSelectionRange`, `clearHighlights`, `setPlaying`,
    `seekTo`, `refreshFromBackend`) and the cheatsheet
    open/close setter.
  - `src/components/editor/TranscriptEditor.tsx:161-173`
    Ctrl/Cmd+F handler becomes a registry entry in the
    `editor` group.
  - `src/App.tsx:67-89` Ctrl/Cmd+Shift+D debug toggle
    becomes a registry entry in the `navigation` group.
- **R-001 / R-001-b — Cheatsheet modal component.**
  New `src/components/modals/ShortcutsCheatsheet.tsx`. It is
  a controlled component driven by a boolean in the editor
  store (new `cheatsheetOpen` flag). Renders with the same
  styling tokens used by `src/components/ui/Alert.tsx` and
  reuses the outside-click pattern already present in the
  Find overlay inside
  `src/components/editor/TranscriptEditor.tsx`. Registry
  entries with `action: (_e, ctx) => ctx.openCheatsheet()`
  bound to `?` and `Ctrl+/` provide the triggers; the modal's
  own Esc handler closes it before the editor's global Esc
  runs (dispatcher short-circuits when
  `cheatsheetOpen === true`).
- **R-002 — Platform-correct chords at render time.**
  `formatChord(def.keys)` is called inside the modal's
  render, never memoized at module scope. Keeps the check
  cheap (`navigator.platform` read is trivial) and immune to
  platform-spoofing surprises in tests.
- **R-002 — i18n placement.**
  New keys live under `shortcuts.groups.{editor,playback,
  navigation}` and `shortcuts.descriptions.<id>`. The
  feature adds keys only; it does not rename or remove any
  existing key. Non-English locales are seeded with the
  English string per repo policy; `i18n-pruning` and
  `scripts/check-translations.ts` are the enforcement.
- **R-003 — Removal-is-automatic is structural.**
  Because the modal and the dispatcher both iterate the same
  exported array, removing an entry is a one-line edit in
  `registry.ts`. No parallel list needs to be updated. AC-003-a
  is verified manually with a throwaway entry in the
  pre-merge walkthrough.

## Component & module touch-list

New:

- `src/lib/shortcuts/registry.ts`
- `src/lib/shortcuts/dispatcher.ts`
- `src/lib/shortcuts/platform.ts`
- `src/lib/shortcuts/index.ts` (barrel)
- `src/components/modals/ShortcutsCheatsheet.tsx`
- `src/components/modals/ShortcutsCheatsheet.module.css`
  (only if the existing Alert pattern uses CSS modules; else
  inline tailwind classes)

Modified:

- `src/components/editor/EditorView.tsx:79-160` — delete
  inline keydown effect; call `useShortcutDispatcher`.
- `src/components/editor/TranscriptEditor.tsx:161-173` —
  delete inline Ctrl/Cmd+F effect; register as
  `editor.toggle-find`.
- `src/App.tsx:67-89` — delete inline Ctrl/Cmd+Shift+D
  effect; register as `navigation.toggle-debug`.
- `src/App.tsx` (or the main layout) — mount
  `<ShortcutsCheatsheet />` once.
- `src/i18n/locales/*/translation.json` (x20) — add
  `shortcuts.*` subtree.
- Editor store — add `cheatsheetOpen` boolean plus
  `openCheatsheet` / `closeCheatsheet` setters, following
  the pattern of other modal flags already in the store.

Not touched:

- `src-tauri/**` — no backend change.
- Any audio, caption, export, or transcription module.
- Handy-era modules listed in `handy-legacy-pruning`.

## Single-source-of-truth placement

- **Authority:** `src/lib/shortcuts/registry.ts` — the
  `shortcuts` array.
- **Consumers:** `src/lib/shortcuts/dispatcher.ts` (runtime
  hit-test + action dispatch) and
  `src/components/modals/ShortcutsCheatsheet.tsx` (render).
- **Non-authorities:** the three previous keydown sites now
  own zero shortcut knowledge; each just supplies its store
  accessors via the dispatcher context.
- This is frontend-only; unlike audio / caption features,
  there is no backend authority to mirror.

## Data flow

1. App mounts. Editor store exposes `cheatsheetOpen = false`.
2. `useShortcutDispatcher(ctx)` runs inside the top-level
   editor view; attaches one `window.keydown` listener.
3. On each keydown: dispatcher checks focus-in-input guard,
   then iterates `shortcuts`, calling the first `def.match(e)`
   that returns true with `def.action(e, ctx)`.
4. The `?` / `Ctrl+/` entries call
   `ctx.openCheatsheet()` which sets `cheatsheetOpen = true`.
5. `<ShortcutsCheatsheet />` observes `cheatsheetOpen`,
   renders the grouped list via `groupShortcuts(shortcuts)`
   and `formatChord(def.keys)`, and owns its own Esc /
   outside-click handlers that set `cheatsheetOpen = false`.
6. The dispatcher's global-Esc registry entry is gated with
   `ignoreWhileCheatsheetOpen: true` so Esc-closes-modal
   wins over Esc-clears-selection.

## Migration / compatibility

- All existing chords keep their exact semantics; only the
  dispatch site changes. No user-visible change outside the
  new `?` / `Ctrl+/` behavior.
- No settings, no persisted state, no schema migration.
- Rollback is a straight revert of the new files plus the
  three modified call sites.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Refactor silently drops a shortcut | Enumerate all three existing sites into `registry.ts` entries in a single commit; reviewer checks grep of `addEventListener('keydown'` returns only the dispatcher. | AC-001-a, AC-001-c |
| Esc closes modal AND clears editor selection | Dispatcher short-circuits on `cheatsheetOpen`; modal's Esc handler runs first via capture-phase listener. | AC-001-b |
| `?` fires while typing in Find box | `ignoreInEditable: true` on the trigger entry; dispatcher checks active element. | AC-001-b |
| i18n drift (new keys missing from some locale) | `scripts/check-translations.ts` gate. | AC-002-b |
| Platform detection wrong at render | `isMacLike()` reads `navigator.platform` every call; never cached. | AC-002-a |
| Cheatsheet and handler diverge over time | Structural: both iterate the same exported array. | AC-001-a, AC-003-a |
