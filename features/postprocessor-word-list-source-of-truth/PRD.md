# PRD: Post-processor word-list source of truth

## Problem & Goals

(a) Clarify the post-processor's model dependency in user-visible docs and UI; (b) make `AppSettings.custom_words` and `AppSettings.custom_filler_words` the single source of truth for protected tokens and filler-word list in the cleanup prompt, replacing transcript-only heuristics and hardcoded lists. See `REQUEST.md` for full rationale.

## Scope

### In scope

- Backend wiring of `custom_words` -> protected tokens and `custom_filler_words` -> filler list in the cleanup prompt builder.
- Removal of any hardcoded filler list in the cleanup template / prompts.
- Cargo test asserting the wiring.
- Docs + UI clarity on local-only model requirement.

### Out of scope (explicit)

- Hosted-provider option.
- In-app model downloader.
- ASR model registry changes.
- Prompt-format / structured-output schema overhaul.

## Requirements

### R-001 - Allow list flows into cleanup prompt as protected tokens

- Description: at the call site in `src-tauri/src/managers/cleanup/mod.rs:483` where `protected_tokens_for_prompt` is built, merge `settings.custom_words` with the transcript-derived tokens (deduped, sanitized).
- Rationale: the user's Allow list expresses "do not modify these words" intent and must be honoured by cleanup.
- Acceptance Criteria
  - AC-001-a - `cd src-tauri; cargo test cleanup_prompt_includes_custom_words -- --nocapture` exits 0. The test sets `custom_words = ["Toaster", "Tauri"]` and asserts the resulting prompt string contains both.
  - AC-001-b - When `custom_words` is empty, the prompt builder still works (transcript-derived tokens behave as today).
  - AC-001-c - Tokens from `custom_words` are sanitized in the prompt (no raw `<>"'&` characters interpolated; reuse the existing front-end sanitizer pattern from `src/components/settings/AllowWords.tsx:24` server-side).

### R-002 - Discard list replaces hardcoded filler-word list

- Description: any hardcoded filler-word list in the cleanup prompt template is replaced by interpolation from `settings.custom_filler_words`. If the template currently uses a `${filler_words}` placeholder (or similar), substitute from settings; if there is no placeholder but a literal list, refactor the template to take one.
- Rationale: AGENTS.md SSOT rule explicitly cites this hardcoded list as the canonical anti-pattern.
- Acceptance Criteria
  - AC-002-a - `cd src-tauri; cargo test cleanup_prompt_uses_custom_filler_words -- --nocapture` exits 0. Test sets `custom_filler_words = ["um", "uh", "like"]` and asserts the prompt string contains all three and does NOT contain a different hardcoded set.
  - AC-002-b - `rg -n "\\bum\\b.*\\buh\\b|\\bfiller_words\\s*=\\s*\\[" src-tauri/src/managers/cleanup` returns zero matches for any inline literal filler list (no `["um", "uh", ...]` left in source).
  - AC-002-c - When `custom_filler_words` is empty, the prompt degrades gracefully (template either omits the filler clause or substitutes a benign string; no orphan `${...}` placeholder reaches the LLM).

### R-003 - Document the model dependency

- Description: add a short, user-readable section to `docs/build.md` (or new `docs/post-processing.md`) titled "Local LLM for post-processing" stating concretely which providers / endpoints work, what the user must install (e.g. Ollama / LM Studio / a local llama.cpp server), and that no Toaster build step downloads or hosts a model. Add an inline `<Alert>` in `PostProcessingSettings.tsx` linking to that section when no Provider has been configured.
- Rationale: the user's question "do we need to download something new?" must be answerable from the UI without source-diving; AGENTS.md "Local-only inference" must be visibly true.
- Acceptance Criteria
  - AC-003-a - The doc section exists and answers all three questions: which providers / endpoints, what the user installs, no built-in download.
  - AC-003-b - Live launch: opening Settings -> Post-processing with no Provider configured shows a non-blocking alert linking to the doc section.
  - AC-003-c - Code review confirms no hosted-API URL / SDK was added (no new `https://api.openai.com` or similar; reuses existing `local_openai_provider` machinery at `mod.rs:482`, `prompts.rs`, `llm_dispatch.rs`).

### R-004 - i18n parity for new strings

- Description: any user-visible string added by R-003 (alert text, link label) goes through i18n.
- Rationale: AGENTS.md i18n rule.
- Acceptance Criteria
  - AC-004-a - `bun run scripts/check-translations.ts` exits 0 after the change.

## Edge cases & constraints

- Empty allow / discard lists must not break the prompt.
- Backend-side sanitization of user-provided tokens (defence in depth).
- ASCII only in artifacts.
- File-size cap: `mod.rs` is large; if the merge logic adds significant code, extract to `cleanup/protected_tokens.rs`.

## Data model

No new persisted fields. Reuse `AppSettings.custom_words: Vec<String>` and `AppSettings.custom_filler_words: Vec<String>`.

## Non-functional requirements

- No new runtime network calls.
- Cleanup attempts must remain local-only.
