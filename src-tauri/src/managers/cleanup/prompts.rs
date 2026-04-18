//! Cleanup prompt + schema builders (extracted from cleanup/mod.rs).

use super::{build_system_prompt, CLEANUP_CONTRACT_VERSION, CLEANUP_TRANSCRIPTION_FIELD};

pub(super) fn build_cleanup_contract_system_prompt(
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

pub(super) fn build_cleanup_legacy_prompt(
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
