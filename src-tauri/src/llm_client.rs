use crate::settings::PostProcessProvider;
use log::debug;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::net::IpAddr;

/// Errors produced by the LLM client.
///
/// `LocalOnlyViolation` is returned whenever an outbound request would target a
/// host that is not loopback. This enforces the non-negotiable "local-only
/// inference" boundary (see AGENTS.md) at the network edge, as defense in depth
/// behind the settings sanitizer.
#[derive(Debug)]
pub enum LlmClientError {
    LocalOnlyViolation { host: String },
    Other(String),
}

impl fmt::Display for LlmClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmClientError::LocalOnlyViolation { host } => write!(
                f,
                "Local-only policy violation: refusing to contact non-loopback host '{}'. Toaster only talks to localhost/127.0.0.1/::1 for LLM inference.",
                host
            ),
            LlmClientError::Other(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for LlmClientError {}

/// Returns true only when the URL's host is a strict loopback address.
/// Accepted: `localhost` (case-insensitive), any IPv4 in 127.0.0.0/8, and IPv6 `::1`.
/// Rejected: everything else including RFC1918 private ranges and `.local` mDNS.
pub(crate) fn is_local_host(url: &reqwest::Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    // Strip IPv6 brackets that may appear via host_str() variants.
    let stripped = host.trim_start_matches('[').trim_end_matches(']');
    match stripped.parse::<IpAddr>() {
        Ok(ip) => ip.is_loopback(),
        Err(_) => false,
    }
}

/// Enforces the local-only boundary for an already-parsed URL.
fn enforce_local_host(url: &reqwest::Url) -> Result<(), LlmClientError> {
    if is_local_host(url) {
        Ok(())
    } else {
        Err(LlmClientError::LocalOnlyViolation {
            host: url.host_str().unwrap_or("<no-host>").to_string(),
        })
    }
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct JsonSchema {
    name: String,
    strict: bool,
    schema: Value,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    json_schema: JsonSchema,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct ReasoningConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ReasoningConfig>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

/// Build headers for API requests. All supported providers are OpenAI-compatible
/// (Ollama / LM Studio / custom local endpoints), so we use Bearer auth only.
fn build_headers(_provider: &PostProcessProvider, api_key: &str) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();

    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Toaster/1.0 (+https://github.com/itsnotaboutthecell/toaster)"),
    );

    if !api_key.is_empty() {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|e| format!("Invalid authorization header value: {}", e))?,
        );
    }

    Ok(headers)
}

/// Create an HTTP client with provider-specific headers
fn create_client(provider: &PostProcessProvider, api_key: &str) -> Result<reqwest::Client, String> {
    let headers = build_headers(provider, api_key)?;
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

fn build_api_url(base_url: &str, endpoint: &str) -> Result<String, String> {
    let trimmed_base = base_url.trim();
    if trimmed_base.is_empty() {
        return Err("Provider base URL is empty".to_string());
    }

    let mut normalized_base = trimmed_base.to_string();
    if !normalized_base.ends_with('/') {
        normalized_base.push('/');
    }

    let endpoint_path = endpoint.trim_start_matches('/');
    reqwest::Url::parse(&normalized_base)
        .map_err(|e| format!("Invalid provider base URL '{}': {}", base_url, e))?
        .join(endpoint_path)
        .map(|url| url.to_string())
        .map_err(|e| {
            format!(
                "Failed to build endpoint URL from '{}' and '{}': {}",
                base_url, endpoint, e
            )
        })
}

/// Send a chat completion request to an OpenAI-compatible API
/// Returns Ok(Some(content)) on success, Ok(None) if response has no content,
/// or Err on actual errors (HTTP, parsing, etc.)
pub async fn send_chat_completion(
    provider: &PostProcessProvider,
    api_key: String,
    model: &str,
    prompt: String,
    reasoning_effort: Option<String>,
    reasoning: Option<ReasoningConfig>,
) -> Result<Option<String>, String> {
    send_chat_completion_with_schema(
        provider,
        api_key,
        model,
        prompt,
        None,
        None,
        reasoning_effort,
        reasoning,
    )
    .await
}

/// Send a chat completion request with structured output support
/// When json_schema is provided, uses structured outputs mode
/// system_prompt is used as the system message when provided
/// reasoning_effort sets the OpenAI-style top-level field (e.g., "none", "low", "medium", "high")
/// reasoning sets the OpenRouter-style nested object (effort + exclude)
#[allow(clippy::too_many_arguments)] // Mirrors the OpenAI/OpenRouter chat-completion request shape.
pub async fn send_chat_completion_with_schema(
    provider: &PostProcessProvider,
    api_key: String,
    model: &str,
    user_content: String,
    system_prompt: Option<String>,
    json_schema: Option<Value>,
    reasoning_effort: Option<String>,
    reasoning: Option<ReasoningConfig>,
) -> Result<Option<String>, String> {
    let url = build_api_url(&provider.base_url, "/chat/completions")?;

    let parsed_url = reqwest::Url::parse(&url)
        .map_err(|e| format!("Invalid chat completions URL '{}': {}", url, e))?;
    enforce_local_host(&parsed_url).map_err(|e| e.to_string())?;

    debug!("Sending chat completion request to: {}", url);

    let client = create_client(provider, &api_key)?;

    // Build messages vector
    let mut messages = Vec::new();

    // Add system prompt if provided
    if let Some(system) = system_prompt {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system,
        });
    }

    // Add user message
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: user_content,
    });

    // Build response_format if schema is provided
    let response_format = json_schema.map(|schema| ResponseFormat {
        format_type: "json_schema".to_string(),
        json_schema: JsonSchema {
            name: "transcription_output".to_string(),
            strict: true,
            schema,
        },
    });

    let request_body = ChatCompletionRequest {
        model: model.to_string(),
        messages,
        response_format,
        reasoning_effort,
        reasoning,
    };

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());
        return Err(format!(
            "API request failed with status {}: {}",
            status, error_text
        ));
    }

    let completion: ChatCompletionResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    Ok(completion
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone()))
}

