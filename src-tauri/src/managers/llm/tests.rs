//! Unit tests for `LlmManager`. See PRD R-005 / R-006.
//!
//! These tests use the `MockBackend` to avoid requiring a real GGUF file.
//! They also use `FixedRamProbe` / `FixedFreeSpace` to isolate the tests
//! from the host machine.

use super::*;
use std::sync::Arc;
use tempfile::TempDir;

fn default_model_id() -> String {
    let default_entry = catalog::catalog()
        .into_iter()
        .find(|e| e.is_recommended_default)
        .expect("catalog must have a default entry");
    default_entry.id
}

fn write_fake_gguf(llm_dir: &std::path::Path, id: &str) {
    std::fs::create_dir_all(llm_dir).unwrap();
    let path = llm_dir.join(format!("{}.gguf", id));
    std::fs::write(&path, b"fake-gguf-for-tests").unwrap();
}

fn build_manager_with_ram(ram_bytes: u64) -> (TempDir, LlmManager) {
    let tmp = TempDir::new().unwrap();
    let llm_dir = tmp.path().join("llm");
    let mgr = LlmManager::with_probes(llm_dir.clone(), Arc::new(FixedRamProbe(ram_bytes))).unwrap();
    (tmp, mgr)
}

#[tokio::test]
async fn llm_manager_lazy_loads_on_first_complete() {
    let (_tmp, mgr) = build_manager_with_ram(64 * 1024 * 1024 * 1024);
    let id = default_model_id();
    write_fake_gguf(mgr.llm_dir(), &id);

    assert!(
        !mgr.is_loaded(),
        "fresh manager must not have a model loaded"
    );

    // Swap in a mock backend directly — the real feature-gated loader
    // returns a NullBackend in the default build.
    let mock = Arc::new(std::sync::Mutex::new(inference::MockBackend::with_response(
        "cleaned-output",
    )));
    mgr.install_backend_for_tests(&id, mock.clone());

    let response = mgr
        .complete(
            &id,
            CompletionRequest {
                system_prompt: "s".into(),
                user_prompt: "u".into(),
                json_schema: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(response.content, "cleaned-output");
    assert!(mgr.is_loaded(), "manager should be loaded after complete()");
}

#[tokio::test]
async fn llm_manager_unloads_after_timeout() {
    let (_tmp, mgr) = build_manager_with_ram(64 * 1024 * 1024 * 1024);
    let id = default_model_id();
    write_fake_gguf(mgr.llm_dir(), &id);

    let mock = Arc::new(std::sync::Mutex::new(inference::MockBackend::with_response(
        "x",
    )));
    mgr.install_backend_for_tests(&id, mock);
    // Force last_used back in time: we call maybe_unload with a 0-second
    // timeout so the idle check evaluates to true immediately.
    mgr.maybe_unload(std::time::Duration::from_secs(0));
    assert!(!mgr.is_loaded(), "manager must unload after idle timeout");

    // Idempotency: calling again when nothing is loaded is a no-op.
    mgr.maybe_unload(std::time::Duration::from_secs(0));
    assert!(!mgr.is_loaded());
}

#[tokio::test]
async fn llm_manager_errors_cleanly_when_model_missing() {
    let (_tmp, mgr) = build_manager_with_ram(64 * 1024 * 1024 * 1024);
    let id = default_model_id();
    // No fake gguf file on disk.
    let result = mgr
        .complete(
            &id,
            CompletionRequest {
                system_prompt: "s".into(),
                user_prompt: "u".into(),
                json_schema: None,
            },
        )
        .await;
    let err = result.expect_err("complete() must error when file is missing");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("not downloaded")
            || msg.to_lowercase().contains("does not exist"),
        "error should mention missing file, got: {}",
        msg
    );
}

#[tokio::test]
async fn llm_manager_errors_when_ram_insufficient() {
    // 128 MiB — below the smallest catalog entry's recommended_ram_gb (2 GiB).
    let (_tmp, mgr) = build_manager_with_ram(128 * 1024 * 1024);
    // Use the smallest model so we're not limited by which entries exist.
    let small_entry = catalog::catalog()
        .into_iter()
        .min_by_key(|e| e.recommended_ram_gb)
        .unwrap();
    write_fake_gguf(mgr.llm_dir(), &small_entry.id);

    let result = mgr
        .complete(
            &small_entry.id,
            CompletionRequest {
                system_prompt: "s".into(),
                user_prompt: "u".into(),
                json_schema: None,
            },
        )
        .await;
    let err = result.expect_err("complete() must error on insufficient RAM");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("insufficient ram"),
        "error should mention RAM, got: {}",
        msg
    );
}

#[test]
fn llm_manager_unknown_model_id_errors() {
    let (_tmp, mgr) = build_manager_with_ram(64 * 1024 * 1024 * 1024);
    let err = mgr.ensure_loaded("not-in-catalog").expect_err("must error");
    assert!(err.to_string().contains("Unknown"));
}
