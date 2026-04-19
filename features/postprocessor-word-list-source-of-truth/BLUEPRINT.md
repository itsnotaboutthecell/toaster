# Blueprint: Post-processor word-list source of truth

## Architecture decisions

- **R-001** (allow list -> protected tokens): the cleanup orchestrator at `mod.rs:483` is the single integration point. New helper `protected_tokens_from_settings(&AppSettings) -> Vec<String>` returns sanitized tokens from `settings.custom_words`. Merge with `extract_protected_tokens(transcription)` and dedupe via existing `dedupe_tokens` (line 116). Pass result to `protected_tokens_for_prompt`. The existing `prompts.rs` signature (already takes `&[String]`) is unchanged.
- **R-002** (discard list -> filler list): inspect the cleanup prompt template (likely owned by `PostProcessingSettingsPrompts.tsx` and persisted as a string in settings) for any literal filler list. Replace with a `${filler_words}` placeholder (if the template engine supports placeholders) **or** reshape the prompt builder to inject the list as a separate clause. Decision pending Phase 2 verification of the actual template format.
- **R-003** (docs + UI clarity): single doc page (preferred: extend `docs/build.md` with a new "Post-processing (local LLM)" section, since it's one paragraph). UI alert lives in `PostProcessingSettings.tsx` between the SettingsGroup header and the Provider row, conditionally rendered when `selectedProviderId` is unset / default. Reuses the existing `<Alert>` component already imported there.
- **R-004** (i18n): one or two new keys (`settings.postProcessing.localLlmAlert.title`, `.body`, optional `.link`).

## Component & module touch-list

- Edit: `src-tauri/src/managers/cleanup/mod.rs` - add `protected_tokens_from_settings` helper; wire merge at line 483.
- Possibly add: `src-tauri/src/managers/cleanup/protected_tokens.rs` if helper grows past ~30 lines.
- Edit: cleanup prompt template owner (location confirmed in Phase 2 - likely `PostProcessingSettingsPrompts.tsx` for the user-editable template + a default-template constant in Rust).
- Edit: `src-tauri/src/managers/cleanup/prompts.rs` - if filler-list interpolation requires a builder change (likely a new `filler_words: &[String]` parameter on the legacy prompt builder).
- Add: `src-tauri/src/managers/cleanup/tests.rs` - two new `#[test]` cases.
- Edit: `src/components/settings/post-processing/PostProcessingSettings.tsx` - add conditional `<Alert>`.
- Edit: `docs/build.md` - new "Post-processing (local LLM)" section.
- Edit: `src/i18n/locales/*/translation.json` - new alert keys.
- Update: `features/postprocessor-word-list-source-of-truth/journal.md`.

## Single-source-of-truth placement

This feature **is** the SSOT enforcement for cleanup-related word lists. Decisions:

- `AppSettings.custom_words` is the single source of "words the user wants protected from cleanup". Cleanup reads from here. No duplicate list anywhere.
- `AppSettings.custom_filler_words` is the single source of "words the user considers fillers and wants removed". Cleanup reads from here. No hardcoded filler list anywhere in `src-tauri/src/managers/cleanup/`.
- The transcript-derived `extract_protected_tokens` heuristic is **complementary**, not a replacement: it adds tokens that look programmatic (digits, symbols) which the user is unlikely to enumerate by hand. Both sources merge into the prompt.
- The frontend Allow / Discard word-list components (`src/components/settings/AllowWords.tsx`, `DiscardWords.tsx`) remain the only UI editing surface. They write to the same setting keys consumed by cleanup. No duplication.

## Data flow

```
user types in AllowWords UI
  -> updateSetting("custom_words", string[])
  -> AppSettings.custom_words persisted
  -> on cleanup attempt: protected_tokens_from_settings(&settings)
     merged with extract_protected_tokens(transcription)
     deduped -> protected_tokens_for_prompt
     -> build_cleanup_contract_system_prompt(template, protected_tokens_for_prompt)
     -> LLM dispatch (local provider, no hosted endpoints)
```

## Migration / compatibility

- Existing user settings unchanged.
- Users with empty Allow / Discard lists see no behavioural change (transcript heuristic still applies for protected tokens; cleanup prompt's filler clause degrades gracefully).
- Users who had relied on a hardcoded filler list inside the cleanup template will see an apparent regression unless they populate `custom_filler_words`. Mitigation: ship a default seed list in `src/i18n/locales/en/translation.json` for the discard-words placeholder text **OR** in a one-time migration that fills `custom_filler_words` if it has never been set. Choose during execution; document in journal.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Empty allow list breaks prompt | Existing `if protected_tokens.is_empty()` branch in `prompts.rs:10` | AC-001-b |
| Empty discard list leaves orphan placeholder visible to LLM | Template builder substitutes empty -> omits clause; tested | AC-002-c |
| User-injected token contains prompt-injection payload | Backend sanitization helper | AC-001-c |
| Hidden hardcoded filler list elsewhere in repo | rg sweep before declaring done | AC-002-b |
| User confusion about model dependency | UI alert + doc section with three concrete answers | AC-003-a, AC-003-b |
| Hosted API accidentally introduced | Code review check + no new external SDK in Cargo.toml diff | AC-003-c |
| Cleanup test regression masks the wiring | New tests assert presence, not just absence | AC-001-a, AC-002-a |

## Implementation order suggestion

1. R-002 first: locate the hardcoded filler list (Phase 2 grep) and remove it. This is the explicit AGENTS.md anti-pattern; resolving it first removes the highest-priority debt.
2. R-001: wire allow list as protected tokens.
3. Tests (AC-001-a, AC-002-a) before any prompt-template edits.
4. R-003 docs + UI alert.
5. R-004 i18n.
