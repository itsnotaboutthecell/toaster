// LLM-dispatch helper for transcript cleanup.
//
// Extracted from `cleanup/mod.rs` so the manager file only owns orchestration.
// Provides a single `try_llm_attempt` entry point covering both the
// structured-output (with-schema) attempt and the legacy (without-schema)
// attempt, including the local-openai-provider reasoning special case and
// the legacy retry loop.
//
// Feature B (`local-llm-model-catalog`) added the `DispatchBackend` seam:
// the HTTP path is untouched, and a new `LocalGguf` variant routes through
// `managers::llm::LlmManager` so cleanup can run in-process against a
// downloaded GGUF model.

use log::{debug, error, warn};
use std::sync::Arc;
use std::time::Duration;

use super::prompts::{
    build_cleanup_contract_schema, build_cleanup_contract_system_prompt,
    build_cleanup_legacy_prompt,
};
use super::{
    validate_cleanup_candidate, CleanupContractResponse, CLEANUP_TRANSCRIPTION_FIELD,
    TRANSCRIPTION_FIELD,
};
use crate::llm_client::ReasoningConfig;
use crate::managers::llm::{CompletionRequest, LlmManager};
use crate::settings::PostProcessProvider;

/// Outcome of one LLM-dispatch attempt.
pub(super) enum AttemptOutcome {
    /// Attempt produced a validated cleanup output. Caller should return this.
    Success(String),
    /// Attempt did not yield a usable result. Caller should fall through to the
    /// next path (e.g. structured -> legacy) or preserve the original transcription.
    Fallback,
}

/// Dispatch backend selector. `Http` is the legacy path (unchanged);
/// `LocalGguf` routes through the in-process `LlmManager`. Selection rule
/// lives in `select_dispatch_backend` below.
pub(crate) enum DispatchBackend {
    Http,
    LocalGguf {
        manager: Arc<LlmManager>,
        model_id: String,
    },
}

impl DispatchBackend {
    pub fn is_local_gguf(&self) -> bool {
        matches!(self, DispatchBackend::LocalGguf { .. })
    }
}

/// Apply the selection rule documented in BLUEPRINT "Dispatch integration":
/// if `post_process_provider_id == LOCAL_GGUF_PROVIDER_ID` and a local
/// model is selected, route through `LocalGguf`. Otherwise the caller
/// uses the HTTP path.
pub(crate) fn select_dispatch_backend(
    provider: &PostProcessProvider,
    local_llm_model_id: Option<&str>,
    llm_manager: Option<Arc<LlmManager>>,
) -> DispatchBackend {
    use crate::settings::LOCAL_GGUF_PROVIDER_ID;
    if provider.id == LOCAL_GGUF_PROVIDER_ID {
        if let (Some(model_id), Some(manager)) = (local_llm_model_id, llm_manager) {
            if !model_id.is_empty() {
                return DispatchBackend::LocalGguf {
                    manager,
                    model_id: model_id.to_string(),
                };
            }
        }
    }
    DispatchBackend::Http
}

/// Parameters shared by both attempt shapes. Bundled into one struct so the
/// `try_llm_attempt` signature stays manageable.
pub(super) struct AttemptInputs<'a> {
    pub provider: &'a PostProcessProvider,
    pub api_key: String,
    pub model: &'a str,
    pub transcription: &'a str,
    pub prompt: &'a str,
    pub protected_tokens_for_prompt: &'a [String],
    pub filler_words_for_prompt: &'a [String],
    pub local_openai_provider: bool,
    pub reasoning_effort: Option<String>,
    pub reasoning: Option<ReasoningConfig>,
}

/// Run one cleanup-LLM attempt. `use_schema = true` issues a structured-output
/// request and falls through to the legacy path on any failure. `use_schema =
/// false` issues the legacy prompt and, for the local OpenAI-compatible
/// provider, retries once on transient errors or validation failures.
pub(super) async fn try_llm_attempt(
    inputs: &AttemptInputs<'_>,
    use_schema: bool,
) -> AttemptOutcome {
    if use_schema {
        try_structured_attempt(inputs).await
    } else {
        try_legacy_attempt(inputs).await
    }
}

