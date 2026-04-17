---
name: i18n-pruning
description: 'Use when deleting or renaming any i18next key, or when removing a UI component that owned keys. Ensures every src/i18n/locales/*/translation.json is updated in the same commit so check-translations.ts stays green.'
---

# i18n Pruning

## Overview

Toaster ships 22 locale files under `src/i18n/locales/*/translation.json`. When a UI component is deleted, its keys must be removed from every locale simultaneously — otherwise `scripts/check-translations.ts` fails and translators drift further from the source locale.

**Core principle:** Key changes touch all 22 locale files in one commit.

## The Iron Law

```
DELETING A KEY IN en/translation.json =
DELETING IT IN ALL 22 LOCALES, IN THE SAME COMMIT.
```

## Gate Function

**Deleting a component or setting screen:**

```
1. Find the t("...") calls in the deleted component. Note every key.
2. Identify the key group (top-level object) in en/translation.json.
3. If the entire group is now orphaned: delete the group from every locale.
4. If only some keys are orphaned: delete those specific keys from every locale.
5. Run: bun scripts/check-translations.ts
6. Run: npm run lint  (catches stray useTranslation references)
7. Run: npm run build  (catches stray t() calls)
```

**Renaming a key:**

```
1. Update en/translation.json with the new key name.
2. Update every other locale — carry the EXISTING translation under the new key.
3. Do NOT leave the old key behind as an alias.
4. Run check-translations.ts.
```

**Adding a key:**

```
1. Add it to en/translation.json first.
2. Add an English fallback value to every other locale (translators will fix later).
3. Never leave a locale without the key — check-translations.ts will fail.
```

## Dictation-Era Key Groups (High-Value Removal Targets)

When the Handy-era UI is deleted (see `handy-legacy-pruning` skill), the following groups in `en/translation.json` are orphaned in whole or in part. Removal candidates identified by the 2026-04 audit:

- `tray.*` (fully orphaned once tray is removed)
- `settings.general.pushToTalk`
- `settings.sound.*` (audio feedback, microphone, output device, volume)
- `settings.advanced.startHidden`, `autostart`, `showTrayIcon`, `overlay`, `pasteMethod`, `typingTool`, `clipboardHandling`, `autoSubmit`
- `settings.debug.soundTheme`, `alwaysOnMicrophone`, `muteWhileRecording`, `appendTrailingSpace`, `pasteDelay`, `recordingBuffer`, `keyboardImplementation`

Keep (still live):

- `onboarding.permissions.*` — used by `AccessibilityOnboarding.tsx`

Cross-check by grepping the translation files for these prefixes and confirming no live editor component still references them.

## Red Flags — STOP

- About to edit only `en/translation.json`
- `check-translations.ts` is failing and you're about to `git commit --no-verify`
- PR description says "i18n updates in follow-up" — no, do it now
- Adding a key with a hardcoded English string instead of using i18next

## When To Apply

- Any PR that deletes a UI component
- Any PR that renames or removes a setting field
- Any PR that adds user-visible text
- Any PR where `check-translations.ts` is failing
