# src/components/settings — UI conventions (path-scoped)

Authoritative rules for settings screens. Nearest-AGENTS.md wins per
[agents.md spec](https://agents.md/). See [`../../../docs/design-system.md`](../../../docs/design-system.md)
for the full design system (tokens, primitives, CI gates).

## Must-do for every settings page

1. **Hero on top.** `<h1 text-xl font-semibold mb-2>` + `<p text-sm text-text/60>`
   wrapping the page. Models/Advanced/About follow this. Both i18n keys
   (`settings.<page>.title` + `settings.<page>.description`) exist in all
   20 locale files.
2. **One `<h1>` per page.** `SettingsGroup` titles are `<h2>`. Don't
   duplicate the page title on the first group.
3. **Per-group descriptions are opt-in.** Default is title-only — round-3
   QC removed the noisy redundant prose. Reach for `description` only for
   onboarding framing (e.g. Experimental banner).
4. **Every setting has a label + description.** Never surface raw flag
   names (`caption_bg_alpha_b3` is a bug — "Background transparency" is
   the label). Description lives in a `Tooltip` behind the info-icon by
   default (`descriptionMode="tooltip"`).
5. **Numeric controls are live-update.** `SliderWithInput`'s `onChange`
   fires during drag. Bind preview-affecting setters to `onChange`, not
   `onCommit`. Debounce IPC downstream, not UI upstream.
6. **Live-preview setters fan out through `settingUpdaters`.** Any
   `Settings` key that drives a live preview (captions, export, loudness)
   MUST have an entry in `src/stores/settingsStore.ts > settingUpdaters`.
   Missing entry = silent no-op (round-3 `caption_profiles` bug). Gate:
   `bun run check:settings-updater-coverage`.

## Must-not

- **No raw `<button>` styled with `bg-logo-primary` / `bg-background-ui`**
  — use `<Button variant="brand|primary|secondary">`. Gate:
  `bun run check:button-variants`.
- **No hex color literals** in `.ts`/`.tsx`. All colors via tokens. Gate:
  `bun run check:design-tokens`.
- **No red-on-dark, no light-grey-on-white** — readability regressions.
- **No hardcoded English strings** — route every user-facing string
  through i18next; mirror across all 20 locales.

## Primitives you reach for

| Control | Primitive | File |
|---------|-----------|------|
| Button / action | `<Button variant="…">` | `src/components/ui/Button.tsx` |
| Boolean | `<ToggleSwitch>` | `src/components/ui/ToggleSwitch.tsx` |
| Numeric | `<SliderWithInput>` | `src/components/ui/SliderWithInput.tsx` |
| One-of-N (> 3 options) | `<Dropdown>` | `src/components/ui/Dropdown.tsx` |
| State pill | `<Badge>` | `src/components/ui/Badge.tsx` |
| Inline warning | `<Alert>` | `src/components/ui/Alert.tsx` |
| Row wrapper | `<SettingContainer>` | `src/components/ui/SettingContainer.tsx` |
| Card wrapper | `<SettingsGroup>` | `src/components/ui/SettingsGroup.tsx` |

## Review checklist before PR

- [ ] Hero block present on any new top-level settings page.
- [ ] `SettingsGroup` titles unique; none duplicates the hero.
- [ ] Every user-visible string uses `t("…")`; `check:translations` green.
- [ ] Numeric preview controls bind to `onChange` (live), not `onCommit`.
- [ ] Every new `caption_*`/`export_*`/`loudness_*`/`normalize_audio_*`
      key has a `settingUpdaters` entry; `check:settings-updater-coverage` green.
- [ ] No raw `<button>` carrying brand/UI bg classes; `check:button-variants` green.
- [ ] No hex literals; `check:design-tokens` green.
- [ ] Live QC: drag a slider — preview moves **during** drag. Change a
      color — live preview repaints within one frame.

## Related

- [`../../../docs/design-system.md`](../../../docs/design-system.md) — full token table, anatomy, contracts, gates.
- [`../../AGENTS.md`](../../AGENTS.md) — frontend conventions (TypeScript, i18n, bindings.ts).
- [`../../../.github/skills/design-system/SKILL.md`](../../../.github/skills/design-system/SKILL.md) — the skill to invoke when editing these files.
