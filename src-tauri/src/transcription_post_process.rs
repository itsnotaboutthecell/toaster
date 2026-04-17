// Extracted from actions.rs by p1-extract-process-transcription (scripts/_split_actions.py).
// Holds the live transcription post-processing pipeline. Dictation state
// machine stays in actions.rs until p1-remove-actions deletes it.

use crate::managers::editor::LocalLlmWordProposal;
use crate::settings::{get_settings, AppSettings, APPLE_INTELLIGENCE_PROVIDER_ID};
use ferrous_opencc::{config::BuiltinConfig, OpenCC};
use log::{debug, error, warn};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use transcribe_rs::TranscriptionSegment;
#[derive(Clone, serde::Serialize)]
struct LocalCleanupReviewRequestEvent {
    request_id: String,
    original_text: String,
    cleaned_text: String,
}

const LOCAL_CLEANUP_REVIEW_TIMEOUT_SECS: u64 = 45;

const TRANSCRIPTION_FIELD: &str = "transcription";
const CLEANUP_CONTRACT_VERSION: &str = "transcript_cleanup_contract_v1";
const CLEANUP_TRANSCRIPTION_FIELD: &str = "cleaned_transcription";

/// Strip invisible Unicode characters that some LLMs may insert
fn strip_invisible_chars(s: &str) -> String {
    s.replace(['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'], "")
}

/// Build a system prompt from the user's prompt template.
/// Removes `${output}` placeholder since the transcription is sent as the user message.
fn build_system_prompt(prompt_template: &str) -> String {
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
struct CleanupContractResponse {
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

fn build_cleanup_contract_system_prompt(
    prompt_template: &str,
    protected_tokens: &[String],
) -> String {
    let user_prompt = build_system_prompt(prompt_template);
    let protected_tokens_clause = if protected_tokens.is_empty() {
        "- Protected tokens in source transcript: none detected".to_string()
    } else {
        format!(
            "- Protected tokens in source transcript (must be unchanged): {}",
            protected_tokens.join(", ")
        )
    };

    if user_prompt.is_empty() {
        format!(
            "You are a transcript cleanup engine.\n\
             Follow these non-negotiable invariants:\n\
             - Preserve the source language.\n\
             - Do not reorder surviving content.\n\
             - Do not paraphrase meaning.\n\
             {}\n\
             If any invariant cannot be satisfied, return the source transcript unchanged.\n\
             Return only JSON that matches the cleanup contract schema.",
            protected_tokens_clause
        )
    } else {
        format!(
            "You are a transcript cleanup engine.\n\
             Follow these non-negotiable invariants:\n\
             - Preserve the source language.\n\
             - Do not reorder surviving content.\n\
             - Do not paraphrase meaning.\n\
             {}\n\
             If any invariant cannot be satisfied, return the source transcript unchanged.\n\
             Return only JSON that matches the cleanup contract schema.\n\n\
             User cleanup instructions:\n{}",
            protected_tokens_clause, user_prompt
        )
    }
}

fn build_cleanup_legacy_prompt(
    prompt_template: &str,
    transcription: &str,
    protected_tokens: &[String],
) -> String {
    let base_prompt = prompt_template.replace("${output}", transcription);
    let protected_tokens_clause = if protected_tokens.is_empty() {
        "none detected".to_string()
    } else {
        protected_tokens.join(", ")
    };

    format!(
        "{}\n\nNon-negotiable constraints:\n\
         - Preserve source language.\n\
         - Do not reorder words or paraphrase meaning.\n\
         - Keep protected tokens unchanged: {}\n\
         Return only cleaned transcript text.",
        base_prompt, protected_tokens_clause
    )
}

fn build_cleanup_contract_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "contract_version": {
                "type": "string",
                "const": CLEANUP_CONTRACT_VERSION
            },
            (CLEANUP_TRANSCRIPTION_FIELD): {
                "type": "string",
                "description": "The cleaned transcript text after safe corrections"
            },
            "invariants": {
                "type": "object",
                "properties": {
                    "preserve_language": { "type": "boolean" },
                    "no_reorder": { "type": "boolean" },
                    "no_paraphrase": { "type": "boolean" },
                    "protected_tokens_preserved": { "type": "boolean" }
                },
                "required": [
                    "preserve_language",
                    "no_reorder",
                    "no_paraphrase",
                    "protected_tokens_preserved"
                ],
                "additionalProperties": false
            }
        },
        "required": [
            "contract_version",
            CLEANUP_TRANSCRIPTION_FIELD,
            "invariants"
        ],
        "additionalProperties": false
    })
}

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

