---
name: i18n-pruning
description: 'Use when deleting, renaming, or adding any user-visible i18next key. Ensures every src/i18n/locales/*/translation.json moves together so scripts/check-translations.ts stays green. Toaster-specific; use alongside superpowers:verification-before-completion.'
---

# i18n Pruning

Toaster ships 20 locale files under `src/i18n/locales/*/translation.json`.
`scripts/check-translations.ts` enforces parity and fails CI if they drift.

```
DELETING A KEY IN en/translation.json =
DELETING IT IN ALL 20 LOCALES, IN THE SAME COMMIT.
```

## Deleting a component or setting screen

1. Find every `t("...")` call in the deleted code. Note each key.
2. Identify the key group (top-level object) in `en/translation.json`.
3. Delete the orphaned group (or specific keys) from every locale.
4. Run `bun scripts/check-translations.ts`.
5. Run `npm run lint` (catches stray `useTranslation`).
6. Run `npm run build` (catches stray `t()` calls).

## Renaming a key

1. Update `en/translation.json` with the new name.
2. Update every other locale — carry the existing translation to the new key.
3. Do **not** leave the old key behind as an alias.
4. Run `check-translations.ts`.

## Adding a key

1. Add to `en/translation.json` first.
2. Add an English fallback to every other locale. Translators fix later.
3. Never leave a locale without the key.

## Dictation-era removal candidates (2026-04 audit)

Orphaned once the Handy-era UI is deleted — coordinate with
`handy-legacy-pruning`:

- `tray.*` (fully orphaned once tray is removed)
- `settings.general.pushToTalk`
- `settings.sound.*`
- `settings.advanced.{startHidden,autostart,showTrayIcon,overlay,pasteMethod,typingTool,clipboardHandling,autoSubmit}`
- `settings.debug.{soundTheme,alwaysOnMicrophone,muteWhileRecording,appendTrailingSpace,pasteDelay,recordingBuffer,keyboardImplementation}`

Keep: `onboarding.permissions.*` (used by `AccessibilityOnboarding.tsx`).

## Red flags

- About to edit only `en/translation.json`.
- `check-translations.ts` failing and reaching for `--no-verify`.
- "i18n updates in a follow-up PR" — no, same commit.
- Adding a key with a hardcoded English string instead of using i18next.

## Related skills

- `handy-legacy-pruning` — drives the "which keys are orphaned" question.
- `superpowers:verification-before-completion` — `check-translations.ts`
  output is the evidence that parity holds.
