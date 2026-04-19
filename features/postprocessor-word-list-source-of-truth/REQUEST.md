# Feature request: Post-processor word-list source of truth

## 1. Problem & Goals

User feedback raises two distinct sub-questions:

> "For the Post processor section - how does this tie into our model list? Do we need to download something new to enable this capability? The transcript template hardcodes filler words but a lot of these should be inherited from the word menu list as variables."

### Sub-question 3a: model dependency

The post-processor surface (`src/components/settings/post-processing/PostProcessingSettings.tsx`) shows Provider / API Key / Base URL / Model selectors, suggesting it routes to an external or local OpenAI-compatible LLM. Toaster's `AGENTS.md` "Local-only inference" rule forbids hosted dependencies. We need to **document concretely** in the PRD whether enabling post-processing requires a new model download (and from where), or whether it reuses the existing ASR model registry, or whether it expects the user to bring their own local provider (e.g. Ollama). The current Settings UI does not surface this clearly, and we cannot guess.

### Sub-question 3b: hardcoded vs configured filler / protected words

The cleanup manager extracts "protected tokens" purely from the transcript text via `extract_protected_tokens` (`src-tauri/src/managers/cleanup/mod.rs:105-114`). It looks for tokens containing digits or "protected symbols" - this is a transcript-derived heuristic, **not** a read of the user's `custom_words` setting. Meanwhile, the user's Allow / Discard word lists in Settings are stored under `custom_words` / `custom_filler_words` and are consumed elsewhere (filler-word filtering). Today the cleanup post-processor's prompt does not include the user's allow list as protected tokens.

This is the same anti-pattern AGENTS.md calls out:

> "the hardcoded filler list both came from violating this rule" (Single source of truth for dual-path logic)

The user's Allow list should flow into the cleanup prompt as additional protected tokens. The user's Discard list should flow into the cleanup prompt template as the canonical filler-word list (so the prompt can say "remove these specific filler words" instead of relying on the model to guess).

**Goal:** (a) document the model dependency clearly in user-visible docs and the post-processor UI; (b) wire the user's `custom_words` and `custom_filler_words` settings into the cleanup prompt as the single source of truth, removing the transcript-only heuristic as the sole protected-token source and removing any hardcoded filler-word list from the prompt template.

## 2. Desired Outcome & Acceptance Criteria

- A user reading Settings -> Post-processing or `docs/build.md` can answer "do I need to download something to use this?" without source-diving.
- The cleanup prompt sent to the LLM lists the user's Allow words as protected tokens (in addition to the transcript-derived ones).
- The cleanup prompt either includes the user's Discard list as the explicit filler-word list, or - if the prompt template uses a `${filler_words}` placeholder - the placeholder is substituted from `custom_filler_words` rather than a hardcoded list.
- A new cargo test verifies the contract: given a user with `custom_words = ["Toaster", "Tauri"]` and `custom_filler_words = ["um", "uh"]`, the prompt string contains those tokens.
- No hosted-inference dependency added (Toaster remains local-only).

(See `PRD.md` for the formalized AC list.)

## 3. Scope Boundaries

### In scope

- Audit `src-tauri/src/managers/cleanup/` and any prompt template for hardcoded filler / protected tokens; replace with reads from `AppSettings.custom_words` and `AppSettings.custom_filler_words`.
- Extend `extract_protected_tokens` (or its caller in `mod.rs:483`) to additionally include `settings.custom_words` in the protected-token list passed to the prompt builder.
- Substitute filler-word placeholders in the cleanup prompt template from `settings.custom_filler_words`.
- Add a Rust unit / integration test in `src-tauri/src/managers/cleanup/tests.rs` validating the wiring.
- Document the model dependency: edit `docs/build.md` (or add `docs/post-processing.md`) and the PostProcessingSettings UI to state plainly which models are required, where to obtain them, and confirm local-only.
- i18n: any new help-text strings.

### Out of scope (explicit)

- Adding a hosted (cloud) post-processing provider option.
- Building an in-app model downloader for cleanup (separate feature if needed).
- Changing the ASR model registry.
- Reworking the post-processing prompt format / structured-output schema.
- Rewriting the prompt-template UI in `PostProcessingSettingsPrompts.tsx`.

## 4. References to Existing Code

- `src-tauri/src/managers/cleanup/mod.rs:105-114` - `extract_protected_tokens` (transcript-only heuristic; needs to be extended or wrapped to also include `settings.custom_words`).
- `src-tauri/src/managers/cleanup/mod.rs:483-484` - call site that builds `protected_tokens_for_prompt`. The settings-derived tokens get merged here.
- `src-tauri/src/managers/cleanup/prompts.rs:5-67` - prompt builders that consume `protected_tokens: &[String]`. The signature already takes a slice; the caller is the integration point.
- `src-tauri/src/managers/cleanup/tests.rs:1-100` - existing test suite; new test goes here.
- `src-tauri/src/managers/cleanup/llm_dispatch.rs:40, 66, 172` - uses `protected_tokens_for_prompt` from the inputs struct; no changes needed here.
- `src/components/settings/post-processing/PostProcessingSettings.tsx:43-141` - UI surface that needs the model-dependency clarification.
- `src/components/settings/post-processing/PostProcessingSettingsPrompts.tsx` - the prompt-template editor (~9 KB); likely has the hardcoded filler list the user is naming. Needs verification during analysis.
- `src-tauri/src/managers/model/` - ASR model registry; reference only.
- `AGENTS.md` "Single source of truth for dual-path logic" + "Local-only inference" - the binding rules.

## 5. Edge Cases & Constraints

- `custom_words` may be empty - prompt builder must produce a sensible string (it already handles `protected_tokens.is_empty()` at `prompts.rs:10`).
- `custom_filler_words` may be empty - the prompt template must degrade gracefully (no orphan "${filler_words}" placeholder visible to the LLM).
- A user could put strings into `custom_words` that include prompt-injection payloads - sanitize / quote tokens before interpolating into the prompt. (`src/components/settings/AllowWords.tsx:24` already strips `<>"'&` on input but the backend should defend in depth.)
- ASCII only in artifacts.
- All cleanup must remain local: no provider added that calls hosted endpoints.

## 6. Data Model (optional)

No persisted-setting change. The wiring consumes existing `AppSettings.custom_words: Vec<String>` and `AppSettings.custom_filler_words: Vec<String>` on the backend.

## Q&A

Resolved 2026-04-18:

- **Authoritative rule: respect what the UI shows, no hardcoded values.** The cleanup pipeline must pull its filler and allow lists from the same `custom_words` / `custom_filler_words` settings the UI renders. Remove any hardcoded filler list inside the cleanup module; remove any hardcoded default seed that would reintroduce values not visible in the UI. If the UI lists are empty, the cleanup pipeline operates on empty lists - no phantom "default" filler removal happens behind the user's back.
- **Merging with transcript heuristic:** The transcript-derived protected-token heuristic (proper nouns, acronyms, etc.) is *detection*, not a user-configured value, and remains. It is merged with the user's Allow list (union + dedupe). The heuristic is never replaced by an empty user list, and the user list is never overridden by the heuristic.
- **Model-dependency disclosure:** Inline alert in PostProcessingSettings + a docs section explaining the local OpenAI-compatible endpoint requirement (no new model download needed beyond what is already wired).
