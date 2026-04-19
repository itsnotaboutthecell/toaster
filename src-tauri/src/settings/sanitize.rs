//! Input sanitization helpers for post-process provider settings.

use super::types::PostProcessProvider;
use std::net::IpAddr;

pub(crate) fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}

/// Returns true only when `base_url` parses as http(s) with a loopback host.
/// Used to enforce the local-only inference boundary for providers that have
/// `local_only: true`. The `apple-intelligence://local` scheme is not http(s)
/// and must be checked separately by callers.
pub(crate) fn base_url_is_loopback(base_url: &str) -> bool {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return false;
    }
    let Ok(parsed) = reqwest::Url::parse(trimmed) else {
        return false;
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return false;
    }
    parsed
        .host_str()
        .map(is_loopback_host)
        .unwrap_or(false)
}

pub fn is_local_post_process_provider(provider: &PostProcessProvider) -> bool {
    provider.local_only
}

pub fn sanitize_local_post_process_base_url(base_url: &str) -> Result<String, String> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err("Base URL cannot be empty".to_string());
    }

    let parsed = reqwest::Url::parse(trimmed)
        .map_err(|e| format!("Invalid base URL '{}': {}", trimmed, e))?;

    match parsed.scheme() {
        "http" | "https" => {}
        _ => {
            return Err("Local provider URL must use http:// or https://".to_string());
        }
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "Base URL must include a host".to_string())?;
    if !is_loopback_host(host) {
        return Err("Local provider URL must point to localhost, 127.0.0.1, or ::1".to_string());
    }

    Ok(trimmed.trim_end_matches('/').to_string())
}

pub fn sanitize_post_process_model(model: &str) -> Result<String, String> {
    let trimmed = model.trim();

    if trimmed.len() > 256 {
        return Err("Model identifier is too long (max 256 characters)".to_string());
    }

    if trimmed.chars().any(|c| c.is_control()) {
        return Err("Model identifier contains invalid control characters".to_string());
    }

    Ok(trimmed.to_string())
}
