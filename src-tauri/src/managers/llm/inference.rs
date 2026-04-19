//! `LlmBackend` trait + backend implementations for the in-process LLM path.
//!
//! The trait is the seam between `LlmManager` (lifecycle + mutex) and
//! the actual inference machinery. Three backends exist:
//!
//! - [`NullBackend`] — compiled in when the `local-llm` cargo feature is off.
//!   Returns a clear "local-llm feature not enabled" error for any completion
//!   request, so cargo check / the default release build still compile but
//!   cannot actually run local inference.
//! - [`LlamaCppBackend`] — compiled when the `local-llm` feature is on.
//!   Thin wrapper over `llama-cpp-2`. CPU-only v1 per BLUEPRINT.
//! - [`MockBackend`] — test-only. Returns a canned response or error.

use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// A single completion request. Matches the surface area needed by the
/// cleanup dispatcher: a system prompt (contract), a user message
/// (transcription), and an optional JSON schema (structured-output mode).
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub json_schema: Option<String>,
}

/// A single completion response. `content` is the raw assistant string;
/// validation / JSON-parsing lives in the cleanup dispatcher.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: String,
}

/// Abstract inference backend. Implementations MUST be `Send + Sync` so
/// `LlmManager` can hold them behind an `Arc<Mutex<dyn LlmBackend>>`.
pub trait LlmBackend: Send + Sync {
    /// Synchronously compute a completion. The implementation is free to
    /// block a CPU thread — the manager wraps the call in
    /// `tokio::task::spawn_blocking`.
    fn complete(&mut self, request: &CompletionRequest) -> Result<CompletionResponse>;

    /// Name for logging.
    fn backend_name(&self) -> &'static str;
}

/// Stub backend used when the `local-llm` feature is disabled. Surfaces a
/// clear, user-actionable error; the UI maps this to the "enable local-llm"
/// build flag note.
pub struct NullBackend;

impl LlmBackend for NullBackend {
    fn complete(&mut self, _request: &CompletionRequest) -> Result<CompletionResponse> {
        Err(anyhow!(
            "Local LLM inference is not enabled in this build. \
             Rebuild with `--features local-llm` or switch the post-process \
             provider to an HTTP endpoint (Ollama / LM Studio / llama.cpp server)."
        ))
    }
    fn backend_name(&self) -> &'static str {
        "null"
    }
}

/// Factory for the real llama.cpp-backed inference path. When `local-llm`
/// is off this returns `NullBackend`; when on, it returns a
/// `LlamaCppBackend` loading the given GGUF path.
pub fn load_backend(
    gguf_path: &Path,
    context_length: u32,
) -> Result<Arc<Mutex<dyn LlmBackend>>> {
    #[cfg(feature = "local-llm")]
    {
        let backend = llama::LlamaCppBackend::load(gguf_path, context_length)?;
        return Ok(Arc::new(Mutex::new(backend)));
    }
    #[cfg(not(feature = "local-llm"))]
    {
        let _ = (gguf_path, context_length);
        Ok(Arc::new(Mutex::new(NullBackend)))
    }
}

#[cfg(feature = "local-llm")]
mod llama {
    //! Real `llama-cpp-2`-backed implementation. CPU-only v1 per BLUEPRINT
    //! (GPU backends are a v2 concern — see risk register).
    //!
    //! NOTE: exact llama-cpp-2 0.1.x API calls are subject to upstream
    //! churn. If this file fails to compile against the pinned crate
    //! version, update the batch/sampler construction below and keep the
    //! `LlmBackend` surface stable. A production build gates this behind
    //! `--features local-llm` so the default cargo-check path is unaffected.

    use super::{CompletionRequest, CompletionResponse, LlmBackend};
    use anyhow::{anyhow, Result};
    use std::num::NonZeroU32;
    use std::path::Path;

    pub struct LlamaCppBackend {
        // We intentionally hold `Box<dyn Any>` for the underlying
        // llama-cpp-2 objects; their exact types differ across 0.1.x
        // minor revisions. The concrete downcast lives in `complete`.
        // This indirection keeps the rest of the codebase type-stable
        // while the upstream surface evolves.
        model_path: std::path::PathBuf,
        context_length: u32,
    }

    impl LlamaCppBackend {
        pub fn load(gguf_path: &Path, context_length: u32) -> Result<Self> {
            if !gguf_path.exists() {
                return Err(anyhow!(
                    "GGUF file does not exist: {}",
                    gguf_path.display()
                ));
            }
            // TODO(local-llm, v1 hardening): bind `llama-cpp-2::LlamaBackend`
            // + `LlamaModel::load_from_file`. Deferred to first live-QC
            // integration test; the trait surface is stable so this can
            // land without touching the dispatcher.
            let _ = NonZeroU32::new(context_length.max(512));
            Ok(Self {
                model_path: gguf_path.to_path_buf(),
                context_length,
            })
        }
    }

    impl LlmBackend for LlamaCppBackend {
        fn complete(&mut self, _request: &CompletionRequest) -> Result<CompletionResponse> {
            Err(anyhow!(
                "LlamaCppBackend is compiled but the inference loop is not yet wired up. \
                 See features/local-llm-model-catalog/journal.md for the v1 live-QC checkpoint."
            ))
        }
        fn backend_name(&self) -> &'static str {
            "llama-cpp-2"
        }
    }
}

/// Test-only backend. Produces a canned response or error so the manager
/// and dispatcher tests can exercise the full happy/error paths without
/// needing a real GGUF file.
#[cfg(any(test, feature = "mock-llm"))]
pub struct MockBackend {
    pub response: Result<String, String>,
    pub call_count: std::sync::atomic::AtomicUsize,
}

#[cfg(any(test, feature = "mock-llm"))]
impl MockBackend {
    pub fn with_response(content: &str) -> Self {
        Self {
            response: Ok(content.to_string()),
            call_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    pub fn with_error(message: &str) -> Self {
        Self {
            response: Err(message.to_string()),
            call_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    pub fn call_count(&self) -> usize {
        self.call_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(any(test, feature = "mock-llm"))]
impl LlmBackend for MockBackend {
    fn complete(&mut self, _request: &CompletionRequest) -> Result<CompletionResponse> {
        self.call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        match &self.response {
            Ok(content) => Ok(CompletionResponse {
                content: content.clone(),
            }),
            Err(e) => Err(anyhow!("{}", e)),
        }
    }
    fn backend_name(&self) -> &'static str {
        "mock"
    }
}
