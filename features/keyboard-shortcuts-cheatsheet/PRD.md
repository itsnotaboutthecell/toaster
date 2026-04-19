# PRD: Keyboard shortcuts cheatsheet

## Problem & Goals

Editor shortcuts are discoverable only by reading source. Users
have to learn shortcuts from docs, AGENTS.md, or by watching a
teammate. Ship an in-app cheatsheet sourced from the same
registry the keydown handlers consume, so it cannot drift from
the real bindings. Closes part of SR8
(`features/product-map-v1/PRD.md:366-368`, Milestone 2 row 2.6
at line 476).

## Scope

### In scope

- Central shortcut registry at `src/lib/shortcuts/registry.ts`.
- Dispatcher hook/module that wires the registry to
  `window.keydown` (replacing the three ad-hoc listeners at
  `src/components/editor/EditorView.tsx:79-160`,
  `src/components/editor/TranscriptEditor.tsx:161-173`, and
  `src/App.tsx:67-89`).
- Cheatsheet modal component reading from the same registry.
- `?` and `Ctrl+/` triggers to open the modal; Esc / outside
  click to dismiss.
- Platform-aware chord rendering (Cmd on macOS, Ctrl on
  Windows/Linux) resolved at render time from
  `navigator.platform`.
- i18n keys for group headings and per-shortcut descriptions,
  present in all 20 `src/i18n/locales/*/translation.json`
  files, seeded with English where a translation is not yet
  available.

### Out of scope (explicit)

- User-editable / remappable shortcuts.
- Backend or Rust changes.
- New dialog library; mirror existing UI primitives.
- Mobile / touch affordances.
- Handy-era tray / push-to-talk surfaces.

## Requirements

### R-001 — Single-source registry drives handlers and cheatsheet

- Description: exactly one TypeScript module defines every
  keyboard shortcut. Both the global keydown dispatcher and
  the cheatsheet modal render from it. No duplicated shortcut
  tables anywhere in `src/`.
- Rationale: today three separate `addEventListener('keydown',
  ...)` blocks hold implicit knowledge of the bindings; any
  cheatsheet built alongside them will silently drift the
  moment someone adds or edits a shortcut.
- Acceptance Criteria
  - AC-001-a — A module at `src/lib/shortcuts/registry.ts`
    exports a single `shortcuts` array, and the keydown
    dispatcher + the cheatsheet modal both import from it;
    no other file in `src/` holds a list of shortcut chords
    or descriptions.
  - AC-001-b — Pressing `?` (Shift+/) or `Ctrl+/` while focus
    is outside a text input opens the cheatsheet modal;
    pressing Esc or clicking outside the modal dismisses it
    without clearing the editor's word selection.
  - AC-001-c — With a clip loaded and the editor active, the
    open cheatsheet lists every chord currently wired by the
    dispatcher, grouped under Editor, Playback, and
    Navigation headings.

### R-002 — Platform-correct chords and full i18n parity

- Description: chord labels render as Cmd on macOS and Ctrl on
  Windows/Linux; group headings and per-shortcut descriptions
  live under a `shortcuts.*` i18n subtree present in every
  locale.
- Rationale: Toaster ships on macOS, Windows, and Linux; the
  cheatsheet must match the keys the user actually presses.
  The repo already enforces i18n key parity via
  `scripts/check-translations.ts` and `i18n-pruning`.
- Acceptance Criteria
  - AC-002-a — On macOS the modal renders chords with the Cmd
    symbol; on Windows / Linux the same chords render with
    Ctrl; resolution uses `navigator.platform` at render time
    (no module-load cache).
  - AC-002-b — Every new key under `shortcuts.*` exists in
    all 20 `src/i18n/locales/*/translation.json` files, and
    `bun scripts/check-translations.ts` exits 0.

### R-003 — Registry removal flows through automatically

- Description: deleting a shortcut from the registry removes
  it from the cheatsheet and disables its keydown behavior
  with no further edits.
- Rationale: prevents the class of bug this feature exists to
  prevent — the cheatsheet and the reality diverging.
- Acceptance Criteria
  - AC-003-a — With a throwaway registry entry temporarily
    added and then removed, the cheatsheet's visible list
    and the live keydown behavior both update without any
    other file edit; this is exercised as a documented
    manual verification step.

## Edge cases & constraints

- Focus-in-input guard: dispatcher must ignore shortcuts whose
  `ignoreInEditable` flag is set when the focused element is
  an `input`, `textarea`, or `contenteditable` region (the
  Find box in `TranscriptEditor.tsx` must keep working).
- Esc ownership: when the modal is open, Esc closes the modal
  only; the existing Esc-clears-selection behavior at
  `EditorView.tsx:145-148` must not fire in the same keystroke.
- `?` vs `/`: bind both `event.key === '?'` and
  `event.key === '/' && (ctrlKey || metaKey)` to cover
  keyboard layouts without a dedicated `?` key.
- Ctrl+/ is currently unbound (grep of `src/**` shows no
  existing handler); adding it as an alias is safe.
- 800-line cap per file; keep `registry.ts`, the dispatcher,
  and the modal component in separate files.
- ASCII only in planning artifacts.
- No hosted-inference dependency (AGENTS.md non-negotiable).

## Data model

See REQUEST.md §6 for the `ShortcutDefinition` and
`ShortcutContext` shapes. BLUEPRINT.md locks the exact
TypeScript.

## Non-functional requirements

- No measurable typing latency regression in the transcript
  editor (dispatcher must short-circuit on non-matching
  events).
- Modal open / close must feel instant on the reference
  fixture (`eval/fixtures/toaster_example.mp4`).
- Bundle size delta < 5 KB gzipped for the new modules
  combined.
