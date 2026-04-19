//! Tauri command surface for the LLM transcript cleanup (post-processor).
//!
//! Thin wrapper over `managers::cleanup::process_transcription_output`, which
//! until now had no caller (see the module-level TODO in
//! `managers/cleanup/mod.rs`). This command is the frontend's entry point:
//! after a transcription finishes, the Editor invokes `cleanup_transcription`
//! if `settings.post_process_enabled` is true, and — on success — replaces the
//! displayed transcript text with the cleaned version. Per-word timestamps
//! continue to come from the ASR result; the cleanup contract
//! (preserve_language / no_reorder / no_paraphrase / protected_tokens_preserved)
//! keeps ordinal word mapping intact so the editor can overlay cleaned text
//! without realignment.
//!
//! Error semantics: `Ok(None)` on every "no-op" path — feature disabled, no
//! provider/model/prompt selected, endpoint unreachable, contract violation.
//! `Err(String)` is reserved for actual bugs (serialization failures etc.) and
//! is not currently produced by the inner pipeline. The frontend should treat
//! `None` as "leave transcript as-is, optionally surface a subtle info toast".

use crate::managers::cleanup::process_transcription_output;
use crate::settings;
use tauri::AppHandle;

#[tauri::command]
#[specta::specta]
pub async fn cleanup_transcription(
    app: AppHandle,
    transcription: String,
) -> Result<Option<String>, String> {
    let settings = settings::get_settings(&app);
    if !settings.post_process_enabled {
        return Ok(None);
    }
    let processed = process_transcription_output(&app, &transcription, true).await;
    Ok(processed.post_processed_text)
}
