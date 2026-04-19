# Blueprint: local-llm-model-catalog

## Crate choice

**Decision: `llama-cpp-2`** (thin wrapper over upstream llama.cpp via cmake+ninja).

Rationale:
- Widest GGUF coverage of the three candidates (qwen2.5, llama-3.x, mistral, phi all supported).
- Same cmake+ninja build pattern as the existing `whisper-rs-sys` dep — the Windows MSVC hardening already in tree (`scripts/setup-env.ps1`) extends naturally. No new toolchain dimension.
- Tracks upstream llama.cpp closely; bug/perf fixes upstream reach us quickly.

Considered and rejected:
- `candle-transformers` — pure Rust, easy build story, but narrower GGUF coverage and slower to adopt new quantization formats. Future re-evaluation as candle matures.
- `mistralrs` — higher-level, opinionated. Good for a v2 when we add streaming / function-calling. Too much surface area for v1.

### Risks the choice introduces

| Risk | Mitigation |
|------|-----------|
| Cold build time grows by 2-5 minutes (llama.cpp build + Vulkan probe) | Feature-flag GPU backend; default to CPU-only to keep cold-build overhead bounded. Document new range in `AGENTS.md` "Cargo runtime expectations" if the delta exceeds the current 2-10 minute window. |
| Link-order conflict with `whisper-rs-sys` (both pull ggml) | Use the variant of `llama-cpp-2` that links ggml statically per-crate, OR pin both to a compatible ggml submodule. Audit at the first cargo-check after crate add. |
| Windows MSVC + ninja quirks around CUDA/Vulkan flags | Reuse `scripts/setup-env.ps1`; disable CUDA feature flag on v1 (CPU-only); document with a HOWTO in `docs/build.md`. |
| License — llama-cpp-2 is MIT, llama.cpp is MIT, individual model GGUFs are HuggingFace-specific | Ship catalog URLs pointing at HuggingFace; user accepts the model license on first download. License text surfaced in the download confirmation dialog (out of v1 scope? confirm in risk register). |

## Asset-event generalization

**Decision: generalize `ModelDownloadEvent` into a `DownloadableAssetEvent` enum**, reused by both Whisper and LLM managers.

The current Whisper event shape (`started`, `progress { downloaded, total }`, `complete`, `error`) already fits LLM downloads. Keep the event payload schema identical; disambiguate by `asset_kind: "whisper" | "llm"`.

Frontend: `useModelStore` stays Whisper-specific; add a parallel `useLlmModelStore` with the same reducer shape. Do not merge stores in v1 — the UX surfaces are separate pages.

## Dispatch integration

`managers/cleanup/llm_dispatch.rs::AttemptInputs` gains a new `backend: DispatchBackend` enum member so the dispatcher picks the branch:

```rust
enum DispatchBackend {
    Http(HttpProviderConfig),
    LocalGguf(Arc<LlmManager>),
}
```

The existing HTTP branch is untouched. The new local branch calls `llm_manager.complete(system_prompt, user_prompt, schema).await`. Return shape (`AttemptOutcome`) is identical, so all downstream contract assertions (`prompts.rs` tests, SSOT filler/protected-token tests) continue to pass.

Selection rule: if `settings.post_process_provider_id == "local" && settings.local_llm_model_id.is_some()`, use `LocalGguf`; else `Http`.

## Unload policy

**Decision: reuse the existing `model_unload_timeout` setting** for both Whisper and LLM unload. Two-keys-for-the-same-concept violates SSOT. If a user needs different timeouts per manager, we add a second key in v2 on evidence of demand.

`LlmManager` subscribes to the same "idle N seconds" signal as the Whisper model manager. Implementation detail: the subscription lives in `managers/llm/mod.rs` and calls into the shared timer utility (audit existing `managers/model/mod.rs` for the helper).

## Catalog schema + storage

Entries: see REQUEST section 6. Ship in `managers/llm/catalog.rs` as a `pub const CATALOG: &[LlmCatalogEntry]`. URLs point to HuggingFace revisions pinned by sha256; a URL drift would be caught by verify step.

Disk layout: `<app-data>/llm/<id>.gguf`. Separate dir from Whisper `models/` so the user can nuke one without touching the other.

Disk-space preflight: before `download_start`, stat the drive hosting the app-data dir; require `free >= 2 * entry.size_bytes`. Surface as a user-readable error in the UI.

