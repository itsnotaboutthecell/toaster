// Transcript cleanup post-processing.
//
// Preserved subset of the former `transcription_post_process` module after
// p3-prune-handy-transcription-post-process. Holds the cleanup-contract
// schema (CleanupContractResponse / CleanupInvariantClaims), the validation
// logic that enforces it, the Chinese-variant conversion, and the
// `process_transcription_output` entry point still used by the history-retry
// command. Dictation-era paths (low-confidence span routing,
// local-cleanup-review UI round-trip, Apple Intelligence provider branch,
// streaming-segment confidence heuristics) were removed with the rest of the
// Handy dictation surface.

use crate::settings::{get_settings, AppSettings};
use ferrous_opencc::{config::BuiltinConfig, OpenCC};
use log::{debug, error};
use std::collections::{HashMap, HashSet};
use std::fmt;
use tauri::AppHandle;

pub(super) const TRANSCRIPTION_FIELD: &str = "transcription";
pub(super) const CLEANUP_CONTRACT_VERSION: &str = "transcript_cleanup_contract_v1";
pub(super) const CLEANUP_TRANSCRIPTION_FIELD: &str = "cleaned_transcription";

/// Strip invisible Unicode characters that some LLMs may insert
fn strip_invisible_chars(s: &str) -> String {
    s.replace(['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'], "")
}

/// Build a system prompt from the user's prompt template.
/// Removes `${output}` placeholder since the transcription is sent as the user message.
pub(super) fn build_system_prompt(prompt_template: &str) -> String {
    prompt_template.replace("${output}", "").trim().to_string()
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CleanupInvariantClaims {
    preserve_language: bool,
    no_reorder: bool,
    no_paraphrase: bool,
    protected_tokens_preserved: bool,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CleanupContractResponse {
    contract_version: String,
    cleaned_transcription: String,
    invariants: CleanupInvariantClaims,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ScriptGroup {
    Latin,
    Cyrillic,
    Cjk,
    Arabic,
    Devanagari,
    Hangul,
    Other,
}

impl ScriptGroup {
    fn as_str(self) -> &'static str {
        match self {
            ScriptGroup::Latin => "latin",
            ScriptGroup::Cyrillic => "cyrillic",
            ScriptGroup::Cjk => "cjk",
            ScriptGroup::Arabic => "arabic",
            ScriptGroup::Devanagari => "devanagari",
            ScriptGroup::Hangul => "hangul",
            ScriptGroup::Other => "other",
        }
    }
}

#[derive(Debug)]
struct CleanupValidationError {
    reasons: Vec<String>,
}

impl fmt::Display for CleanupValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reasons.join("; "))
    }
}

fn is_protected_symbol(ch: char) -> bool {
    matches!(
        ch,
        '$' | '€' | '£' | '¥' | '₹' | '₩' | '¢' | '%' | '#' | '@' | '&' | '+' | '=' | '/' | '\\'
    )
}

fn normalize_protected_token(token: &str) -> String {
    token
        .trim_matches(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | '.' | '!' | '?' | ';'
                )
        })
        .to_string()
}

fn extract_protected_tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(normalize_protected_token)
        .filter(|token| {
            !token.is_empty()
                && (token.chars().any(|c| c.is_ascii_digit())
                    || token.chars().any(is_protected_symbol))
        })
        .collect()
}

fn dedupe_tokens(tokens: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for token in tokens {
        if seen.insert(token.clone()) {
            deduped.push(token.clone());
        }
    }
    deduped
}

mod llm_dispatch;
mod prompts;

use llm_dispatch::{try_llm_attempt, AttemptInputs, AttemptOutcome};

fn classify_script(ch: char) -> Option<ScriptGroup> {
    let codepoint = ch as u32;
    if !ch.is_alphabetic() {
        return None;
    }

    if matches!(codepoint, 0x0041..=0x024F | 0x1E00..=0x1EFF) {
        return Some(ScriptGroup::Latin);
    }
    if matches!(codepoint, 0x0400..=0x052F) {
        return Some(ScriptGroup::Cyrillic);
    }
    if matches!(
        codepoint,
        0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    ) {
        return Some(ScriptGroup::Cjk);
    }
    if matches!(codepoint, 0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF) {
        return Some(ScriptGroup::Arabic);
    }
    if matches!(codepoint, 0x0900..=0x097F) {
        return Some(ScriptGroup::Devanagari);
    }
    if matches!(codepoint, 0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF) {
        return Some(ScriptGroup::Hangul);
    }
    Some(ScriptGroup::Other)
}