/// Fetch available models from an OpenAI-compatible API
/// Returns a list of model IDs
pub async fn fetch_models(
    provider: &PostProcessProvider,
    api_key: String,
) -> Result<Vec<String>, String> {
    let models_endpoint = provider.models_endpoint.as_deref().unwrap_or("/models");
    let url = build_api_url(&provider.base_url, models_endpoint)?;

    let parsed_url = reqwest::Url::parse(&url)
        .map_err(|e| format!("Invalid models URL '{}': {}", url, e))?;
    enforce_local_host(&parsed_url).map_err(|e| e.to_string())?;

    debug!("Fetching models from: {}", url);

    let client = create_client(provider, &api_key)?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch models: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!(
            "Model list request failed ({}): {}",
            status, error_text
        ));
    }

    let parsed: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let mut models = Vec::new();

    // Handle OpenAI format: { data: [ { id: "..." }, ... ] }
    if let Some(data) = parsed.get("data").and_then(|d| d.as_array()) {
        for entry in data {
            if let Some(id) = entry.get("id").and_then(|i| i.as_str()) {
                models.push(id.to_string());
            } else if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
                models.push(name.to_string());
            }
        }
    }
    // Handle array format: [ "model1", "model2", ... ]
    else if let Some(array) = parsed.as_array() {
        for entry in array {
            if let Some(model) = entry.as_str() {
                models.push(model.to_string());
            }
        }
    }

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider_with(base_url: &str) -> PostProcessProvider {
        PostProcessProvider {
            id: "test".to_string(),
            label: "Test".to_string(),
            base_url: base_url.to_string(),
            allow_base_url_edit: true,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: true,
            local_only: true,
            requires_api_key: false,
        }
    }

    #[test]
    fn is_local_host_accepts_loopback_urls() {
        let cases = [
            "http://localhost:11434/v1/chat/completions",
            "http://LocalHost/v1",
            "http://127.0.0.1:1234/",
            "http://127.5.6.7/x",
            "http://[::1]:8080/",
        ];
        for c in cases {
            let url = reqwest::Url::parse(c).expect("parse");
            assert!(is_local_host(&url), "expected loopback for {}", c);
        }
    }

    #[test]
    fn is_local_host_rejects_non_loopback_urls() {
        let cases = [
            "https://api.openai.com/v1/chat/completions",
            "http://192.168.1.5/",
            "http://10.0.0.1/",
            "http://172.16.0.1/",
            "http://example.com/",
            "http://printer.local/",
            "http://0.0.0.0/",
        ];
        for c in cases {
            let url = reqwest::Url::parse(c).expect("parse");
            assert!(!is_local_host(&url), "expected non-loopback for {}", c);
        }
    }

    #[test]
    fn enforce_local_host_returns_typed_violation() {
        let url = reqwest::Url::parse("https://api.openai.com/v1/chat/completions").unwrap();
        match enforce_local_host(&url) {
            Err(LlmClientError::LocalOnlyViolation { host }) => {
                assert_eq!(host, "api.openai.com");
            }
            other => panic!("expected LocalOnlyViolation, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn send_chat_completion_refuses_non_loopback_without_network() {
        // Point to an unroutable RFC5737 TEST-NET-1 address. If the gate fails,
        // reqwest would attempt to connect and either hang or eventually time
        // out. With the gate in place we must get a LocalOnlyViolation message
        // synchronously before any I/O.
        let provider = provider_with("http://192.0.2.1:12345/v1");
        let result = send_chat_completion(
            &provider,
            String::new(),
            "test-model",
            "hello".to_string(),
            None,
            None,
        )
        .await;

        let err = result.expect_err("expected error for non-loopback URL");
        assert!(
            err.contains("Local-only policy violation"),
            "expected LocalOnlyViolation message, got: {}",
            err
        );
        assert!(
            err.contains("192.0.2.1"),
            "expected violated host in message, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn fetch_models_refuses_non_loopback_without_network() {
        let provider = provider_with("https://api.openai.com/v1");
        let result = crate::llm_client::fetch_models(&provider, String::new()).await;
        let err = result.expect_err("expected error for non-loopback URL");
        assert!(
            err.contains("Local-only policy violation"),
            "expected LocalOnlyViolation message, got: {}",
            err
        );
    }
}
