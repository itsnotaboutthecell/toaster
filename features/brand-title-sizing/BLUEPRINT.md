# Blueprint: brand title sizing

## Architecture decisions

- **R-001 (wordmark size):** change one Tailwind class on one element.
  At `src/components/Sidebar.tsx:76`:

      <img src={toasterLogo} alt="Toaster" className="w-[120px] m-4" />

  becomes

      <img src={toasterLogo} alt="Toaster" className="w-[144px] mx-0 my-4" />

  Width 144 px = sidebar column 160 px (`w-40`,
  `src/components/Sidebar.tsx:75`) minus `px-2` on the sidebar
  container (8 px * 2). `mx-0` removes the previous 16 px horizontal
  margin so the image can use the full inner column. Asset import is
  unchanged (`src/components/Sidebar.tsx:4`).

- **R-002 (reclaim gutter):** two coordinated token changes.
  At `src/App.tsx:205`:

      <div className="flex flex-col items-center p-4 gap-4">

  becomes

      <div className="flex flex-col items-center py-4 gap-4">

  (drop horizontal padding; vertical and gap unchanged.) Then raise
  inner caps:

  | File:Line | Old | New |
  |-----------|-----|-----|
  | `src/components/editor/EditorView.tsx:400`              | `max-w-4xl` | `max-w-6xl` |
  | `src/components/settings/about/AboutSettings.tsx:31`    | `max-w-3xl` | `max-w-5xl` |
  | `src/components/settings/advanced/AdvancedSettings.tsx:20` | `max-w-3xl` | `max-w-5xl` |
  | `src/components/settings/history/HistorySettings.tsx:242`  | `max-w-3xl` | `max-w-5xl` |
  | `src/components/settings/models/ModelsSettings.tsx:200`    | `max-w-3xl` | `max-w-5xl` |
  | `src/components/settings/models/ModelsSettings.tsx:209`    | `max-w-3xl` | `max-w-5xl` |

  Pattern source: existing `max-w-3xl w-full mx-auto` and
  `max-w-4xl w-full mx-auto` idioms in those same files. We keep
  `w-full mx-auto` so smaller viewports collapse naturally.

- **R-003 (responsive):** Tailwind's `max-w-*` is an upper bound and
  `w-full` is the default; no new breakpoints needed. The browser
  handles narrow viewports without media queries.

- **R-004 (no editor/settings regression):** every other class on
  every touched line is preserved verbatim. Only the `max-w-*` token
  is swapped on cap rows; only `p-4` -> `py-4` on the App row; only
  `w-[120px] m-4` -> `w-[144px] mx-0 my-4` on the wordmark row.

- **R-005 (gates):** changes are class-string-only; no new files, no
  new imports, no new color literals, no new i18n keys. File-size cap
  is unaffected.

## Component & module touch-list

- `src/components/Sidebar.tsx` -- one class change (line 76).
- `src/App.tsx` -- one class change (line 205).
- `src/components/editor/EditorView.tsx` -- one class change
  (line 400).
- `src/components/settings/about/AboutSettings.tsx` -- one class
  change (line 31).
- `src/components/settings/advanced/AdvancedSettings.tsx` -- one
  class change (line 20).
- `src/components/settings/history/HistorySettings.tsx` -- one
  class change (line 242).
- `src/components/settings/models/ModelsSettings.tsx` -- two class
  changes (lines 200 and 209; same `max-w-3xl` token in two render
  branches).

Total surface: 8 single-line class-string edits across 7 files.

## Single-source-of-truth placement

Not applicable. This change does not touch any preview / export /
caption / time-mapping path; AGENTS.md "Non-negotiable boundaries"
and the dual-path SSoT rule are not engaged. The brand asset already
has a single source (`toaster_text.svg` at repo root, imported in
`src/components/Sidebar.tsx:4`); we keep that arrangement.

The settings-pane `max-w-*` value is _not_ centralized today (each
pane spells out its own cap). This blueprint matches the existing
convention rather than introducing a shared constant -- centralizing
would itself be a restructure (out of scope per REQUEST.md). If the
team later wants a shared `<SettingsPaneRoot>` wrapper, that is a
separate feature.

## Data flow

n/a (presentational).

## Migration / compatibility

- No persisted state, no user setting, no schema, no backend command
  involved. The change ships in one PR with no migration step.
- Screenshots in `docs/` and any marketing material that show the old
  small wordmark are out of scope; they will look slightly stale
  until refreshed independently.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Wordmark clips against sidebar right edge after the bump. | Width pinned to 144 px = exact inner column width; `mx-0` removes the old 16 px horizontal margin. | AC-001-b |
| Editor toolbar / sections visually break when content widens to 1152 px. | Inner blocks already use `w-full` and `space-y-*`; widening only the cap does not change child layout. Live-app smoke confirms. | AC-004-a |
| Settings pane controls become too wide and look "stretched". | Settings rows already use `SettingContainer` / `SettingsGroup` which size their own inner content; the cap bump only affects the outer column. Live-app smoke confirms. | AC-004-b |
| Removing horizontal `p-4` exposes content to viewport edge on a narrow window. | Inner views still have their own `mx-auto` plus `max-w-*` cap; on narrow viewports `items-center` keeps content inset by sidebar width. | AC-003-a |
| A touched file crosses the 800-line cap. | All seven files are well below cap today (largest is `EditorView.tsx`, ~570 lines); diff adds zero lines. | AC-005-a |
| Diff sneaks in a new color literal. | Reviewer plus AC-005-b grep gate; per BLUEPRINT, only width / margin / padding tokens are permitted. | AC-005-b |
