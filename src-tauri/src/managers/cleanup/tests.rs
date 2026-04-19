//! Extracted from the inline mod tests block (monolith-split).

use super::*;

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

    let error = validation.expect_err("validation should fail when protected tokens are missing");
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

#[test]
fn cleanup_prompt_includes_custom_words() {
    // Custom_words from the Allow list must flow into the contract system prompt
    // as protected tokens so the LLM does not rewrite them. The transcript itself
    // provides no digit/symbol tokens, so the whole list must come from settings.
    let protected = super::dedupe_tokens(&[
        "Toaster".to_string(),
        "Tauri".to_string(),
    ]);
    let prompt = super::prompts::build_cleanup_contract_system_prompt(
        "",
        &protected,
        &[],
    );
    assert!(
        prompt.contains("Toaster"),
        "prompt must include custom Allow word 'Toaster', got: {prompt}"
    );
    assert!(
        prompt.contains("Tauri"),
        "prompt must include custom Allow word 'Tauri', got: {prompt}"
    );
    assert!(
        !prompt.contains("none detected"),
        "non-empty Allow list must not render as 'none detected'"
    );
}

#[test]
fn cleanup_prompt_uses_custom_filler_words() {
    // Discard Words list ("custom_filler_words") must flow verbatim into the
    // cleanup prompt; no other hardcoded filler list may leak through.
    let fillers = vec![
        "um".to_string(),
        "uh".to_string(),
        "like".to_string(),
    ];
    let prompt = super::prompts::build_cleanup_contract_system_prompt(
        "",
        &[],
        &fillers,
    );
    for filler in &fillers {
        assert!(
            prompt.contains(filler),
            "prompt must include configured filler {filler}, got: {prompt}"
        );
    }

    // User prompt template containing the placeholder should have it expanded.
    let user_template = "Please remove: ${filler_words}\nTranscript:\n${output}";
    let legacy = super::prompts::build_cleanup_legacy_prompt(
        user_template,
        "hello world",
        &[],
        &fillers,
    );
    let expected_filler_line = format!("Please remove: {}", fillers.join(", "));
    assert!(
        legacy.contains(&expected_filler_line),
        "legacy prompt must substitute ${{filler_words}} placeholder, got: {legacy}"
    );
    assert!(
        !legacy.contains("${filler_words}"),
        "legacy prompt must not leave raw placeholder, got: {legacy}"
    );
}

#[test]
fn cleanup_prompt_omits_fillers_when_list_empty() {
    // Empty Discard Words list means NO filler-removal clause reaches the LLM.
    // There is no hardcoded fallback — "respect what the UI shows".
    let prompt = super::prompts::build_cleanup_contract_system_prompt(
        "",
        &[],
        &[],
    );
    assert!(
        !prompt.to_lowercase().contains("filler"),
        "empty filler list must not produce a filler clause, got: {prompt}"
    );

    // Placeholder in a user prompt template must still be substituted,
    // rendered as "none" so the LLM never sees the raw placeholder.
    let user_template = "Remove: ${filler_words}\nTranscript:\n${output}";
    let legacy = super::prompts::build_cleanup_legacy_prompt(
        user_template,
        "hello",
        &[],
        &[],
    );
    assert!(
        legacy.contains("Remove: none"),
        "empty filler list must substitute placeholder with 'none', got: {legacy}"
    );
    assert!(
        !legacy.contains("${filler_words}"),
        "empty filler list must not leave raw placeholder, got: {legacy}"
    );
}

#[test]
fn protected_tokens_from_settings_sanitizes_input() {
    // The frontend AllowWords sanitizer strips <>"'& before saving; the backend
    // mirrors it as defence-in-depth so adversarial saved settings can't break
    // prompt formatting.
    let mut settings = crate::settings::get_default_settings();
    settings.custom_words = vec![
        "Toaster".to_string(),
        "<script>".to_string(),
        "evil\"word".to_string(),
        "   ".to_string(),
    ];
    let tokens = super::protected_tokens_from_settings(&settings);
    assert!(tokens.contains(&"Toaster".to_string()));
    assert!(tokens.contains(&"script".to_string()));
    assert!(tokens.contains(&"evilword".to_string()));
    assert!(
        !tokens.iter().any(|t| t.is_empty()),
        "empty/whitespace tokens must be dropped"
    );
    for token in &tokens {
        for bad in ['<', '>', '"', '\'', '&'] {
            assert!(
                !token.contains(bad),
                "sanitized token {token:?} still contains forbidden char {bad:?}"
            );
        }
    }
}