async fn try_structured_attempt(inputs: &AttemptInputs<'_>) -> AttemptOutcome {
    let provider = inputs.provider;
    debug!("Using structured outputs for provider '{}'", provider.id);

    let system_prompt = build_cleanup_contract_system_prompt(
        inputs.prompt,
        inputs.protected_tokens_for_prompt,
        inputs.filler_words_for_prompt,
    );
    let user_content = inputs.transcription.to_string();
    let json_schema = build_cleanup_contract_schema();

    match crate::llm_client::send_chat_completion_with_schema(
        provider,
        inputs.api_key.clone(),
        inputs.model,
        user_content,
        Some(system_prompt),
        Some(json_schema),
        inputs.reasoning_effort.clone(),
        inputs.reasoning.clone(),
    )
    .await
    {
        Ok(Some(content)) => match serde_json::from_str::<CleanupContractResponse>(&content) {
            Ok(contract_response) => match validate_cleanup_candidate(
                inputs.transcription,
                &contract_response.cleaned_transcription,
                Some(&contract_response),
            ) {
                Ok(validated) => {
                    debug!(
                        "Structured cleanup post-processing succeeded for provider '{}'. Output length: {} chars",
                        provider.id,
                        validated.len()
                    );
                    AttemptOutcome::Success(validated)
                }
                Err(validation_error) => {
                    warn!(
                        "Structured cleanup output rejected for provider '{}': {}. Falling back to legacy mode.",
                        provider.id, validation_error
                    );
                    AttemptOutcome::Fallback
                }
            },
            Err(contract_parse_error) => {
                warn!(
                    "Structured cleanup contract parse failed for provider '{}': {}. Attempting compatibility fallback.",
                    provider.id, contract_parse_error
                );

                let fallback_candidate = serde_json::from_str::<serde_json::Value>(&content)
                    .ok()
                    .and_then(|json| {
                        json.get(CLEANUP_TRANSCRIPTION_FIELD)
                            .and_then(|value| value.as_str())
                            .map(ToString::to_string)
                            .or_else(|| {
                                json.get(TRANSCRIPTION_FIELD)
                                    .and_then(|value| value.as_str())
                                    .map(ToString::to_string)
                            })
                    });

                if let Some(candidate) = fallback_candidate {
                    match validate_cleanup_candidate(inputs.transcription, &candidate, None) {
                        Ok(validated) => {
                            debug!(
                                "Structured compatibility fallback succeeded for provider '{}'. Output length: {} chars",
                                provider.id,
                                validated.len()
                            );
                            AttemptOutcome::Success(validated)
                        }
                        Err(validation_error) => {
                            warn!(
                                "Structured compatibility fallback rejected for provider '{}': {}. Falling back to legacy mode.",
                                provider.id, validation_error
                            );
                            AttemptOutcome::Fallback
                        }
                    }
                } else {
                    warn!(
                        "Structured response from provider '{}' did not contain '{}' or '{}'; falling back to legacy mode.",
                        provider.id, CLEANUP_TRANSCRIPTION_FIELD, TRANSCRIPTION_FIELD
                    );
                    AttemptOutcome::Fallback
                }
            }
        },
        Ok(None) => {
            warn!(
                "Structured output API returned no content for provider '{}'; falling back to legacy mode.",
                provider.id
            );
            AttemptOutcome::Fallback
        }
        Err(e) => {
            warn!(
                "Structured output call failed for provider '{}': {}. Falling back to legacy mode.",
                provider.id, e
            );
            AttemptOutcome::Fallback
        }
    }
}

