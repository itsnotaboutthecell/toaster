---
name: handy-legacy-pruning
description: 'Use before editing any file in the Handy-era dictation surface (actions.rs, shortcut/, overlay.rs, tray*.rs, clipboard.rs, input.rs, audio_feedback.rs, apple_intelligence.rs, audio_toolkit/audio/recorder.rs, audio_toolkit/vad/, PushToTalk.tsx, AudioFeedback.tsx, AccessibilityPermissions.tsx, HandyKeysShortcutInput.tsx). Forces the question "is this still on the transcript-editor path?" before extending dead code.'
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

## Dictation-Era Surface (High-Confidence)

Backend (`src-tauri/src/`):

- `actions.rs` — record / transcribe / paste pipeline
- `shortcut/` — global hotkey, PTT, paste method, auto-submit, typing tools
- `overlay.rs` — recording overlay window
- `tray.rs`, `tray_i18n.rs` — dictation-centric tray menu
- `clipboard.rs`, `input.rs` — paste to focused app / keyboard synthesis
- `audio_feedback.rs` — start/stop recording sounds
- `apple_intelligence.rs` (+ `src-tauri/swift/apple_intelligence*`) — dictation cleanup LLM
- `audio_toolkit/audio/recorder.rs`, `audio_toolkit/vad/*` — live mic capture + VAD

Frontend (`src/components/`):

- `AccessibilityPermissions.tsx` (partial — onboarding still live)
- `settings/general/GeneralSettings.tsx` and every component it imports (PushToTalk, AudioFeedback, VolumeSlider, MuteWhileRecording, PasteMethod, TypingTool, ClipboardHandling, AutoSubmit, AutostartToggle, StartHidden, ShowTrayIcon, SoundPicker, GlobalShortcutInput) — unreachable via Sidebar
- `settings/HandyKeysShortcutInput.tsx`
- `settings/debug/KeyboardImplementationSelector.tsx`
- Dictation-specific settings in `stores/settingsStore.ts` (19 fields: `audio_feedback*`, `sound_theme`, `start_hidden`, `autostart_enabled`, `push_to_talk`, `paste_delay_ms`, `paste_method`, `typing_tool`, `external_script_path`, `clipboard_handling`, `auto_submit*`, `mute_while_recording`, `append_trailing_space`, `extra_recording_buffer_ms`, `show_tray_icon`, `overlay_position`)

Ambiguous (audit before touching):

- `llm_client.rs` — used by dictation cleanup AND possibly the editor local-cleanup-review flow (see `LocalLlmWordProposal`, `LocalCleanupReviewState`)
- `audio_toolkit/text.rs` — may still be used by transcription segment post-processing
- `AccessibilityPermissions.tsx` + `AccessibilityOnboarding.tsx` — still rendered from `App.tsx` during onboarding; decide whether onboarding stays
- `post-processing/*` — UI components appear dead but the review modal in `App.tsx` is live

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

- Adding a new function to `actions.rs`
- Adding a new `change_*_setting` command to `shortcut/mod.rs`
- Adding a new field to `AppSettings` that only dictation UI reads
- Adding a new i18n key under a dictation-only group (`tray.*`, `settings.sound.*`, `settings.advanced.{autoSubmit,pasteMethod,typingTool,clipboardHandling,startHidden,autostart,showTrayIcon,overlay}`, `settings.debug.{soundTheme,muteWhileRecording,appendTrailingSpace,pasteDelay,recordingBuffer,alwaysOnMicrophone,keyboardImplementation}`)
- Adding a new npm dep because "the push-to-talk screen needs it"
- Extending `llm_client.rs` before determining whether the call is dictation or editor local-cleanup-review
- Adding a new screen under `settings/` that mirrors a dictation concern (sounds, paste, shortcuts, keyboard synth)

## Removal Procedure

When an audit verdict is FULLY DEAD:

1. Delete the module file.
2. Remove every `mod` declaration and `use` path referencing it.
3. Remove every `change_*` command registration from `lib.rs`.
4. Remove matching fields from `AppSettings` (`settings.rs`) and add a migration that strips them from persisted stores.
5. Remove the i18n keys from every locale under `src/i18n/locales/*/translation.json`.
6. Remove the Cargo/npm dependency if it has no other consumer (see `dep-hygiene` skill).
7. Run `cargo check`, `cargo clippy`, `cargo test`, `npm run lint`, `npm test`, and the monitored launch script. Verify the app still starts before moving on.

## When To Apply

- Before ANY edit to the files listed above
- When a new feature request touches recording, push-to-talk, paste, tray, overlay, shortcuts, accessibility permissions, audio feedback
- Before adding a new `change_*_setting` Tauri command
- Before adding a new top-level settings screen
