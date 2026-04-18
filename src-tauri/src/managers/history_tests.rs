//! Extracted from the inline `mod tests` block (monolith-split).

use super::*;

use rusqlite::{params, Connection};

fn setup_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(
        "CREATE TABLE transcription_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_name TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            saved BOOLEAN NOT NULL DEFAULT 0,
            title TEXT NOT NULL,
            transcription_text TEXT NOT NULL,
            post_processed_text TEXT,
            post_process_prompt TEXT,
            post_process_requested BOOLEAN NOT NULL DEFAULT 0
        );",
    )
    .expect("create transcription_history table");
    conn
}

fn insert_entry(conn: &Connection, timestamp: i64, text: &str, post_processed: Option<&str>) {
    conn.execute(
        "INSERT INTO transcription_history (
            file_name,
            timestamp,
            saved,
            title,
            transcription_text,
            post_processed_text,
            post_process_prompt,
            post_process_requested
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            format!("handy-{}.wav", timestamp),
            timestamp,
            false,
            format!("Recording {}", timestamp),
            text,
            post_processed,
            Option::<String>::None,
            false,
        ],
    )
    .expect("insert history entry");
}

#[test]
fn get_latest_entry_returns_none_when_empty() {
    let conn = setup_conn();
    let entry = HistoryManager::get_latest_entry_with_conn(&conn).expect("fetch latest entry");
    assert!(entry.is_none());
}

#[test]
fn get_latest_entry_returns_newest_entry() {
    let conn = setup_conn();
    insert_entry(&conn, 100, "first", None);
    insert_entry(&conn, 200, "second", Some("processed"));

    let entry = HistoryManager::get_latest_entry_with_conn(&conn)
        .expect("fetch latest entry")
        .expect("entry exists");

    assert_eq!(entry.timestamp, 200);
    assert_eq!(entry.transcription_text, "second");
    assert_eq!(entry.post_processed_text.as_deref(), Some("processed"));
}

#[test]
fn get_latest_completed_entry_skips_empty_entries() {
    let conn = setup_conn();
    insert_entry(&conn, 100, "completed", None);
    insert_entry(&conn, 200, "", None);

    let entry = HistoryManager::get_latest_completed_entry_with_conn(&conn)
        .expect("fetch latest completed entry")
        .expect("completed entry exists");

    assert_eq!(entry.timestamp, 100);
    assert_eq!(entry.transcription_text, "completed");
}

#[test]
fn get_latest_completed_entry_returns_none_when_all_empty() {
    let conn = setup_conn();
    insert_entry(&conn, 100, "", None);
    insert_entry(&conn, 200, "", None);

    let entry = HistoryManager::get_latest_completed_entry_with_conn(&conn)
        .expect("fetch latest completed entry");
    assert!(entry.is_none());
}

#[test]
fn get_latest_completed_entry_returns_newest_completed() {
    let conn = setup_conn();
    insert_entry(&conn, 100, "old completed", None);
    insert_entry(&conn, 200, "", None);
    insert_entry(&conn, 300, "newest completed", Some("post-processed"));

    let entry = HistoryManager::get_latest_completed_entry_with_conn(&conn)
        .expect("fetch latest completed entry")
        .expect("entry exists");

    assert_eq!(entry.timestamp, 300);
    assert_eq!(entry.transcription_text, "newest completed");
    assert_eq!(entry.post_processed_text.as_deref(), Some("post-processed"));
}

#[test]
fn save_entry_creates_and_returns_entry() {
    let conn = setup_conn();

    let entry = HistoryManager::save_entry_with_conn(
        &conn,
        "test-recording.wav",
        "hello world",
        false,
        None,
        None,
    )
    .expect("save entry");

    assert_eq!(entry.file_name, "test-recording.wav");
    assert_eq!(entry.transcription_text, "hello world");
    assert!(!entry.saved);
    assert!(!entry.post_process_requested);
    assert!(entry.post_processed_text.is_none());
    assert!(entry.id > 0);

    // Verify it persisted in the database
    let fetched = HistoryManager::get_latest_entry_with_conn(&conn)
        .expect("fetch")
        .expect("entry exists");
    assert_eq!(fetched.id, entry.id);
    assert_eq!(fetched.transcription_text, "hello world");
}

#[test]
fn save_entry_with_post_processing() {
    let conn = setup_conn();

    let entry = HistoryManager::save_entry_with_conn(
        &conn,
        "recording.wav",
        "raw transcript",
        true,
        Some("cleaned transcript"),
        Some("fix grammar"),
    )
    .expect("save entry with post-processing");

    assert_eq!(entry.transcription_text, "raw transcript");
    assert!(entry.post_process_requested);
    assert_eq!(
        entry.post_processed_text.as_deref(),
        Some("cleaned transcript")
    );
    assert_eq!(entry.post_process_prompt.as_deref(), Some("fix grammar"));
}