async fn try_legacy_attempt(inputs: &AttemptInputs<'_>) -> AttemptOutcome {
    let provider = inputs.provider;
    let processed_prompt = build_cleanup_legacy_prompt(
        inputs.prompt,
        inputs.transcription,
        inputs.protected_tokens_for_prompt,
        inputs.filler_words_for_prompt,
    );
    debug!("Processed prompt length: {} chars", processed_prompt.len());

    let max_attempts = if inputs.local_openai_provider { 2 } else { 1 };
    for attempt in 1..=max_attempts {
        match crate::llm_client::send_chat_completion(
            provider,
            inputs.api_key.clone(),
            inputs.model,
            processed_prompt.clone(),
            inputs.reasoning_effort.clone(),
            inputs.reasoning.clone(),
        )
        .await
        {
            Ok(Some(content)) => {
                match validate_cleanup_candidate(inputs.transcription, &content, None) {
                    Ok(validated) => {
                        debug!(
                            "LLM post-processing succeeded for provider '{}'. Output length: {} chars",
                            provider.id,
                            validated.len()
                        );
                        return AttemptOutcome::Success(validated);
                    }
                    Err(validation_error) => {
                        if inputs.local_openai_provider && attempt < max_attempts {
                            warn!(
                                "Legacy cleanup output rejected for local provider '{}' (attempt {}): {}. Retrying once.",
                                provider.id, attempt, validation_error
                            );
                            tokio::time::sleep(Duration::from_millis(250)).await;
                            continue;
                        }

                        warn!(
                            "Legacy cleanup output rejected for provider '{}': {}. Preserving original transcription.",
                            provider.id, validation_error
                        );
                        return AttemptOutcome::Fallback;
                    }
                }
            }
            Ok(None) => {
                error!(
                    "LLM post-processing returned no content for provider '{}'; preserving original transcription",
                    provider.id
                );
                return AttemptOutcome::Fallback;
            }
            Err(e) => {
                if inputs.local_openai_provider && attempt < max_attempts {
                    warn!(
                        "Transient local LLM error for provider '{}' (attempt {}): {}. Retrying once.",
                        provider.id, attempt, e
                    );
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    continue;
                }

                error!(
                    "LLM post-processing failed for provider '{}': {}. Falling back to original transcription.",
                    provider.id, e
                );
                return AttemptOutcome::Fallback;
            }
        }
    }

    AttemptOutcome::Fallback
}


