//! Cleanup prompt + schema builders (extracted from cleanup/mod.rs).

use super::{build_system_prompt, CLEANUP_CONTRACT_VERSION, CLEANUP_TRANSCRIPTION_FIELD};

pub(super) fn build_cleanup_contract_system_prompt(
    prompt_template: &str,
    protected_tokens: &[String],
    filler_words: &[String],
) -> String {
    let user_prompt = build_system_prompt(prompt_template);
    let user_prompt = substitute_filler_placeholder(&user_prompt, filler_words);
    let protected_tokens_clause = if protected_tokens.is_empty() {
        "- Protected tokens in source transcript: none detected".to_string()
    } else {
        format!(
            "- Protected tokens in source transcript (must be unchanged): {}",
            protected_tokens.join(", ")
        )
    };
    let filler_words_clause = filler_words_clause(filler_words);

    if user_prompt.is_empty() {
        format!(
            "You are a transcript cleanup engine.\n\
             Follow these non-negotiable invariants:\n\
             - Preserve the source language.\n\
             - Do not reorder surviving content.\n\
             - Do not paraphrase meaning.\n\
             {}{}\n\
             If any invariant cannot be satisfied, return the source transcript unchanged.\n\
             Return only JSON that matches the cleanup contract schema.",
            protected_tokens_clause, filler_words_clause
        )
    } else {
        format!(
            "You are a transcript cleanup engine.\n\
             Follow these non-negotiable invariants:\n\
             - Preserve the source language.\n\
             - Do not reorder surviving content.\n\
             - Do not paraphrase meaning.\n\
             {}{}\n\
             If any invariant cannot be satisfied, return the source transcript unchanged.\n\
             Return only JSON that matches the cleanup contract schema.\n\n\
             User cleanup instructions:\n{}",
            protected_tokens_clause, filler_words_clause, user_prompt
        )
    }
}

pub(super) fn build_cleanup_legacy_prompt(
    prompt_template: &str,
    transcription: &str,
    protected_tokens: &[String],
    filler_words: &[String],
) -> String {
    let with_filler = substitute_filler_placeholder(prompt_template, filler_words);
    let base_prompt = with_filler.replace("${output}", transcription);
    let protected_tokens_clause = if protected_tokens.is_empty() {
        "none detected".to_string()
    } else {
        protected_tokens.join(", ")
    };
    let filler_words_clause = filler_words_clause(filler_words);

    format!(
        "{}\n\nNon-negotiable constraints:\n\
         - Preserve source language.\n\
         - Do not reorder words or paraphrase meaning.\n\
         - Keep protected tokens unchanged: {}{}\n\
         Return only cleaned transcript text.",
        base_prompt, protected_tokens_clause, filler_words_clause
    )
}

/// Render the filler-word list as a prompt clause. Returns an empty string
/// when the user has not configured any filler words, so the resulting prompt
/// makes no mention of filler removal. Never falls back to a hardcoded list:
/// the UI's Discard Words list is the single source of truth.
fn filler_words_clause(filler_words: &[String]) -> String {
    if filler_words.is_empty() {
        String::new()
    } else {
        format!(
            "\n- Filler words to remove when appropriate: {}",
            filler_words.join(", ")
        )
    }
}

/// Replace a `${filler_words}` placeholder in a user-authored prompt template
/// with the comma-separated filler-word list. When the list is empty the
/// placeholder expands to "none" so the resulting prompt is still well-formed
/// and never leaks the literal `${filler_words}` token to the LLM.
#[allow(dead_code)] // reachable only once the cleanup pipeline is wired to a Tauri command; see `managers::cleanup::llm_dispatch` callers.
fn substitute_filler_placeholder(template: &str, filler_words: &[String]) -> String {
    if !template.contains("${filler_words}") {
        return template.to_string();
    }
    let replacement = if filler_words.is_empty() {
        "none".to_string()
    } else {
        filler_words.join(", ")
    };
    template.replace("${filler_words}", &replacement)
}

#[allow(dead_code)] // reachable only once the cleanup pipeline is wired to a Tauri command; consumed by `managers::cleanup::llm_dispatch`.
pub(super) fn build_cleanup_contract_schema() -> serde_json::Value {
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