#[test]
fn save_entry_assigns_unique_ids() {
    let conn = setup_conn();

    let e1 = HistoryManager::save_entry_with_conn(&conn, "a.wav", "first", false, None, None)
        .expect("save first");
    let e2 = HistoryManager::save_entry_with_conn(&conn, "b.wav", "second", false, None, None)
        .expect("save second");

    assert_ne!(e1.id, e2.id);
    assert!(e2.id > e1.id);
}

#[test]
fn update_transcription_modifies_existing_entry() {
    let conn = setup_conn();
    insert_entry(&conn, 100, "original text", None);

    // Get the id of the inserted entry
    let original = HistoryManager::get_latest_entry_with_conn(&conn)
        .expect("fetch")
        .expect("entry exists");

    let updated = HistoryManager::update_transcription_with_conn(
        &conn,
        original.id,
        "updated text",
        Some("post-processed result"),
        Some("summarize"),
    )
    .expect("update transcription");

    assert_eq!(updated.id, original.id);
    assert_eq!(updated.transcription_text, "updated text");
    assert_eq!(
        updated.post_processed_text.as_deref(),
        Some("post-processed result")
    );
    assert_eq!(updated.post_process_prompt.as_deref(), Some("summarize"));
    // Unchanged fields should be preserved
    assert_eq!(updated.file_name, original.file_name);
    assert_eq!(updated.timestamp, original.timestamp);
}

#[test]
fn update_transcription_nonexistent_entry_returns_error() {
    let conn = setup_conn();

    let result = HistoryManager::update_transcription_with_conn(&conn, 9999, "text", None, None);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found"),
        "Expected 'not found' in error message, got: {}",
        err_msg
    );
}

#[test]
fn update_transcription_clears_optional_fields() {
    let conn = setup_conn();
    insert_entry(&conn, 100, "text", Some("old post-processed"));

    let original = HistoryManager::get_latest_entry_with_conn(&conn)
        .expect("fetch")
        .expect("entry exists");
    assert_eq!(
        original.post_processed_text.as_deref(),
        Some("old post-processed")
    );

    let updated =
        HistoryManager::update_transcription_with_conn(&conn, original.id, "new text", None, None)
            .expect("update transcription");

    assert_eq!(updated.transcription_text, "new text");
    assert!(updated.post_processed_text.is_none());
    assert!(updated.post_process_prompt.is_none());
}

#[test]
fn cleanup_by_count_removes_oldest_unsaved() {
    let conn = setup_conn();
    insert_entry(&conn, 100, "oldest", None);
    insert_entry(&conn, 200, "middle", None);
    insert_entry(&conn, 300, "newest", None);

    let deleted = HistoryManager::cleanup_by_count_with_conn(&conn, 2).expect("cleanup");
    assert_eq!(deleted, 1);

    // Verify only 2 entries remain (the newest ones)
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transcription_history", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(count, 2);

    // The oldest entry should be gone
    let oldest: Option<i64> = conn
        .query_row(
            "SELECT id FROM transcription_history WHERE timestamp = 100",
            [],
            |row| row.get(0),
        )
        .optional()
        .expect("query");
    assert!(oldest.is_none(), "oldest entry should have been deleted");
}

#[test]
fn cleanup_by_count_preserves_saved_entries() {
    let conn = setup_conn();
    insert_entry(&conn, 100, "old saved", None);
    // Mark the first entry as saved
    conn.execute(
        "UPDATE transcription_history SET saved = 1 WHERE timestamp = 100",
        [],
    )
    .expect("mark as saved");
    insert_entry(&conn, 200, "unsaved", None);
    insert_entry(&conn, 300, "unsaved newer", None);

    // Cleanup with limit=1: should only remove unsaved entries beyond limit
    let deleted = HistoryManager::cleanup_by_count_with_conn(&conn, 1).expect("cleanup");
    assert_eq!(deleted, 1);

    // Saved entry should still exist
    let saved_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM transcription_history WHERE timestamp = 100 AND saved = 1",
            [],
            |row| row.get(0),
        )
        .expect("check saved");
    assert!(saved_exists, "saved entry should be preserved");

    // Total entries: 1 saved + 1 unsaved (newest) = 2
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transcription_history", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(count, 2);
}

#[test]
fn cleanup_by_count_no_op_when_under_limit() {
    let conn = setup_conn();
    insert_entry(&conn, 100, "first", None);
    insert_entry(&conn, 200, "second", None);

    let deleted = HistoryManager::cleanup_by_count_with_conn(&conn, 5).expect("cleanup");
    assert_eq!(deleted, 0);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transcription_history", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(count, 2);
}

#[test]
fn cleanup_by_count_removes_all_when_limit_zero() {
    let conn = setup_conn();
    insert_entry(&conn, 100, "first", None);
    insert_entry(&conn, 200, "second", None);
    insert_entry(&conn, 300, "third", None);

    let deleted = HistoryManager::cleanup_by_count_with_conn(&conn, 0).expect("cleanup");
    assert_eq!(deleted, 3);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transcription_history", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(count, 0);
}
