---
name: design-system
description: 'Use when editing `src/components/settings/**`, `src/components/ui/**`, or any UI that renders a settings surface. Enforces hero + SettingsGroup + SettingContainer anatomy, the live-update and live-preview fan-out contracts, and the no-hex + button-variant + settings-updater-coverage CI gates.'
---

# Design system

Toaster's aesthetic went through three QC rounds before becoming a
documented standard. This skill is the guard that keeps it from
regressing. Read [`docs/design-system.md`](../../../docs/design-system.md)
and [`src/components/settings/AGENTS.md`](../../../src/components/settings/AGENTS.md)
before editing any settings or UI primitive file.

## Invoke before you edit

- Any new top-level settings page
- Any `src/components/settings/**/*.tsx`
- Any `src/components/ui/*.tsx` (Button, Slider, ToggleSwitch, Dropdown, Badge, Alert, ProgressBar, Tooltip, SettingsGroup, SettingContainer)
- Any new `caption_*` / `export_*` / `loudness_*` / `normalize_audio_*` field in `AppSettings`

## Non-negotiable checks

1. **Hero pattern** on every new top-level settings page:
   `<h1 text-xl font-semibold mb-2>{title}</h1><p text-sm text-text/60>{description}</p>`.
   Both i18n keys live in all 20 locale files.
2. **Tokens only, no hex** in `.ts`/`.tsx`. Gate: `check:design-tokens`.
3. **`<Button variant="…">` not raw `<button>`**. Gate: `check:button-variants`.
4. **Live-update numeric controls.** Slider `onChange` fires during drag;
   preview-affecting setters bind there, not on commit.
5. **Live-preview fan-out.** Any new `Settings[K]` wired to a preview
   surface gets an entry in `settingUpdaters` in
   `src/stores/settingsStore.ts`. Gate: `check:settings-updater-coverage`.
6. **i18n parity.** Every user-visible string through `t("…")`, mirrored
   across all 20 locales. Gate: `check:translations`.

## Run before claiming done

```bash
bun run check:design-tokens
bun run check:button-variants
bun run check:settings-updater-coverage
bun run check:translations
npm run lint
npx tsc --noEmit
```

And a live-app smoke for any change that affects a preview:

```powershell
.\scripts\launch-toaster-monitored.ps1 -Duration 5m
```

Drag a slider — preview must move **during** drag. Change a color —
preview must repaint within one frame.

## Red flags that trip this skill

- Adding a new setting without a label + description pair.
- Hardcoded hex color in a component file.
- Raw `<button>` carrying `bg-logo-primary` or `bg-background-ui`.
- `onCommit` on a preview-affecting numeric slider.
- Updating `caption_profiles` (or any new preview setting) without also
  updating `settingUpdaters`.
- Duplicate per-group `description` prose on a page that already has a hero.
- New string in `en/translation.json` only, without mirrors in 19 others.

## Related skills

- `i18n-pruning` — every key must land in all 20 locales.
- `canonical-instructions` — AGENTS.md is the single source of truth;
  `.github/instructions/*.instructions.md` are pointers.
- `superpowers:verification-before-completion` — the gates above are the
  evidence; running them IS the verification.
