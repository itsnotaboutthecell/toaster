# Feature request: local-llm-model-catalog

## 1. Problem & Goals

Post-Process tells the user to install a local LLM, but the UX is asymmetric with the Whisper Models page: no catalog, no in-app download, no verify/delete, no disk state. Whisper models get first-class treatment. Cleanup LLM models get a raw HTTP endpoint configurator and a docs link.

Close the gap: ship an in-app catalog of curated small GGUF LLM models, with download / verify / delete / select parity, and run inference in-process against the chosen GGUF. The HTTP OpenAI-compatible provider path stays as an advanced fallback. "Local-only inference" (AGENTS.md) gets reinforced, not eroded — the catalog download is a user-initiated asset fetch, and inference is in-process.

## 2. Desired outcome & acceptance criteria

- Post-Process page gains a local-models section that mirrors the Models page's ModelCard UX (download, progress, verify, select, delete, cancel).
- A new `LlmManager` lazy-loads the selected GGUF and serves cleanup prompts in-process.
- The cleanup dispatch gains a third branch: `provider_kind = local_gguf`. Existing HTTP path untouched and its tests still pass.
- Settings: `local_llm_model_id: Option<String>`; selecting "Local (in-process)" on the provider selector hides HTTP fields.
- Airplane-mode QC: cleanup works offline once a model is downloaded.
- `transcript-precision-eval` + cleanup contract tests green under the local path.

## 3. Scope boundaries

In scope: Rust LlmManager + catalog + download + inference; Tauri command surface; dispatch integration; Post-Process UI; settings; unload timeout; dep-hygiene + i18n; cargo tests; live QC.

Out of scope: removing the HTTP provider path (it stays); GPU acceleration tuning beyond the crate defaults; streaming token output; fine-tuning; multi-model ensembles; model quantization on-device.

## 4. References to existing code

- `src-tauri/src/managers/model/{mod,download,extract,verify}.rs` — Whisper pattern to mirror / generalize
- `src-tauri/src/managers/model/catalog.rs` (if present) — catalog shape
- `src-tauri/src/managers/cleanup/llm_dispatch.rs` — dispatch surface, add third branch
- `src-tauri/src/managers/cleanup/prompts.rs` — unchanged (system + user prompts + schema)
- `src-tauri/src/settings/defaults.rs` — AppSettings shape; add local_llm_model_id
- `src-tauri/src/commands/` — mirror the whisper command surface for LLM
- `src/components/settings/post-processing/PostProcessingSettings.tsx` — grow a local-models section
- `src/components/onboarding/ModelCard.tsx` — reuse / generalize
- `src-tauri/Cargo.toml` — new crate dep
- `docs/post-processing.md` — user-facing documentation (update after shipping)

## 5. Edge cases & constraints

- Catalog downloads can exceed 2 GB per model; disk-space check required before starting.
- On-device RAM insufficient: loader must fail cleanly with a user-readable error; do not crash the app.
- cmake/ninja toolchain on Windows MSVC is already used by `whisper-rs-sys`; a new llama.cpp-based crate piggybacks on the same toolchain but might introduce link-order or symbol-conflict headaches — blueprint surfaces this risk.
- Existing HTTP provider path must keep passing all 11 existing cleanup tests.
- Model catalog URLs should point to HuggingFace (or a project-controlled mirror). Never commit the GGUF files.
- `AGENTS.md` "Local-only inference": the new path MUST be runnable offline. Airplane-mode QC step enforces this.
- `dep-hygiene` skill: the new crate must be justified in blueprint; `cargo machete` stays clean.
- File-size cap 800 lines: split `managers/llm/` into mod + catalog + download + inference + tests.

## 6. Data model

```rust
// settings/defaults.rs additions
local_llm_model_id: Option<String>, // default None; if Some, dispatch uses local_gguf

// managers/llm/catalog.rs
pub struct LlmCatalogEntry {
    pub id: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub quantization: String,          // "Q4_K_M", etc.
    pub download_url: String,
    pub context_length: u32,
    pub recommended_ram_gb: u32,
    pub is_recommended_default: bool,
}
```

Provider-selector change: keep `post_process_provider_id` as a string, add a reserved sentinel value (e.g. `"local"`) that the dispatch switch recognizes.

## Q&A (resolved)

- UX scope: full parity (user chose Option A).
- Hosted inference dependency: never introduced. Downloads are user-initiated static asset fetches.
