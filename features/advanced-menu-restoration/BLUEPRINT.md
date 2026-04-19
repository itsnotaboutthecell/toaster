# Blueprint: Advanced menu restoration

## Architecture decisions

- **R-001** (remove inline block): the editor view is stripped of the relocated `<DiscardWords>` / `<AllowWords>` block plus the `SettingsGroup` shell at `EditorView.tsx:574-578` and the two unused imports at lines 26-27. No other editor logic touched.
- **R-002** (Advanced surface): create `src/components/settings/advanced/AdvancedSettings.tsx` modelled on `experimental/ExperimentalSettings.tsx`. It is a thin shell: one `<SettingsGroup>` titled `settings.advanced.title`, two child rows for `<DiscardWords grouped>` and `<AllowWords grouped>`. Wire it into the Settings router / navigation the same way `ExperimentalSettings` is wired (concrete site discovered during execution).
- **R-003** (i18n): use `i18n-pruning` skill. New keys: `settings.advanced.title`, `settings.advanced.description`, plus any navigation label the existing settings router needs. Default English strings: "Advanced" / "Configured-once controls (allow / discard word lists, etc.)".
- **R-004** (audit + heuristic): heuristic stays in **one** authoritative place - propose `docs/settings-placement.md` (new file) referenced from the journal. The audit table itself lives in `journal.md` (operational artifact, may evolve as more settings are added).

## Component & module touch-list

- Edit: `src/components/editor/EditorView.tsx` (remove imports + block, ~5 line delete).
- Add: `src/components/settings/advanced/AdvancedSettings.tsx` (~50 lines).
- Edit: wherever the Settings nav is defined (likely `src/components/settings/index.ts` or a router file - confirm during execution).
- Edit: `src/i18n/locales/*/translation.json` (22 files, 2-3 keys each).
- Add: `docs/settings-placement.md`.
- Update: `features/advanced-menu-restoration/journal.md` (audit + execution notes).

## Single-source-of-truth placement

Not a dual-path concern (no preview / export pair). The two `<DiscardWords>` / `<AllowWords>` components remain the single React render site for those settings; this PR only changes *where in the tree* they render.

Note the connection to **Item 3** (`postprocessor-word-list-source-of-truth`): that feature ensures the cleanup manager actually consumes `custom_words` / `custom_filler_words` from settings instead of from a transcript-derived list. Item 3's coverage gate validates the backend contract; this feature only relocates the UI. They are independent and can land in either order.

## Data flow

```
Settings UI (Advanced tab)
  -> updateSetting("custom_words" | "custom_filler_words", string[])
  -> Tauri command -> persisted JSON
  -> backend reads at cleanup time (per Item 3)
```

No flow change vs today.

## Migration / compatibility

- No setting-schema migration. Existing word lists keep their values.
- A user mid-edit when the build lands will simply not see the inline block any more; their settings still apply server-side.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| New i18n keys missing in some locales | Use `i18n-pruning` skill; CI runs `check-translations.ts` | AC-003-a |
| Settings nav wiring is non-obvious; new tab fails to render | Find pattern by reading how `ExperimentalSettings` is wired before touching code | AC-002-a (live launch) |
| Inline block removal breaks editor layout (e.g., adjacent SettingsGroup spacing) | Live-launch verification per AGENTS.md "Verified means the live app" | AC-001-b |
| Audit becomes stale | Heuristic doc is short and authoritative; table in journal is allowed to evolve | AC-004-b |
| Scope creep into actually relocating other set-once settings | PRD R-004 explicitly says recommend-only; out-of-scope list in REQUEST.md is firm | n/a (process gate) |

## Implementation order suggestion

1. R-002 first (build the new Advanced page so users always have a route to the controls).
2. R-001 second (remove inline block).
3. R-003 in parallel (i18n keys can land before or after the UI code).
4. R-004 last (audit informed by the actual implementation).
