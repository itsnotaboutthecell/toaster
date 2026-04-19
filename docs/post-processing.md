# Post-processing (local LLM)

Toaster's post-processing cleanup runs a local Large Language Model (LLM). The default path runs the model **in-process** inside Toaster using a curated GGUF catalog (no separate server, no network calls). An "advanced" path also lets you point Toaster at a locally hosted OpenAI-compatible HTTP endpoint such as Ollama, LM Studio, or llama.cpp `./server`. **No Toaster build step bundles or hosts a model**, and there is no runtime network call to a hosted inference API. See [`AGENTS.md`](../AGENTS.md) "Local-only inference" for the project-wide rule this section implements.

## Who sees this feature (Simple vs. Expert mode)

Toaster ships with a **Simple / Expert** split so the default surface stays quiet for users who only want to edit by deleting words.

- **Simple (default):** LLM cleanup is invisible. Settings shows Editor, Models, Advanced, About. The Editor's `Remove fillers` button still runs the deterministic rule-based filler removal (`cleanup_all`); no LLM runs.
- **Expert mode on:** Settings -> Advanced -> Expert mode toggle. Enabling this reveals the `AI cleanup (LLM connection)` group in Advanced (provider / base URL / API key / model selection, plus the `Run AI cleanup at transcription` execution toggle) and an `AI cleanup prompt` drawer in the Editor (between Transcription and Edit).

The two toggles are independent on purpose:

- `ui_expert_mode_enabled` — UI visibility gate. Hides every LLM-related surface when off.
- `post_process_enabled` — execution gate. Even with Expert mode on, LLM cleanup only runs when this is also on, which makes A/B'ing raw vs cleaned output a single-checkbox operation.

## Default: Local (in-process) provider

Settings -> Advanced -> AI cleanup (LLM connection) -> Provider -> "Local (in-process)". The Models tab is the single place where vetted GGUFs (Llama 3.2 1B / 3B, Qwen2.5 0.5B / 7B, ...) are downloaded, pinned, and deleted. Use the category filter there to narrow to `Post-processing`; the active post-processor shows the same `active` badge as the active transcription model and floats to the top.

For each model in the Models tab:

- **Download** fetches the GGUF from its upstream HuggingFace mirror, verifies sha256, and writes it under the app data dir's `llm/` folder. Disk-space preflight requires 2x the model size.
- **Use this model** pins the model as the active local LLM for post-processing (`post_process_models[local]` in settings). Transcription and post-processing each have exactly one active model at a time.
- **Delete** removes the file when no longer needed (the active model cannot be deleted).
- The download is resumable (a `.partial` file survives interruption) and cancellable.

Inference runs in-process via [`llama-cpp-2`](https://crates.io/crates/llama-cpp-2), gated behind the `local-llm` Cargo feature so default `cargo check` builds stay fast. Production releases enable the feature.

## Advanced: OpenAI-compatible HTTP server

If you prefer a self-hosted runtime, pick an OpenAI-compatible HTTP provider in the Provider dropdown (Expert mode -> Advanced -> AI cleanup):

- [**Ollama**](https://ollama.com) — `ollama serve` exposes `http://localhost:11434/v1`.
- [**LM Studio**](https://lmstudio.ai) — GUI with one-click "Start Server", `http://localhost:1234/v1`.
- [**llama.cpp**](https://github.com/ggerganov/llama.cpp) `./server` — exposes `/v1/chat/completions`.
- Any other local runtime that implements `POST /v1/chat/completions`.

For these providers, configure Base URL / API Key (if required) / Model name. Toaster never sends post-process traffic to a remote host.

## Where to configure it in Toaster

With Expert mode enabled:

1. **Settings -> Advanced -> AI cleanup (LLM connection)** — provider, endpoint, model, and the execution toggle.
2. **Editor -> AI cleanup prompt** (appears between Transcription and Edit when a transcript is loaded) — edit, create, or delete prompt templates.
3. **Settings -> Models** — download / pin / delete the post-processor GGUF used by the Local provider.

When the active provider has nothing selected (no local model picked, or no remote model configured), an info banner appears inside the LLM connection group (`settings.postProcessing.localLlmAlert.*`).

## Word lists drive the prompt

Cleanup's prompt is assembled from three inputs:

1. The default prompt template ("Improve Transcriptions"), which you can edit under Editor -> AI cleanup prompt (Expert mode).
2. The **Allow Words** list (Settings -> Advanced) — tokens the cleanup pass must not modify. These merge with transcript-derived tokens (digits, currency, etc.) and are passed to the LLM as protected tokens.
3. The **Discard Words** list (Settings -> Advanced) — filler words the cleanup pass should remove. When the list is empty, the cleanup prompt makes no mention of filler-word removal. The default template uses a `${filler_words}` placeholder that expands to your configured list (or "none" when empty).

These two lists are the single source of truth. Toaster does not ship a hardcoded filler list that overrides your configuration: if you clear the Discard Words list, the cleanup pass stops removing fillers.

## What Toaster does NOT do

- Does not call out to `api.openai.com`, `api.anthropic.com`, or any other hosted inference endpoint.
- Does not download or update models on your behalf.
- Does not auto-select a provider if none is configured; post-processing simply remains disabled until you wire one up.
- Does not expose the LLM surface to Simple users; Expert mode must be toggled on explicitly.