/// Run one cleanup attempt against the in-process local-GGUF backend.
/// Uses the same prompts as the HTTP path so the cleanup contract is
/// identical across backends; only the transport differs.
///
/// `use_schema = true` issues a structured-output request (system prompt +
/// schema); the local backend returns a raw assistant string which the
/// dispatcher parses through the same `CleanupContractResponse` path as
/// the HTTP branch. On any error or validation failure this function
/// returns `AttemptOutcome::Fallback`, matching the HTTP contract.
pub(super) async fn try_llm_attempt_local_gguf(
    inputs: &AttemptInputs<'_>,
    manager: &LlmManager,
    model_id: &str,
    use_schema: bool,
) -> AttemptOutcome {
    let (system_prompt, user_prompt, json_schema) = if use_schema {
        let sys = build_cleanup_contract_system_prompt(
            inputs.prompt,
            inputs.protected_tokens_for_prompt,
            inputs.filler_words_for_prompt,
        );
        let schema = build_cleanup_contract_schema().to_string();
        (sys, inputs.transcription.to_string(), Some(schema))
    } else {
        let legacy = build_cleanup_legacy_prompt(
            inputs.prompt,
            inputs.transcription,
            inputs.protected_tokens_for_prompt,
            inputs.filler_words_for_prompt,
        );
        (String::new(), legacy, None)
    };

    let req = CompletionRequest {
        system_prompt,
        user_prompt,
        json_schema,
    };

    let response = match manager.complete(model_id, req).await {
        Ok(r) => r,
        Err(e) => {
            error!(
                "Local GGUF LLM call failed for model '{}': {}. Preserving original transcription.",
                model_id, e
            );
            return AttemptOutcome::Fallback;
        }
    };

    let content = response.content;
    if use_schema {
        match serde_json::from_str::<CleanupContractResponse>(&content) {
            Ok(contract_response) => match validate_cleanup_candidate(
                inputs.transcription,
                &contract_response.cleaned_transcription,
                Some(&contract_response),
            ) {
                Ok(validated) => {
                    debug!(
                        "Local GGUF structured cleanup succeeded for model '{}'. Output length: {} chars",
                        model_id,
                        validated.len()
                    );
                    AttemptOutcome::Success(validated)
                }
                Err(validation_error) => {
                    warn!(
                        "Local GGUF structured output rejected for model '{}': {}. Falling back.",
                        model_id, validation_error
                    );
                    AttemptOutcome::Fallback
                }
            },
            Err(_) => {
                // Try the same compatibility-fallback shape as the HTTP path.
                let candidate = serde_json::from_str::<serde_json::Value>(&content)
                    .ok()
                    .and_then(|json| {
                        json.get(CLEANUP_TRANSCRIPTION_FIELD)
                            .and_then(|v| v.as_str())
                            .map(ToString::to_string)
                            .or_else(|| {
                                json.get(TRANSCRIPTION_FIELD)
                                    .and_then(|v| v.as_str())
                                    .map(ToString::to_string)
                            })
                    });
                match candidate {
                    Some(c) => {
                        match validate_cleanup_candidate(inputs.transcription, &c, None) {
                            Ok(v) => AttemptOutcome::Success(v),
                            Err(e) => {
                                warn!(
                                    "Local GGUF compat fallback rejected for model '{}': {}",
                                    model_id, e
                                );
                                AttemptOutcome::Fallback
                            }
                        }
                    }
                    None => {
                        // Not JSON — treat as legacy raw output.
                        match validate_cleanup_candidate(
                            inputs.transcription,
                            content.trim(),
                            None,
                        ) {
                            Ok(v) => AttemptOutcome::Success(v),
                            Err(_) => AttemptOutcome::Fallback,
                        }
                    }
                }
            }
        }
    } else {
        match validate_cleanup_candidate(inputs.transcription, &content, None) {
            Ok(v) => AttemptOutcome::Success(v),
            Err(e) => {
                warn!(
                    "Local GGUF legacy output rejected for model '{}': {}",
                    model_id, e
                );
                AttemptOutcome::Fallback
            }
        }
    }
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::managers::llm::{inference::MockBackend, LlmManager};
    use crate::settings::{LOCAL_GGUF_PROVIDER_ID, OLLAMA_PROVIDER_ID, PostProcessProvider};
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    fn test_provider(id: &str) -> PostProcessProvider {
        PostProcessProvider {
            id: id.to_string(),
            label: id.to_string(),
            base_url: "".into(),
            allow_base_url_edit: false,
            models_endpoint: None,
            supports_structured_output: true,
            local_only: true,
            requires_api_key: false,
        }
    }

    fn build_manager_with_mock(
        model_id: &str,
        backend: Arc<Mutex<MockBackend>>,
    ) -> (TempDir, Arc<LlmManager>) {
        use crate::managers::llm::{download::download_tests::FixedFreeSpace, FixedRamProbe, catalog};
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("llm");
        std::fs::create_dir_all(&dir).unwrap();
        // Write a fake gguf for the entry.
        let filename = catalog::find_entry(model_id).unwrap().filename();
        std::fs::write(dir.join(&filename), b"fake").unwrap();
        let mgr = Arc::new(
            LlmManager::with_probes(
                dir,
                Arc::new(FixedRamProbe(64 * 1024 * 1024 * 1024)),
                Arc::new(FixedFreeSpace(1_000_000_000_000)),
            )
            .unwrap(),
        );
        mgr.install_backend_for_tests(model_id, backend as Arc<Mutex<dyn crate::managers::llm::LlmBackend>>);
        (tmp, mgr)
    }

    #[test]
    fn select_dispatch_backend_routes_to_local_when_local_provider_and_model_set() {
        let provider = test_provider(LOCAL_GGUF_PROVIDER_ID);
        let (_tmp, mgr) = build_manager_with_mock(
            "llama-3.2-1b-instruct-q4",
            Arc::new(Mutex::new(MockBackend::with_response("x"))),
        );
        let backend =
            select_dispatch_backend(&provider, Some("llama-3.2-1b-instruct-q4"), Some(mgr));
        assert!(backend.is_local_gguf());
    }

    #[test]
    fn select_dispatch_backend_routes_to_http_when_local_model_unset() {
        let provider = test_provider(LOCAL_GGUF_PROVIDER_ID);
        let backend = select_dispatch_backend(&provider, None, None);
        assert!(!backend.is_local_gguf());
    }

    #[test]
    fn select_dispatch_backend_routes_to_http_for_non_local_provider() {
        let provider = test_provider(OLLAMA_PROVIDER_ID);
        let (_tmp, mgr) = build_manager_with_mock(
            "llama-3.2-1b-instruct-q4",
            Arc::new(Mutex::new(MockBackend::with_response("x"))),
        );
        let backend =
            select_dispatch_backend(&provider, Some("llama-3.2-1b-instruct-q4"), Some(mgr));
        assert!(!backend.is_local_gguf());
    }

    /// AC-007-a: dispatcher routes local-gguf through `LlmManager`, and
    /// the returned content survives cleanup validation.
    #[tokio::test]
    async fn cleanup_dispatch_routes_local_gguf_through_llm_manager() {
        let original = "hello world";
        let mock = Arc::new(Mutex::new(MockBackend::with_response(original)));
        let (_tmp, mgr) =
            build_manager_with_mock("llama-3.2-1b-instruct-q4", mock.clone());

        let provider = test_provider(LOCAL_GGUF_PROVIDER_ID);
        let inputs = AttemptInputs {
            provider: &provider,
            api_key: String::new(),
            model: "llama-3.2-1b-instruct-q4",
            transcription: original,
            prompt: "",
            protected_tokens_for_prompt: &[],
            filler_words_for_prompt: &[],
            local_openai_provider: true,
            reasoning_effort: None,
            reasoning: None,
        };

        let outcome =
            try_llm_attempt_local_gguf(&inputs, &mgr, "llama-3.2-1b-instruct-q4", false).await;
        match outcome {
            AttemptOutcome::Success(content) => assert_eq!(content, original),
            AttemptOutcome::Fallback => panic!("expected success outcome"),
        }
        assert_eq!(mock.lock().unwrap().call_count(), 1);
    }

    /// AC-007-c: the cleanup prompt contract assertions pass under the
    /// local dispatch. We verify that protected tokens drop-out is
    /// rejected end-to-end via the local path.
    #[tokio::test]
    async fn cleanup_prompt_contract_passes_under_local_path() {
        let original = "Budget is $5 and ref A1B2 remains.";
        // Mock returns destructive rewrite — contract must reject it.
        let bad_response = "Budget is five dollars and ref AB remains.";
        let mock = Arc::new(Mutex::new(MockBackend::with_response(bad_response)));
        let (_tmp, mgr) =
            build_manager_with_mock("llama-3.2-1b-instruct-q4", mock.clone());
        let provider = test_provider(LOCAL_GGUF_PROVIDER_ID);
        let inputs = AttemptInputs {
            provider: &provider,
            api_key: String::new(),
            model: "llama-3.2-1b-instruct-q4",
            transcription: original,
            prompt: "",
            protected_tokens_for_prompt: &["$5".into(), "A1B2".into()],
            filler_words_for_prompt: &[],
            local_openai_provider: true,
            reasoning_effort: None,
            reasoning: None,
        };
        let outcome =
            try_llm_attempt_local_gguf(&inputs, &mgr, "llama-3.2-1b-instruct-q4", false).await;
        // Contract violation -> Fallback.
        assert!(matches!(outcome, AttemptOutcome::Fallback));
    }
}