## RAM budgeting

Before `LlmManager::load(entry)`, compare `entry.recommended_ram_gb * 1024 * 1024 * 1024` against `sysinfo::System::total_memory()`. On insufficient RAM, return `LlmError::InsufficientRam`. Frontend renders the manager error via the same Alert component used elsewhere.

GPU backend detection is deferred to v2.

## Post-Process UI integration

Provider selector options (existing):
- OpenAI-compatible (local) [HTTP Ollama / LM Studio / llama.cpp server]
- Custom

Add new option at the top of the list:
- **Local (in-process)** [Recommended]

When selected, render `LlmModelCatalog` (new component under `src/components/settings/post-processing/local-models/`) in place of the HTTP fields. Existing fields (Base URL, API key, model) disappear but their stored values are preserved (so switching back restores config).

ModelCard: generalize via a `source: "whisper" | "llm"` prop, or fork into `LlmModelCard` if generalization requires >50 LOC of branching. Decision deferred to the component task; tracked as a risk.

## Precision / boundary evals under local path

Plan: run `transcript-precision-eval` on the existing fixture set with `post_process_provider_id=local` and the recommended default model. If the 1b-default fails precision gates, escalate the default to 3b-q4 and document in journal. 0.5b stays in the catalog as a "minimum viable" option marked with a UX warning.

Cleanup contract tests: parameterize the existing cleanup tests by dispatch backend so both `Http (mock)` and `LocalGguf (mock)` paths run. Mock `LlmManager::complete` returns a fixture-provided JSON response mirroring real output.

## Risk register (summary)

| Risk | Likelihood | Impact | Response |
|------|-----------|--------|---------|
| Build-time blowup breaks CI | medium | high | Gate llama-cpp-2 behind a feature flag in Cargo.toml; enable in release/dev only; skip in hooks-only CI runs. |
| 1b default too weak for cleanup contract | medium | medium | Promote to 3b default; 0.5b stays in catalog as minimum-viable. |
| HuggingFace URL drift / rate limits | low | medium | Project-controlled mirror as a v2 fallback; v1 trusts HF. |
| GGUF license text surfacing | low (legal nit) | medium | Add license acknowledgement dialog on first download (v1.1). |
| Simultaneous Whisper + LLM load OOMs small machines | medium | medium | Stagger unload + load: LLM unloads Whisper when it loads, and vice versa. |
| Cargo build targets diverge (cmake vs ninja vs pure-Rust) between whisper-rs-sys and llama-cpp-2 | low | high | Pin ggml submodule; verify with cold build + test. |

## Implementation order

1. Crate add: `llama-cpp-2` to `src-tauri/Cargo.toml`; smoke `cargo check -p toaster --lib`.
2. `managers/llm/catalog.rs`: catalog entries + cargo tests for AC-001-a/b.
3. `managers/llm/download.rs`: reuse or port download/verify/extract logic; cargo tests for AC-003-a, AC-004-a.
4. `managers/llm/inference.rs`: LlmManager with lazy load + unload; cargo tests for AC-005-a/b/c, AC-006-a.
5. Tauri commands: mirror Whisper command surface (download/cancel/delete/list).
6. `settings/defaults.rs`: add `local_llm_model_id`; bindings regen.
7. `managers/cleanup/llm_dispatch.rs`: add `DispatchBackend::LocalGguf`; parameterize existing tests; cargo tests for AC-007-a/b/c.
8. `DownloadableAssetEvent` generalization (or parallel event enum if generalization risks regression).
9. Frontend: `LlmModelCatalog` component + Post-Process integration; ModelCard generalization decision.
10. i18n: 20 locale updates.
11. docs/post-processing.md: reflect in-app-default path.
12. `dep-hygiene` skill + static gates.
13. Precision eval under local path.
14. Live-app QC including airplane-mode + provider switch.

## Open questions (to resolve during execution, not blocking planning)

- Should the first-download flow surface a license-acknowledgement dialog (HF model licenses)? Leaning yes for v1.1.
- Mirror hosting — project-owned bucket vs HF direct. Trust HF for v1; mirror when traffic warrants.
- Should Whisper unload when LLM loads (and vice versa)? Yes under 8 GB RAM; gate by `total_memory()`.
