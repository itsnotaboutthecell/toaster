---
name: handy-legacy-pruning
description: 'Use before extending any remaining Handy-era code (dead settings fields in settings/types.rs and settings/defaults.rs, open_recordings_folder command in commands/mod.rs, llm_client.rs). Most dictation-era modules have been deleted; this skill guards residual dead code from receiving new features.'
---

# Handy Legacy Pruning

## Overview

Toaster was forked from Handy (a local dictation / push-to-talk app) and has pivoted to a **transcript-first video/audio editor**. Substantial Handy-era code remains in the repository. Adding features into that code locks in more dead weight and slows future cleanup.

**Core principle:** Never extend dictation-era modules. Either remove them, or migrate the needed piece into the editor surface.

## The Iron Law

```
IF A MODULE IS ONLY REACHABLE FROM THE DICTATION FLOW,
IT DOES NOT RECEIVE NEW CODE.
```

## Dictation-Era Surface (Remaining)

Most Handy-era dictation modules have been deleted. The following residual code remains:

Backend (`src-tauri/src/`):

- `commands/mod.rs` — `open_recordings_folder` command (dead, no UI caller)
- `settings/types.rs` — dead fields: `ShortcutBinding`, `start_hidden`, `bindings`, and related dictation settings
- `settings/defaults.rs` — default values for the dead settings fields above

Ambiguous (audit before touching):

- `llm_client.rs` — used by dictation cleanup AND possibly the editor local-cleanup-review flow (see `LocalLlmWordProposal`, `LocalCleanupReviewState`)

### Previously Deleted (for reference)

The following have been fully removed: `actions.rs`, `shortcut/`, `overlay.rs`, `tray.rs`, `tray_i18n.rs`, `clipboard.rs`, `input.rs`, `audio_feedback.rs`, `apple_intelligence.rs`, `audio_toolkit/audio/recorder.rs`, `audio_toolkit/vad/*`, `PushToTalk.tsx`, `AudioFeedback.tsx`, `AccessibilityPermissions.tsx`, `HandyKeysShortcutInput.tsx`, `GeneralSettings.tsx` and all its sub-components, `KeyboardImplementationSelector.tsx`.

## Gate Function

Before editing any file in the list above:

```
1. GREP: rg --fixed-strings "<symbol>" src src-tauri/src
2. TRACE: every caller to either
     a) src-tauri/src/lib.rs command registration invoked by an editor
        component, OR
     b) src/App.tsx → Sidebar (SECTIONS_CONFIG) → editor route
3. VERDICT:
     - No path found → FULLY DEAD. Do not extend. Remove or quarantine.
     - Path found but only through dictation routes → PARTIALLY DEAD.
       Extract the live piece into managers/ or commands/ and delete the rest.
     - Path found through an editor route → STILL LIVE. Edit normally,
       but document why in the commit.
4. CITE evidence in the PR description (file:line).
```

Important: Sidebar `SECTIONS_CONFIG` (`src/components/Sidebar.tsx`) is the gating root for settings UI. A component not in `SECTIONS_CONFIG` and not exported from `src/components/settings/index.ts` is unreachable even if it is imported by another dead component.

## Red Flags — STOP

- Adding a new field to `AppSettings` that only dictation UI reads
- Adding a new i18n key under a dictation-only group (`tray.*`, `settings.sound.*`, `settings.advanced.{autoSubmit,pasteMethod,typingTool,clipboardHandling,startHidden,autostart,showTrayIcon,overlay}`, `settings.debug.{soundTheme,muteWhileRecording,appendTrailingSpace,pasteDelay,recordingBuffer,alwaysOnMicrophone,keyboardImplementation}`)
- Extending `llm_client.rs` before determining whether the call is dictation or editor local-cleanup-review
- Extending `open_recordings_folder` or adding callers to it
- Adding new default values in `settings/defaults.rs` for dead dictation fields

## Removal Procedure

When an audit verdict is FULLY DEAD:

1. Delete the module file.
2. Remove every `mod` declaration and `use` path referencing it.
3. Remove every `change_*` command registration from `lib.rs`.
4. Remove matching fields from `AppSettings` (`settings.rs`) and add a migration that strips them from persisted stores.
5. Remove the i18n keys from every locale under `src/i18n/locales/*/translation.json`.
6. Remove the Cargo/npm dependency if it has no other consumer (see the `dep-hygiene` skill).
7. Run `cargo check`, `cargo clippy`, `cargo test`, `npm run lint`, `npm test`, and the monitored launch script. Verify the app still starts before moving on.

## When To Apply

- Before ANY edit to the remaining Handy-era files listed above
- When a new feature request touches dead settings fields (`ShortcutBinding`, `start_hidden`, etc.)
- Before extending `llm_client.rs` or `open_recordings_folder`
- Before adding a new `change_*_setting` Tauri command
- Before adding a new top-level settings screen
