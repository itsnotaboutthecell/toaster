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
