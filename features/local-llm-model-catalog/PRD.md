# PRD: local-llm-model-catalog

## R-001 — Curated LLM catalog shipped in-app

A static catalog in `src-tauri/src/managers/llm/catalog.rs` lists 3-5 models.
Proposed v1 roster (blueprint may refine):
qwen2.5-0.5b-instruct-q4,
llama-3.2-1b-instruct-q4 (recommended default),
llama-3.2-3b-instruct-q4,
qwen2.5-7b-instruct-q4.
Each entry carries id, display_name, size_bytes, sha256, quantization, download_url, context_length, recommended_ram_gb, is_recommended_default.

- AC-001-a — cargo test `llm_catalog_is_nonempty_and_has_exactly_one_default` passes.
- AC-001-b — cargo test `llm_catalog_entries_have_required_fields` asserts each entry has non-empty id, sha256 len==64, size_bytes>0, URL scheme https.

## R-002 — Download / verify / delete / cancel UX parity with Whisper

Post-Process page grows a "Local models" section that uses the same ModelCard (or near-clone) as Models, wired to the same progress/speed/verify/cancel state shape.

- AC-002-a — Live-app QC step: download the smallest catalog model, observe progress/speed, verify completes, card shows "available".
- AC-002-b — Live-app QC step: delete the downloaded model; card returns to "downloadable" state; disk file is gone.
- AC-002-c — Live-app QC step: start a download, click cancel, confirm the partial file is cleaned up.
- AC-002-d — TypeScript type: ModelCard (or a new `LlmModelCard`) accepts the same status/progress/speed prop shape used by Whisper cards.

## R-003 — On-disk layout under `llm/` subdirectory

Downloaded GGUFs land in the Toaster app-data dir under `llm/<model_id>.gguf`, distinct from `models/` used by Whisper.

- AC-003-a — cargo test `llm_download_writes_under_llm_subdir` verifies output path composition.
- AC-003-b — Live-app QC step: after one successful download, confirm the file exists at `<app-data>/llm/<id>.gguf`.

## R-004 — Disk-space pre-check

Before starting a download, warn the user if free disk < 2x the model size.

- AC-004-a — cargo test `llm_download_preflight_errors_when_disk_tight` simulates low free space and asserts an early-out error.
- AC-004-b — Live-app QC step: on a drive with < 2x free, download attempt surfaces a user-readable warning in the UI.

## R-005 — LlmManager lifecycle

`src-tauri/src/managers/llm/mod.rs` owns the manager. Lazy-loads on first cleanup call. Unloads after inactivity timeout (reuse existing `model_unload_timeout` OR new `llm_unload_timeout` — blueprint decides).

- AC-005-a — cargo test `llm_manager_lazy_loads_on_first_complete` passes (uses a 1-token dummy model or a mocked inference backend per blueprint).
- AC-005-b — cargo test `llm_manager_unloads_after_timeout` passes.
- AC-005-c — cargo test `llm_manager_errors_cleanly_when_model_missing` asserts that an unknown or corrupt path returns `Err(_)` rather than panicking.

## R-006 — Insufficient-RAM handling

Loader fails cleanly when the model's `recommended_ram_gb` exceeds available system RAM.

- AC-006-a — cargo test `llm_manager_errors_when_ram_insufficient` (mock low-RAM path via a test-only hook).
- AC-006-b — Live-app QC step (optional if hardware allows): select the largest catalog model on a constrained machine, confirm UI shows an actionable error rather than crashing.

## R-007 — Cleanup dispatch gains `local_gguf` branch

`managers/cleanup/llm_dispatch.rs` adds a third provider branch that calls `LlmManager::complete(system_prompt, user_prompt, schema)` and returns the same `AttemptOutcome` shape as the HTTP path.

- AC-007-a — cargo test `cleanup_dispatch_routes_local_gguf_through_llm_manager` passes (uses an injected mock LlmManager).
- AC-007-b — All 11 existing cleanup tests (`cargo test -p toaster --lib cleanup`) continue to pass without modification.
- AC-007-c — cargo test `cleanup_prompt_contract_passes_under_local_path` runs the existing contract assertions through the local dispatch using a mock.

## R-008 — Settings surface

`local_llm_model_id: Option<String>` added. Provider selector on Post-Process offers "Local (in-process)"; when selected, HTTP fields (base URL, API key, model name) are hidden and not persisted.

- AC-008-a — `cargo check -p toaster --lib` green after settings addition.
- AC-008-b — Live-app QC step: switch provider to Local; confirm HTTP fields disappear and the local-models section becomes the primary config.

## R-009 — Airplane-mode / offline invariant

With a model downloaded, cleanup runs successfully with the network disconnected. No hosted inference endpoint is ever contacted.

- AC-009-a — Live-app QC step: download a model, disable network / enable airplane mode, run cleanup on a fixture file, confirm success.
- AC-009-b — `dep-hygiene` review: no crate in the transitive graph contacts a default hosted-inference endpoint. Spot-check via `cargo tree -p toaster -e normal` search for openai/anthropic/etc substrings.

## R-010 — Precision + boundary evals green on local path

`transcript-precision-eval` skill run passes under local dispatch; the cleanup contract tests pass.

- AC-010-a — skill: `transcript-precision-eval` run under the local path on the existing fixture set reports green.
- AC-010-b — cargo test `rejects_output_when_protected_tokens_are_dropped` passes with the local path (existing test; runs under both dispatches via a parameterization or an explicit twin test).

## R-011 — Dep-hygiene

New crate added to `Cargo.toml`. Justified in blueprint. `cargo machete` clean. Unused crates not introduced.

- AC-011-a — skill: `dep-hygiene` run reports clean.

## R-012 — HTTP path preserved as fallback

The OpenAI-compatible HTTP provider path remains available and unchanged for users already running Ollama / LM Studio / llama.cpp server.

- AC-012-a — Live-app QC step: switch provider back to HTTP, configure a running local Ollama, run cleanup; confirm success.

## R-013 — i18n + docs

New keys for the local-models section, provider-selector labels, disk-space warnings across 20 locales. `docs/post-processing.md` updated to reflect the in-app path as the default.

- AC-013-a — `bun run scripts/check-translations.ts` exit 0.
- AC-013-b — `docs/post-processing.md` mentions the in-app catalog as the default path; Ollama / LM Studio / llama.cpp listed as advanced fallback.

## R-014 — Static gates

- AC-014-a — `npm run lint` exit 0.
- AC-014-b — `npx tsc --noEmit` exit 0.
- AC-014-c — `cargo check -p toaster --lib` exit 0.
