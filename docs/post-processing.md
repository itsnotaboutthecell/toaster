# Post-processing (local LLM)

Toaster's post-processing cleanup runs a local Large Language Model (LLM). The default path runs the model **in-process** inside Toaster using a curated GGUF catalog (no separate server, no network calls). An "advanced" path also lets you point Toaster at a locally hosted OpenAI-compatible HTTP endpoint such as Ollama, LM Studio, or llama.cpp `./server`. **No Toaster build step bundles or hosts a model**, and there is no runtime network call to a hosted inference API. See [`AGENTS.md`](../AGENTS.md) "Local-only inference" for the project-wide rule this section implements.

## Default: Local (in-process) provider

Settings -> Post-Process -> Provider -> "Local (in-process)". This shows an in-app catalog of vetted GGUF models (e.g. Llama 3.2 1B / 3B, Qwen2.5 0.5B / 7B). For each model:

- **Download** fetches the GGUF from its upstream HuggingFace mirror, verifies sha256, and writes it under the app data dir's `llm/` folder. Disk-space preflight requires 2x the model size.
- **Use this model** marks the model as the active local LLM (`local_llm_model_id` in settings).
- **Delete** removes the file when no longer needed (the active model cannot be deleted).
- The download is resumable (a `.partial` file survives interruption) and cancellable.

Inference runs in-process via [`llama-cpp-2`](https://crates.io/crates/llama-cpp-2), gated behind the `local-llm` Cargo feature so default `cargo check` builds stay fast. Production releases enable the feature.

## Advanced: OpenAI-compatible HTTP server

If you prefer a self-hosted runtime, pick an OpenAI-compatible HTTP provider in the Provider dropdown:

- [**Ollama**](https://ollama.com) — `ollama serve` exposes `http://localhost:11434/v1`.
- [**LM Studio**](https://lmstudio.ai) — GUI with one-click "Start Server", `http://localhost:1234/v1`.
- [**llama.cpp**](https://github.com/ggerganov/llama.cpp) `./server` — exposes `/v1/chat/completions`.
- Any other local runtime that implements `POST /v1/chat/completions`.

For these providers, configure Base URL / API Key (if required) / Model name. Toaster never sends post-process traffic to a remote host.

## Where to configure it in Toaster

Settings -> Post-Process. The provider dropdown selects the path; the rest of the page reflects the selected provider:

1. **Local (in-process)** — shows the GGUF catalog above.
2. **OpenAI-compatible** — shows Base URL / API Key / Model fields.

When the active provider has nothing selected (no local model picked, or no remote model configured), an info banner appears (`settings.postProcessing.localLlmAlert.*`).



## Word lists drive the prompt

Cleanup's prompt is assembled from three inputs:

1. The default prompt template ("Improve Transcriptions"), which you can edit under Settings -> Post-Process -> Prompts.
2. The **Allow Words** list (Settings -> Advanced) — tokens the cleanup pass must not modify. These merge with transcript-derived tokens (digits, currency, etc.) and are passed to the LLM as protected tokens.
3. The **Discard Words** list (Settings -> Advanced) — filler words the cleanup pass should remove. When the list is empty, the cleanup prompt makes no mention of filler-word removal. The default template uses a `${filler_words}` placeholder that expands to your configured list (or "none" when empty).

These two lists are the single source of truth. Toaster does not ship a hardcoded filler list that overrides your configuration: if you clear the Discard Words list, the cleanup pass stops removing fillers.

## What Toaster does NOT do

- Does not call out to `api.openai.com`, `api.anthropic.com`, or any other hosted inference endpoint.
- Does not download or update models on your behalf.
- Does not auto-select a provider if none is configured; post-processing simply remains disabled until you wire one up.