fn dominant_script(text: &str) -> Option<ScriptGroup> {
    let mut counts: HashMap<ScriptGroup, usize> = HashMap::new();
    for ch in text.chars() {
        if let Some(script) = classify_script(ch) {
            *counts.entry(script).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .and_then(|(script, count)| if count >= 3 { Some(script) } else { None })
}

fn normalize_word_for_match(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

fn normalized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(normalize_word_for_match)
        .filter(|word| !word.is_empty())
        .collect()
}

fn multiset_overlap_ratio(original_words: &[String], candidate_words: &[String]) -> f32 {
    if original_words.is_empty() || candidate_words.is_empty() {
        return 0.0;
    }

    let mut original_counts: HashMap<&str, usize> = HashMap::new();
    let mut candidate_counts: HashMap<&str, usize> = HashMap::new();

    for word in original_words {
        *original_counts.entry(word.as_str()).or_insert(0) += 1;
    }
    for word in candidate_words {
        *candidate_counts.entry(word.as_str()).or_insert(0) += 1;
    }

    let mut shared = 0usize;
    for (word, original_count) in original_counts {
        if let Some(candidate_count) = candidate_counts.get(word) {
            shared += original_count.min(*candidate_count);
        }
    }

    shared as f32 / original_words.len().max(candidate_words.len()) as f32
}

fn ordered_overlap_ratio(original_words: &[String], candidate_words: &[String]) -> f32 {
    if original_words.is_empty() || candidate_words.is_empty() {
        return 0.0;
    }

    let mut positions: HashMap<&str, Vec<usize>> = HashMap::new();
    for (idx, word) in original_words.iter().enumerate() {
        positions.entry(word.as_str()).or_default().push(idx);
    }

    let mut cursors: HashMap<&str, usize> = HashMap::new();
    let mut last_idx: Option<usize> = None;
    let mut matched = 0usize;

    for word in candidate_words {
        let Some(indices) = positions.get(word.as_str()) else {
            continue;
        };

        let cursor = cursors.entry(word.as_str()).or_insert(0);
        while *cursor < indices.len()
            && last_idx
                .map(|last| indices[*cursor] <= last)
                .unwrap_or(false)
        {
            *cursor += 1;
        }

        if *cursor < indices.len() {
            last_idx = Some(indices[*cursor]);
            *cursor += 1;
            matched += 1;
        }
    }

    matched as f32 / original_words.len().max(candidate_words.len()) as f32
}

fn missing_protected_tokens(original: &str, candidate: &str) -> Vec<String> {
    let original_tokens = extract_protected_tokens(original);
    if original_tokens.is_empty() {
        return Vec::new();
    }

    let candidate_tokens = extract_protected_tokens(candidate);
    let mut original_counts: HashMap<String, usize> = HashMap::new();
    let mut candidate_counts: HashMap<String, usize> = HashMap::new();

    for token in original_tokens {
        *original_counts.entry(token).or_insert(0) += 1;
    }
    for token in candidate_tokens {
        *candidate_counts.entry(token).or_insert(0) += 1;
    }

    let mut missing = Vec::new();
    for (token, required_count) in original_counts {
        let candidate_count = candidate_counts.get(&token).copied().unwrap_or(0);
        if candidate_count < required_count {
            missing.push(token);
        }
    }
    missing.sort();
    missing
}

fn normalized_word_set(text: &str) -> HashSet<String> {
    normalized_words(text).into_iter().collect()
}

fn lexical_overlap_ratio(left: &str, right: &str) -> f32 {
    let left_set = normalized_word_set(left);
    let right_set = normalized_word_set(right);
    if left_set.is_empty() || right_set.is_empty() {
        return 0.0;
    }

    let shared = left_set.intersection(&right_set).count();
    shared as f32 / left_set.len().max(right_set.len()) as f32
}

pub(super) fn validate_cleanup_candidate(
    original: &str,
    candidate: &str,
    contract: Option<&CleanupContractResponse>,
) -> Result<String, CleanupValidationError> {
    let cleaned = strip_invisible_chars(candidate).trim().to_string();
    if cleaned.is_empty() {
        return Err(CleanupValidationError {
            reasons: vec!["output is empty".to_string()],
        });
    }

    if cleaned == original.trim() {
        return Ok(cleaned);
    }

    let mut reasons = Vec::new();
    if let Some(contract) = contract {
        if contract.contract_version != CLEANUP_CONTRACT_VERSION {
            reasons.push(format!(
                "unexpected contract_version '{}' (expected '{}')",
                contract.contract_version, CLEANUP_CONTRACT_VERSION
            ));
        }
        if !contract.invariants.preserve_language {
            reasons.push("model marked preserve_language=false".to_string());
        }
        if !contract.invariants.no_reorder {
            reasons.push("model marked no_reorder=false".to_string());
        }
        if !contract.invariants.no_paraphrase {
            reasons.push("model marked no_paraphrase=false".to_string());
        }
        if !contract.invariants.protected_tokens_preserved {
            reasons.push("model marked protected_tokens_preserved=false".to_string());
        }
    }

    let original_word_count = original.split_whitespace().count();
    let candidate_word_count = cleaned.split_whitespace().count();

    if candidate_word_count == 0 {
        reasons.push("output has zero words".to_string());
    }
    if candidate_word_count > original_word_count.saturating_add(12) {
        reasons.push(format!(
            "output gained too many words ({} -> {})",
            original_word_count, candidate_word_count
        ));
    }
    if original_word_count >= 3 && candidate_word_count * 3 < original_word_count {
        reasons.push(format!(
            "output removed too many words ({} -> {})",
            original_word_count, candidate_word_count
        ));
    }

    let original_char_count = original.chars().count();
    let candidate_char_count = cleaned.chars().count();
    if candidate_char_count > original_char_count.saturating_mul(2).saturating_add(24) {
        reasons.push(format!(
            "output length drift too high ({} -> {} chars)",
            original_char_count, candidate_char_count
        ));
    }

    let original_words = normalized_words(original);
    let candidate_words = normalized_words(&cleaned);
    if original_words.len() >= 3 && candidate_words.is_empty() {
        reasons.push("output lost all normalized words".to_string());
    }

    if original_words.len() >= 3 && !candidate_words.is_empty() {
        let lexical_overlap = lexical_overlap_ratio(original, &cleaned);
        let bag_overlap = multiset_overlap_ratio(&original_words, &candidate_words);
        let ordered_overlap = ordered_overlap_ratio(&original_words, &candidate_words);

        if lexical_overlap < 0.25 || bag_overlap < 0.30 {
            reasons.push(format!(
                "high lexical drift (set overlap {:.2}, token overlap {:.2})",
                lexical_overlap, bag_overlap
            ));
        }

        if bag_overlap >= 0.45 && ordered_overlap + 0.20 < bag_overlap {
            reasons.push(format!(
                "token order changed (ordered overlap {:.2} vs token overlap {:.2})",
                ordered_overlap, bag_overlap
            ));
        }

        if original_words.len() >= 8 && ordered_overlap < 0.22 {
            reasons.push(format!(
                "possible paraphrase drift (ordered overlap {:.2})",
                ordered_overlap
            ));
        }
    }

    let missing_tokens = missing_protected_tokens(original, &cleaned);
    if !missing_tokens.is_empty() {
        reasons.push(format!(
            "missing protected tokens: {}",
            missing_tokens.join(", ")
        ));
    }

    if let (Some(original_script), Some(candidate_script)) =
        (dominant_script(original), dominant_script(&cleaned))
    {
        if original_script != candidate_script {
            reasons.push(format!(
                "language/script changed ({} -> {})",
                original_script.as_str(),
                candidate_script.as_str()
            ));
        }
    }

    if reasons.is_empty() {
        Ok(cleaned)
    } else {
        Err(CleanupValidationError { reasons })
    }
}

async fn post_process_transcription(settings: &AppSettings, transcription: &str) -> Option<String> {
    let provider = match settings.active_post_process_provider().cloned() {
        Some(provider) => provider,
        None => {
            debug!("Post-processing enabled but no provider is selected");
            return None;
        }
    };

    let model = settings
        .post_process_models
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    if model.trim().is_empty() {
        debug!(
            "Post-processing skipped because provider '{}' has no model configured",
            provider.id
        );
        return None;
    }

    let selected_prompt_id = match &settings.post_process_selected_prompt_id {
        Some(id) => id.clone(),
        None => {
            debug!("Post-processing skipped because no prompt is selected");
            return None;
        }
    };

    let prompt = match settings
        .post_process_prompts
        .iter()
        .find(|prompt| prompt.id == selected_prompt_id)
    {
        Some(prompt) => prompt.prompt.clone(),
        None => {
            debug!(
                "Post-processing skipped because prompt '{}' was not found",
                selected_prompt_id
            );
            return None;
        }
    };

    if prompt.trim().is_empty() {
        debug!("Post-processing skipped because the selected prompt is empty");
        return None;
    }

    debug!(
        "Starting LLM post-processing with provider '{}' (model: {})",
        provider.id, model
    );

    let api_key = settings
        .post_process_api_keys
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    let local_openai_provider = crate::settings::is_local_post_process_provider(&provider);
    let protected_tokens = extract_protected_tokens(transcription);
    let protected_tokens_for_prompt = dedupe_tokens(&protected_tokens);
    if !protected_tokens_for_prompt.is_empty() {
        debug!(
            "Cleanup contract protecting {} token(s) for provider '{}'",
            protected_tokens_for_prompt.len(),
            provider.id
        );
    }

    // Disable reasoning for providers where post-processing rarely benefits from it.
    // Toaster is local-only, so only the local OpenAI-compatible path sets
    // `reasoning_effort`; the `reasoning` nested-object path is unused.
    let (reasoning_effort, reasoning): (Option<String>, Option<crate::llm_client::ReasoningConfig>) =
        if local_openai_provider {
            (Some("none".to_string()), None)
        } else {
            (None, None)
        };

    let inputs = AttemptInputs {
        provider: &provider,
        api_key,
        model: &model,
        transcription,
        prompt: &prompt,
        protected_tokens_for_prompt: &protected_tokens_for_prompt,
        local_openai_provider,
        reasoning_effort,
        reasoning,
    };

    let should_attempt_structured = provider.supports_structured_output || local_openai_provider;
    if should_attempt_structured {
        if let AttemptOutcome::Success(validated) = try_llm_attempt(&inputs, true).await {
            return Some(validated);
        }
    }

    // Legacy mode fallback for providers without structured output compatibility,
    // or when the structured attempt did not produce a usable result.
    match try_llm_attempt(&inputs, false).await {
        AttemptOutcome::Success(validated) => Some(validated),
        AttemptOutcome::Fallback => None,
    }
}

async fn maybe_convert_chinese_variant(
    settings: &AppSettings,
    transcription: &str,
) -> Option<String> {
    let is_simplified = settings.selected_language == "zh-Hans";
    let is_traditional = settings.selected_language == "zh-Hant";

    if !is_simplified && !is_traditional {
        debug!("selected_language is not Simplified or Traditional Chinese; skipping translation");
        return None;
    }

    debug!(
        "Starting Chinese translation using OpenCC for language: {}",
        settings.selected_language
    );

    let config = if is_simplified {
        BuiltinConfig::Tw2sp
    } else {
        BuiltinConfig::S2tw
    };

    match OpenCC::from_config(config) {
        Ok(converter) => {
            let converted = converter.convert(transcription);
            debug!(
                "OpenCC translation completed. Input length: {}, Output length: {}",
                transcription.len(),
                converted.len()
            );
            Some(converted)
        }
        Err(e) => {
            error!("Failed to initialize OpenCC converter: {}. Falling back to original transcription.", e);
            None
        }
    }
}

fn normalize_diff_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn has_meaningful_text_diff(original: &str, cleaned: &str) -> bool {
    normalize_diff_text(original) != normalize_diff_text(cleaned)
}

pub(crate) struct ProcessedTranscription {
    pub post_processed_text: Option<String>,
    pub post_process_prompt: Option<String>,
}

pub(crate) async fn process_transcription_output(
    app: &AppHandle,
    transcription: &str,
    post_process: bool,
) -> ProcessedTranscription {
    let settings = get_settings(app);
    let mut final_text = transcription.to_string();
    let mut post_processed_text: Option<String> = None;
    let mut post_process_prompt: Option<String> = None;

    if let Some(converted_text) = maybe_convert_chinese_variant(&settings, transcription).await {
        final_text = converted_text;
    }

    if post_process {
        let pre_post_process_text = final_text.clone();
        let processed_text = post_process_transcription(&settings, &final_text).await;

        if let Some(processed_text) = processed_text {
            if has_meaningful_text_diff(&pre_post_process_text, &processed_text) {
                post_processed_text = Some(processed_text.clone());

                if let Some(prompt_id) = &settings.post_process_selected_prompt_id {
                    if let Some(prompt) = settings
                        .post_process_prompts
                        .iter()
                        .find(|prompt| &prompt.id == prompt_id)
                    {
                        post_process_prompt = Some(prompt.prompt.clone());
                    }
                }
            }
        }
    } else if final_text != transcription {
        post_processed_text = Some(final_text.clone());
    }

    ProcessedTranscription {
        post_processed_text,
        post_process_prompt,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