fn validate_cleanup_candidate(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WordRange {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy)]
struct WordToken {
    start: usize,
    end: usize,
}

fn collect_word_tokens(text: &str) -> Vec<WordToken> {
    let mut tokens = Vec::new();
    let mut word_start: Option<usize> = None;

    for (idx, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(start) = word_start.take() {
                tokens.push(WordToken { start, end: idx });
            }
        } else if word_start.is_none() {
            word_start = Some(idx);
        }
    }

    if let Some(start) = word_start {
        tokens.push(WordToken {
            start,
            end: text.len(),
        });
    }

    tokens
}

fn normalize_word_for_match(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

fn words_match_loosely(left: &str, right: &str) -> bool {
    if left.is_empty() || right.is_empty() {
        return false;
    }
    left == right
        || left.starts_with(right)
        || right.starts_with(left)
        || left.trim_matches(|c: char| !c.is_alphanumeric()) == right
}

fn has_repeated_word_run(words: &[String]) -> bool {
    if words.len() < 3 {
        return false;
    }

    let mut run = 1usize;
    for pair in words.windows(2) {
        if pair[0] == pair[1] {
            run += 1;
            if run >= 3 {
                return true;
            }
        } else {
            run = 1;
        }
    }

    false
}

fn merge_and_bound_word_ranges(
    mut ranges: Vec<WordRange>,
    total_words: usize,
    max_words_per_span: usize,
    max_spans: usize,
) -> Vec<WordRange> {
    if ranges.is_empty() || total_words == 0 || max_words_per_span == 0 {
        return Vec::new();
    }

    ranges.sort_by_key(|range| range.start);
    let mut merged: Vec<WordRange> = Vec::new();

    for mut range in ranges {
        range.start = range.start.min(total_words);
        range.end = range.end.min(total_words);
        if range.end <= range.start {
            continue;
        }

        if let Some(last) = merged.last_mut() {
            if range.start <= last.end {
                last.end = last.end.max(range.end);
                continue;
            }
        }

        merged.push(range);
    }

    let mut bounded = Vec::new();
    for range in merged {
        let mut cursor = range.start;
        while cursor < range.end {
            let chunk_end = (cursor + max_words_per_span).min(range.end);
            bounded.push(WordRange {
                start: cursor,
                end: chunk_end,
            });
            if bounded.len() >= max_spans {
                return bounded;
            }
            cursor = chunk_end;
        }
    }

    bounded
}

fn extract_low_confidence_word_ranges(
    transcription: &str,
    segments: Option<&[TranscriptionSegment]>,
) -> Vec<WordRange> {
    // transcribe-rs segments currently expose start/end/text but no explicit
    // probability score. We derive a conservative confidence proxy from how
    // well each segment text aligns to the filtered final transcript, then
    // combine that with safe heuristics (mismatch density, repeated words,
    // implausible speech rate) to pick spans worth local LLM cleanup.
    const LOOKAHEAD_WORDS: usize = 24;
    const LOW_ALIGNMENT_CONFIDENCE: f32 = 0.72;
    const MAX_WORDS_PER_SPAN: usize = 40;
    const MAX_SPANS: usize = 8;

    let Some(segments) = segments else {
        return Vec::new();
    };

    if segments.is_empty() {
        return Vec::new();
    }

    let tokens = collect_word_tokens(transcription);
    if tokens.is_empty() {
        return Vec::new();
    }

    let normalized_tokens: Vec<String> = tokens
        .iter()
        .map(|token| normalize_word_for_match(&transcription[token.start..token.end]))
        .collect();

    let mut cursor = 0usize;
    let mut ranges = Vec::new();

    for segment in segments {
        let segment_words: Vec<String> = segment
            .text
            .split_whitespace()
            .map(normalize_word_for_match)
            .filter(|word| !word.is_empty())
            .collect();
        if segment_words.is_empty() {
            continue;
        }

        let range_start = cursor.min(normalized_tokens.len());
        let mut matched = 0usize;
        let mut unmatched = 0usize;
        let mut last_match: Option<usize> = None;

        for segment_word in &segment_words {
            if cursor >= normalized_tokens.len() {
                unmatched += 1;
                continue;
            }

            let search_end = (cursor + LOOKAHEAD_WORDS).min(normalized_tokens.len());
            let mut found_at: Option<usize> = None;
            for idx in cursor..search_end {
                if words_match_loosely(&normalized_tokens[idx], segment_word) {
                    found_at = Some(idx);
                    break;
                }
            }

            if let Some(idx) = found_at {
                matched += 1;
                last_match = Some(idx);
                cursor = idx + 1;
            } else {
                unmatched += 1;
            }
        }

        let estimated_end = (range_start + segment_words.len()).min(normalized_tokens.len());
        cursor = cursor.max(estimated_end).min(normalized_tokens.len());

        let range_end = match last_match {
            Some(last) => (last + 1).max(estimated_end),
            None => estimated_end.max((range_start + 1).min(normalized_tokens.len())),
        };

        if range_end <= range_start {
            continue;
        }

        let alignment_confidence = matched as f32 / segment_words.len().max(1) as f32;
        let low_confidence = alignment_confidence < LOW_ALIGNMENT_CONFIDENCE;
        let mismatch_heavy = unmatched >= 2 && unmatched >= matched.max(1);
        let repeated_words = has_repeated_word_run(&segment_words);
        let duration_s = (segment.end - segment.start).max(0.0);
        let words_per_second = if duration_s > 0.05 {
            segment_words.len() as f32 / duration_s
        } else {
            f32::INFINITY
        };
        let unlikely_speech_rate = words_per_second.is_finite()
            && (words_per_second > 6.0 || (segment_words.len() >= 3 && words_per_second < 0.25));

        if low_confidence || mismatch_heavy || repeated_words || unlikely_speech_rate {
            ranges.push(WordRange {
                start: range_start,
                end: range_end,
            });
        }
    }

    merge_and_bound_word_ranges(
        ranges,
        normalized_tokens.len(),
        MAX_WORDS_PER_SPAN,
        MAX_SPANS,
    )
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

fn merge_span_rewrites(
    transcription: &str,
    tokens: &[WordToken],
    replacements: &[LocalLlmWordProposal],
) -> String {
    if replacements.is_empty() {
        return transcription.to_string();
    }

    let mut byte_replacements: Vec<(usize, usize, String)> = Vec::new();
    for replacement in replacements {
        if replacement.start_word_index >= replacement.end_word_index
            || replacement.end_word_index > tokens.len()
        {
            continue;
        }
        let start = tokens[replacement.start_word_index].start;
        let end = tokens[replacement.end_word_index - 1].end;
        if end <= start || end > transcription.len() {
            continue;
        }
        if replacement.replacement_words.is_empty() {
            continue;
        }
        let replacement_text = replacement.replacement_words.join(" ");
        byte_replacements.push((start, end, replacement_text));
    }

    if byte_replacements.is_empty() {
        return transcription.to_string();
    }

    byte_replacements.sort_by_key(|(start, _, _)| *start);

    let mut merged = String::with_capacity(transcription.len());
    let mut cursor = 0usize;
    for (start, end, replacement) in byte_replacements {
        if start < cursor {
            continue;
        }
        merged.push_str(&transcription[cursor..start]);
        merged.push_str(&replacement);
        cursor = end;
    }
    merged.push_str(&transcription[cursor..]);
    merged
}

fn build_safe_span_proposal(
    range: WordRange,
    validated_span: &str,
) -> Result<LocalLlmWordProposal, String> {
    let expected_words = range.end.saturating_sub(range.start);
    if expected_words == 0 {
        return Err("proposal range is empty".to_string());
    }

    let replacement_words: Vec<String> = validated_span
        .split_whitespace()
        .map(|word| word.trim().to_string())
        .filter(|word| !word.is_empty())
        .collect();
    if replacement_words.len() != expected_words {
        return Err(format!(
            "replacement word count mismatch for range {}..{}: expected {}, got {}",
            range.start,
            range.end,
            expected_words,
            replacement_words.len()
        ));
    }

    Ok(LocalLlmWordProposal {
        start_word_index: range.start,
        end_word_index: range.end,
        replacement_words,
    })
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

    let local_openai_provider = crate::settings::is_local_post_process_provider(&provider)
        && provider.id != APPLE_INTELLIGENCE_PROVIDER_ID;
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
    // - local providers: top-level reasoning_effort (works for local OpenAI-compat servers)
    // - openrouter: nested reasoning object; exclude:true also keeps reasoning text
    //   out of the response so it can't pollute structured-output JSON parsing
    let (reasoning_effort, reasoning) = if local_openai_provider {
        (Some("none".to_string()), None)
    } else {
        match provider.id.as_str() {
            "openrouter" => (
                None,
                Some(crate::llm_client::ReasoningConfig {
                    effort: Some("none".to_string()),
                    exclude: Some(true),
                }),
            ),
            _ => (None, None),
        }
    };

    let should_attempt_structured = provider.supports_structured_output || local_openai_provider;
    if should_attempt_structured {
        debug!("Using structured outputs for provider '{}'", provider.id);

        let system_prompt =
            build_cleanup_contract_system_prompt(&prompt, &protected_tokens_for_prompt);
        let user_content = transcription.to_string();

        // Apple Intelligence bridge removed — the provider entry is kept in
        // settings for backwards compatibility but produces no output.
        if provider.id == APPLE_INTELLIGENCE_PROVIDER_ID {
            debug!("Apple Intelligence provider selected but native bridge is removed");
            return None;
        }

        let json_schema = build_cleanup_contract_schema();

        match crate::llm_client::send_chat_completion_with_schema(
            &provider,
            api_key.clone(),
            &model,
            user_content,
            Some(system_prompt),
            Some(json_schema),
            reasoning_effort.clone(),
            reasoning.clone(),
        )
        .await
        {
            Ok(Some(content)) => match serde_json::from_str::<CleanupContractResponse>(&content) {
                Ok(contract_response) => match validate_cleanup_candidate(
                    transcription,
                    &contract_response.cleaned_transcription,
                    Some(&contract_response),
                ) {
                    Ok(validated) => {
                        debug!(
                                "Structured cleanup post-processing succeeded for provider '{}'. Output length: {} chars",
                                provider.id,
                                validated.len()
                            );
                        return Some(validated);
                    }
                    Err(validation_error) => {
                        warn!(
                                "Structured cleanup output rejected for provider '{}': {}. Falling back to legacy mode.",
                                provider.id, validation_error
                            );
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
                        match validate_cleanup_candidate(transcription, &candidate, None) {
                            Ok(validated) => {
                                debug!(
                                        "Structured compatibility fallback succeeded for provider '{}'. Output length: {} chars",
                                        provider.id,
                                        validated.len()
                                    );
                                return Some(validated);
                            }
                            Err(validation_error) => {
                                warn!(
                                        "Structured compatibility fallback rejected for provider '{}': {}. Falling back to legacy mode.",
                                        provider.id, validation_error
                                    );
                            }
                        }
                    } else {
                        warn!(
                                "Structured response from provider '{}' did not contain '{}' or '{}'; falling back to legacy mode.",
                                provider.id, CLEANUP_TRANSCRIPTION_FIELD, TRANSCRIPTION_FIELD
                            );
                    }
                }
            },
            Ok(None) => {
                warn!(
                    "Structured output API returned no content for provider '{}'; falling back to legacy mode.",
                    provider.id
                );
            }
            Err(e) => {
                warn!(
                    "Structured output call failed for provider '{}': {}. Falling back to legacy mode.",
                    provider.id, e
                );
            }
        }
    }

    // Legacy mode fallback for providers without structured output compatibility.
    let processed_prompt =
        build_cleanup_legacy_prompt(&prompt, transcription, &protected_tokens_for_prompt);
    debug!("Processed prompt length: {} chars", processed_prompt.len());

    let max_attempts = if local_openai_provider { 2 } else { 1 };
    for attempt in 1..=max_attempts {
        match crate::llm_client::send_chat_completion(
            &provider,
            api_key.clone(),
            &model,
            processed_prompt.clone(),
            reasoning_effort.clone(),
            reasoning.clone(),
        )
        .await
        {
            Ok(Some(content)) => match validate_cleanup_candidate(transcription, &content, None) {
                Ok(validated) => {
                    debug!(
                        "LLM post-processing succeeded for provider '{}'. Output length: {} chars",
                        provider.id,
                        validated.len()
                    );
                    return Some(validated);
                }
                Err(validation_error) => {
                    if local_openai_provider && attempt < max_attempts {
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
                    return None;
                }
            },
            Ok(None) => {
                error!(
                    "LLM post-processing returned no content for provider '{}'; preserving original transcription",
                    provider.id
                );
                return None;
            }
            Err(e) => {
                if local_openai_provider && attempt < max_attempts {
                    warn!(
                        "Transient local LLM error for provider '{}' (attempt {}): {}. Retrying once.",
                        provider.id, attempt, e
                    );
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    continue;
                }

                error!(
                    "LLM post-processing failed for provider '{}': {}. Falling back to original transcription.",
                    provider.id,
                    e
                );
                return None;
            }
        }
    }

    None
}

async fn post_process_low_confidence_spans(
    settings: &AppSettings,
    transcription: &str,
    segments: Option<&[TranscriptionSegment]>,
) -> Option<String> {
    let tokens = collect_word_tokens(transcription);
    if tokens.is_empty() {
        debug!("Post-processing skipped because transcription has no words");
        return None;
    }

    let span_ranges = extract_low_confidence_word_ranges(transcription, segments);
    if span_ranges.is_empty() {
        debug!("Post-processing skipped because no low-confidence spans were detected");
        return None;
    }

    let mut replacements: Vec<LocalLlmWordProposal> = Vec::new();
    for range in span_ranges {
        let span_start = tokens[range.start].start;
        let span_end = tokens[range.end - 1].end;
        if span_end <= span_start || span_end > transcription.len() {
            warn!(
                "Skipping malformed post-process span {}..{} ({}..{} bytes)",
                range.start, range.end, span_start, span_end
            );
            continue;
        }

        let original_span = &transcription[span_start..span_end];
        match post_process_transcription(settings, original_span).await {
            Some(processed_span) => {
                match validate_cleanup_candidate(original_span, &processed_span, None) {
                    Ok(validated_span) => match build_safe_span_proposal(range, &validated_span) {
                        Ok(proposal) => replacements.push(proposal),
                        Err(validation_error) => {
                            warn!(
                                    "Rejected span proposal for range {}..{}: {}. Preserving original text.",
                                    range.start, range.end, validation_error
                                );
                        }
                    },
                    Err(validation_error) => {
                        warn!(
                            "Rejected unsafe post-process rewrite for span {}..{}: {}. Preserving original text.",
                            range.start, range.end, validation_error
                        );
                    }
                }
            }
            None => {
                debug!(
                    "Post-processing failed for span {}..{}; preserving original text",
                    range.start, range.end
                );
            }
        }
    }

    if replacements.is_empty() {
        debug!("No safe span rewrites were produced; preserving original transcription");
        return None;
    }

    let merged = merge_span_rewrites(transcription, &tokens, &replacements);
    if merged == transcription {
        None
    } else {
        Some(merged)
    }
}

async fn maybe_convert_chinese_variant(
    settings: &AppSettings,
    transcription: &str,
) -> Option<String> {
    // Check if language is set to Simplified or Traditional Chinese
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

    // Use OpenCC to convert based on selected language
    let config = if is_simplified {
        // Convert Traditional Chinese to Simplified Chinese
        BuiltinConfig::Tw2sp
    } else {
        // Convert Simplified Chinese to Traditional Chinese
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

async fn request_local_cleanup_review(
    app: &AppHandle,
    original_text: &str,
    cleaned_text: &str,
) -> Option<bool> {
    let review_state = match app.try_state::<crate::LocalCleanupReviewState>() {
        Some(state) => state,
        None => {
            warn!("Cleanup review state is unavailable; skipping review prompt");
            return None;
        }
    };

    let (request_id, decision_rx) = review_state.register();

    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.show();
        let _ = main_window.set_focus();
    }

    let emit_result = app.emit(
        "local-cleanup-review-request",
        LocalCleanupReviewRequestEvent {
            request_id: request_id.clone(),
            original_text: original_text.to_string(),
            cleaned_text: cleaned_text.to_string(),
        },
    );
    if let Err(err) = emit_result {
        review_state.remove(&request_id);
        warn!("Failed to emit cleanup review request: {}", err);
        return None;
    }

    match tokio::time::timeout(
        Duration::from_secs(LOCAL_CLEANUP_REVIEW_TIMEOUT_SECS),
        decision_rx,
    )
    .await
    {
        Ok(Ok(accept)) => Some(accept),
        Ok(Err(_)) => {
            warn!("Cleanup review channel closed before receiving a decision");
            None
        }
        Err(_) => {
            review_state.remove(&request_id);
            warn!(
                "Cleanup review timed out after {}s; defaulting to original transcript",
                LOCAL_CLEANUP_REVIEW_TIMEOUT_SECS
            );
            None
        }
    }
}

pub(crate) struct ProcessedTranscription {
    pub post_processed_text: Option<String>,
    pub post_process_prompt: Option<String>,
}

pub(crate) async fn process_transcription_output(
    app: &AppHandle,
    transcription: &str,
    segments: Option<&[TranscriptionSegment]>,
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
        let use_low_confidence_routing = settings
            .active_post_process_provider()
            .map(crate::settings::is_local_post_process_provider)
            .unwrap_or(false);

        let processed_text = if use_low_confidence_routing {
            post_process_low_confidence_spans(&settings, &final_text, segments).await
        } else {
            post_process_transcription(&settings, &final_text).await
        };

        if let Some(processed_text) = processed_text {
            let has_meaningful_diff =
                has_meaningful_text_diff(&pre_post_process_text, &processed_text);

            if has_meaningful_diff {
                let accepted = if use_low_confidence_routing {
                    request_local_cleanup_review(app, &pre_post_process_text, &processed_text)
                        .await
                        .unwrap_or(false)
                } else {
                    true
                };

                if accepted {
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
mod tests {
    use super::*;

    fn segment(start: f32, end: f32, text: &str) -> TranscriptionSegment {
        TranscriptionSegment {
            start,
            end,
            text: text.to_string(),
        }
    }

    fn valid_contract_response(cleaned_transcription: &str) -> CleanupContractResponse {
        CleanupContractResponse {
            contract_version: CLEANUP_CONTRACT_VERSION.to_string(),
            cleaned_transcription: cleaned_transcription.to_string(),
            invariants: CleanupInvariantClaims {
                preserve_language: true,
                no_reorder: true,
                no_paraphrase: true,
                protected_tokens_preserved: true,
            },
        }
    }

    #[test]
    fn extracts_low_confidence_ranges_from_mismatched_segments() {
        let transcription = "hello wurld this is stable";
        let segments = vec![
            segment(0.0, 1.0, "hello world"),
            segment(1.0, 2.0, "this is stable"),
        ];

        let ranges = extract_low_confidence_word_ranges(transcription, Some(&segments));

        assert_eq!(ranges, vec![WordRange { start: 0, end: 2 }]);
    }

    #[test]
    fn extracts_low_confidence_ranges_from_repeated_word_heuristic() {
        let transcription = "uh uh uh okay stable";
        let segments = vec![
            segment(0.0, 1.2, "uh uh uh okay"),
            segment(1.2, 1.8, "stable"),
        ];

        let ranges = extract_low_confidence_word_ranges(transcription, Some(&segments));

        assert_eq!(ranges, vec![WordRange { start: 0, end: 4 }]);
    }

    #[test]
    fn merge_span_rewrites_preserves_non_target_text() {
        let transcription = "alpha beta  gamma delta";
        let tokens = collect_word_tokens(transcription);
        let rewritten = merge_span_rewrites(
            transcription,
            &tokens,
            &[LocalLlmWordProposal {
                start_word_index: 1,
                end_word_index: 3,
                replacement_words: vec!["BETA".to_string(), "GAMMA".to_string()],
            }],
        );

        assert_eq!(rewritten, "alpha BETA GAMMA delta");
        assert!(rewritten.starts_with("alpha "));
        assert!(rewritten.ends_with(" delta"));
    }

    #[test]
    fn safe_span_proposal_rejects_word_count_mismatch() {
        let error = build_safe_span_proposal(WordRange { start: 0, end: 2 }, "single")
            .expect_err("proposal should be rejected when replacement word count mismatches");

        assert!(error.contains("word count mismatch"));
    }

    #[test]
    fn safe_span_proposal_rejects_beginning_word_deletion() {
        let error = build_safe_span_proposal(WordRange { start: 0, end: 3 }, "world today")
            .expect_err("proposal should reject beginning-word deletion");

        assert!(error.contains("replacement word count mismatch"));
        assert!(error.contains("expected 3, got 2"));
    }

    #[test]
    fn merge_span_rewrites_handles_punctuation_adjacent_edits() {
        let transcription = "Helo, wrld! Keep-this stable.";
        let tokens = collect_word_tokens(transcription);
        let rewritten = merge_span_rewrites(
            transcription,
            &tokens,
            &[
                LocalLlmWordProposal {
                    start_word_index: 0,
                    end_word_index: 1,
                    replacement_words: vec!["Hello,".to_string()],
                },
                LocalLlmWordProposal {
                    start_word_index: 1,
                    end_word_index: 2,
                    replacement_words: vec!["world!".to_string()],
                },
            ],
        );

        assert_eq!(rewritten, "Hello, world! Keep-this stable.");
        assert!(rewritten.ends_with("Keep-this stable."));
    }

    #[test]
    fn rejects_destructive_span_rewrites() {
        assert!(validate_cleanup_candidate("helo world", "hello world", None).is_ok());
        assert!(validate_cleanup_candidate(
            "hello world today",
            "This output rewrites the entire section into unrelated content.",
            None
        )
        .is_err());
    }

    #[test]
    fn accepts_valid_cleanup_contract_output() {
        let original = "Invoice total is $25 on 2024-01-02.";
        let contract = valid_contract_response("Invoice total is $25 on 2024-01-02!");

        let validated =
            validate_cleanup_candidate(original, &contract.cleaned_transcription, Some(&contract));

        assert_eq!(
            validated.expect("output should pass cleanup validation"),
            "Invoice total is $25 on 2024-01-02!"
        );
    }

    #[test]
    fn rejects_contract_when_model_reports_invariant_failure() {
        let original = "Keep this sentence exactly as written.";
        let mut contract = valid_contract_response("Keep this sentence exactly as written!");
        contract.invariants.no_paraphrase = false;

        let validation =
            validate_cleanup_candidate(original, &contract.cleaned_transcription, Some(&contract));

        let error = validation.expect_err("validation should fail when claim is false");
        assert!(error.to_string().contains("no_paraphrase=false"));
    }

    #[test]
    fn rejects_output_when_protected_tokens_are_dropped() {
        let original = "Budget is $5 and ref A1B2 remains.";
        let contract = valid_contract_response("Budget is five dollars and ref AB remains.");

        let validation =
            validate_cleanup_candidate(original, &contract.cleaned_transcription, Some(&contract));

        let error =
            validation.expect_err("validation should fail when protected tokens are missing");
        assert!(error.to_string().contains("missing protected tokens"));
        assert!(error.to_string().contains("$5"));
    }

    #[test]
    fn rejects_reordered_output() {
        let original = "alpha beta gamma delta epsilon";
        let contract = valid_contract_response("delta gamma beta alpha epsilon");

        let validation =
            validate_cleanup_candidate(original, &contract.cleaned_transcription, Some(&contract));

        let error = validation.expect_err("validation should fail for reordered output");
        assert!(error.to_string().contains("token order changed"));
    }

    #[test]
    fn rejects_language_script_drift() {
        let original = "你好 世界 今天";
        let contract = valid_contract_response("hello world today");

        let validation =
            validate_cleanup_candidate(original, &contract.cleaned_transcription, Some(&contract));

        let error = validation.expect_err("validation should fail for script changes");
        assert!(error.to_string().contains("language/script changed"));
    }
}
