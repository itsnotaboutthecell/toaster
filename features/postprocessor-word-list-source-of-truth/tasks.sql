-- Task graph for postprocessor-word-list-source-of-truth.
-- Ingest into the session SQL store with the `sql` tool.

INSERT INTO todos (id, title, description, status) VALUES
  ('postprocessor-word-list-audit-hardcoded',
   'Audit cleanup module for hardcoded filler / protected lists',
   'Run rg -n "\"um\"|\"uh\"|\"like\"|filler_words\\s*=" src-tauri/src/managers/cleanup and rg over the default cleanup prompt template (search src-tauri and src for the literal default text). Document EVERY occurrence in features/postprocessor-word-list-source-of-truth/journal.md before changing anything. This audit feeds R-002. Verifier: AC-002-b per coverage.json (must reach zero matches by end of feature).',
   'pending'),

  ('postprocessor-word-list-protected-tokens',
   'Wire AppSettings.custom_words into cleanup protected tokens',
   'In src-tauri/src/managers/cleanup/mod.rs add a helper `fn protected_tokens_from_settings(settings: &AppSettings) -> Vec<String>` that returns sanitized tokens from settings.custom_words (strip <>"&\047 to match the frontend sanitizer at src/components/settings/AllowWords.tsx:24). At the call site near line 483, merge the helper output with extract_protected_tokens(transcription) and pass through dedupe_tokens. If the helper grows past ~30 lines, extract to cleanup/protected_tokens.rs. Verifier: AC-001-a, AC-001-b, AC-001-c per coverage.json.',
   'pending'),

  ('postprocessor-word-list-filler-words',
   'Replace hardcoded filler list with custom_filler_words substitution',
   'Based on the audit findings, refactor the cleanup prompt template / prompt builder so the filler-word list is sourced from settings.custom_filler_words. If the template uses a placeholder like ${filler_words}, substitute from settings. If the template has a literal list, refactor build_cleanup_*_prompt in prompts.rs to take a filler_words: &[String] parameter and inject as a separate clause that is omitted when empty. Decide migration strategy for users with empty discard lists (either ship a default seed in the English translation.json, or one-time migration filling custom_filler_words on first launch). Document the chosen approach in journal.md. Verifier: AC-002-a, AC-002-b, AC-002-c per coverage.json.',
   'pending'),

  ('postprocessor-word-list-tests',
   'Add cargo tests for cleanup prompt wiring',
   'In src-tauri/src/managers/cleanup/tests.rs add two #[test] cases: (a) cleanup_prompt_includes_custom_words - construct AppSettings with custom_words = ["Toaster", "Tauri"], drive the cleanup prompt builder, assert the result string contains "Toaster" and "Tauri"; (b) cleanup_prompt_uses_custom_filler_words - construct AppSettings with custom_filler_words = ["um", "uh", "like"], drive the prompt builder, assert all three appear in the prompt and that no other hardcoded filler set leaks through. Verifier: AC-001-a, AC-002-a per coverage.json.',
   'pending'),

  ('postprocessor-word-list-docs-ui',
   'Document local-LLM dependency + add UI alert',
   'Add a "Post-processing (local LLM)" section to docs/build.md answering (a) which providers/endpoints are supported, (b) what the user installs (e.g. Ollama / LM Studio / local llama.cpp), (c) confirmation that no Toaster build step downloads a model. In src/components/settings/post-processing/PostProcessingSettings.tsx add a conditional <Alert> rendered when selectedProviderId is unset/default, body keyed to settings.postProcessing.localLlmAlert.body, link to the doc section. Verifier: AC-003-a, AC-003-b, AC-003-c per coverage.json.',
   'pending'),

  ('postprocessor-word-list-i18n',
   'Add localLlmAlert i18n keys to all 22 locales',
   'Invoke i18n-pruning. Add settings.postProcessing.localLlmAlert.title and .body (and optional .link) to every src/i18n/locales/*/translation.json. English defaults: "Local LLM required" / "Toaster post-processing runs against a local OpenAI-compatible endpoint. See docs/build.md#post-processing for setup." Verifier: AC-004-a per coverage.json.',
   'pending'),

  ('postprocessor-word-list-qc',
   'QC: cargo test + grep audit + live launch + i18n gate',
   'Run (1) cd src-tauri; cargo test cleanup -- --nocapture; expect both new tests pass and existing cleanup tests stay green. (2) rg -n "\\bum\\b.*\\buh\\b|filler_words\\s*=\\s*\\[" src-tauri/src/managers/cleanup; expect zero matches. (3) bun run scripts/check-translations.ts; expect exit 0. (4) pwsh scripts/launch-toaster-monitored.ps1 -ObservationSeconds 180; open Settings -> Post-processing with no provider configured, confirm the alert renders and links to docs. Append results to journal.md.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('postprocessor-word-list-protected-tokens', 'postprocessor-word-list-audit-hardcoded'),
  ('postprocessor-word-list-filler-words',     'postprocessor-word-list-audit-hardcoded'),
  ('postprocessor-word-list-tests',            'postprocessor-word-list-protected-tokens'),
  ('postprocessor-word-list-tests',            'postprocessor-word-list-filler-words'),
  ('postprocessor-word-list-i18n',             'postprocessor-word-list-docs-ui'),
  ('postprocessor-word-list-qc',               'postprocessor-word-list-tests'),
  ('postprocessor-word-list-qc',               'postprocessor-word-list-i18n');
