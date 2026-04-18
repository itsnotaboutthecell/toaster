//! Local-LLM apply regression tests (extracted from editor/tests/basic.rs).

use super::super::*;
use super::common::make_words;

#[test]
fn local_llm_apply_preserves_mapping_and_timing_metadata() {
    let mut editor = EditorState::new();
    let mut words = make_words();
    words[4].deleted = true;
    editor.set_words(words);

    let before_non_text: Vec<(i64, i64, bool, bool, i32)> = editor
        .get_words()
        .iter()
        .map(|word| {
            (
                word.start_us,
                word.end_us,
                word.deleted,
                word.silenced,
                word.speaker_id,
            )
        })
        .collect();
    let before_keep_segments = editor.get_keep_segments();
    let probe_times = [0_i64, 500_000, 1_250_000, 2_750_000, 3_900_000];
    let before_mapped: Vec<i64> = probe_times
        .iter()
        .map(|time| editor.map_edit_time_to_source_time(*time))
        .collect();

    let result = editor.apply_local_llm_word_proposals(&[
        LocalLlmWordProposal {
            start_word_index: 0,
            end_word_index: 2,
            replacement_words: vec!["Hey".into(), "planet".into()],
        },
        LocalLlmWordProposal {
            start_word_index: 2,
            end_word_index: 4,
            replacement_words: vec!["this".into(), "exists".into()],
        },
    ]);

    assert_eq!(result.applied_proposals, 2);
    assert!(result.rejected_proposals.is_empty());
    assert_eq!(result.applied_word_indices, vec![0, 1, 3]);
    assert_eq!(editor.get_words()[0].text, "Hey");
    assert_eq!(editor.get_words()[1].text, "planet");
    assert_eq!(editor.get_words()[2].text, "this");
    assert_eq!(editor.get_words()[3].text, "exists");

    for (idx, word) in editor.get_words().iter().enumerate() {
        let (start_us, end_us, deleted, silenced, speaker_id) = before_non_text[idx];
        assert_eq!(word.start_us, start_us);
        assert_eq!(word.end_us, end_us);
        assert_eq!(word.deleted, deleted);
        assert_eq!(word.silenced, silenced);
        assert_eq!(word.speaker_id, speaker_id);
    }

    assert_eq!(editor.get_keep_segments(), before_keep_segments);
    let after_mapped: Vec<i64> = probe_times
        .iter()
        .map(|time| editor.map_edit_time_to_source_time(*time))
        .collect();
    assert_eq!(after_mapped, before_mapped);
}

#[test]
fn local_llm_apply_rejects_beginning_word_deletion_without_precision_drift() {
    let mut editor = EditorState::new();
    let mut words = make_words();
    words[0].text = "Hello,".into();
    words[1].text = "world!".into();
    editor.set_words(words);

    let before_words = editor.get_words().to_vec();
    let before_revision = editor.timing_contract_snapshot().timeline_revision;
    let before_keep_segments = editor.get_keep_segments();
    let probe_times = [0_i64, 750_000, 2_500_000, 4_250_000];
    let before_mapped: Vec<i64> = probe_times
        .iter()
        .map(|time| editor.map_edit_time_to_source_time(*time))
        .collect();

    let result = editor.apply_local_llm_word_proposals(&[LocalLlmWordProposal {
        start_word_index: 0,
        end_word_index: 2,
        replacement_words: vec!["world!".into()],
    }]);

    assert_eq!(result.applied_proposals, 0);
    assert!(result.applied_word_indices.is_empty());
    assert_eq!(result.rejected_proposals.len(), 1);
    assert!(result.rejected_proposals[0]
        .reason
        .contains("count mismatch"));
    assert_eq!(
        editor.timing_contract_snapshot().timeline_revision,
        before_revision
    );

    for (before, after) in before_words.iter().zip(editor.get_words().iter()) {
        assert_eq!(before.text, after.text);
        assert_eq!(before.start_us, after.start_us);
        assert_eq!(before.end_us, after.end_us);
        assert_eq!(before.deleted, after.deleted);
        assert_eq!(before.silenced, after.silenced);
        assert_eq!(before.confidence, after.confidence);
        assert_eq!(before.speaker_id, after.speaker_id);
    }
    assert_eq!(editor.get_keep_segments(), before_keep_segments);
    let after_mapped: Vec<i64> = probe_times
        .iter()
        .map(|time| editor.map_edit_time_to_source_time(*time))
        .collect();
    assert_eq!(after_mapped, before_mapped);
    assert!(
        !editor.undo(),
        "Rejected proposal must not push an undo snapshot"
    );
}

#[test]
fn local_llm_apply_handles_punctuation_adjacent_edits_without_timing_drift() {
    let mut editor = EditorState::new();
    let mut words = make_words();
    words[0].text = "Helo,".into();
    words[1].text = "wrld!".into();
    editor.set_words(words);

    let before_non_text: Vec<(i64, i64, bool, bool, f32, i32)> = editor
        .get_words()
        .iter()
        .map(|word| {
            (
                word.start_us,
                word.end_us,
                word.deleted,
                word.silenced,
                word.confidence,
                word.speaker_id,
            )
        })
        .collect();
    let before_keep_segments = editor.get_keep_segments();
    let probe_times = [0_i64, 500_000, 1_999_999, 2_000_000, 3_500_000];
    let before_mapped: Vec<i64> = probe_times
        .iter()
        .map(|time| editor.map_edit_time_to_source_time(*time))
        .collect();

    let result = editor.apply_local_llm_word_proposals(&[
        LocalLlmWordProposal {
            start_word_index: 0,
            end_word_index: 1,
            replacement_words: vec!["Hello,".into()],
        },
        LocalLlmWordProposal {
            start_word_index: 1,
            end_word_index: 2,
            replacement_words: vec!["world!".into()],
        },
    ]);

    assert_eq!(result.applied_proposals, 2);
    assert_eq!(result.applied_word_indices, vec![0, 1]);
    assert!(result.rejected_proposals.is_empty());
    assert_eq!(editor.get_words()[0].text, "Hello,");
    assert_eq!(editor.get_words()[1].text, "world!");

    for (idx, word) in editor.get_words().iter().enumerate() {
        let (start_us, end_us, deleted, silenced, confidence, speaker_id) = before_non_text[idx];
        assert_eq!(word.start_us, start_us);
        assert_eq!(word.end_us, end_us);
        assert_eq!(word.deleted, deleted);
        assert_eq!(word.silenced, silenced);
        assert_eq!(word.confidence, confidence);
        assert_eq!(word.speaker_id, speaker_id);
    }

    assert_eq!(editor.get_keep_segments(), before_keep_segments);
    let after_mapped: Vec<i64> = probe_times
        .iter()
        .map(|time| editor.map_edit_time_to_source_time(*time))
        .collect();
    assert_eq!(after_mapped, before_mapped);
}

#[test]
fn local_llm_apply_delete_undo_redo_parity_preserves_backend_mapping() {
    let mut editor = EditorState::new();
    let mut words = make_words();
    words[1].text = "wrld!".into();
    editor.set_words(words);

    assert!(editor.delete_word(0));
    let keep_segments_after_delete = editor.get_keep_segments();
    let probe_times = [0_i64, 500_000, 2_000_000, 3_750_000];
    let mapped_after_delete: Vec<i64> = probe_times
        .iter()
        .map(|time| editor.map_edit_time_to_source_time(*time))
        .collect();
    assert_eq!(editor.map_edit_time_to_source_time(0), 1_000_000);

    let apply_result = editor.apply_local_llm_word_proposals(&[LocalLlmWordProposal {
        start_word_index: 1,
        end_word_index: 2,
        replacement_words: vec!["world!".into()],
    }]);
    assert_eq!(apply_result.applied_proposals, 1);
    assert_eq!(apply_result.applied_word_indices, vec![1]);
    assert!(apply_result.rejected_proposals.is_empty());
    assert!(editor.get_words()[0].deleted);
    assert_eq!(editor.get_words()[1].text, "world!");
    assert_eq!(editor.get_keep_segments(), keep_segments_after_delete);
    let mapped_after_apply: Vec<i64> = probe_times
        .iter()
        .map(|time| editor.map_edit_time_to_source_time(*time))
        .collect();
    assert_eq!(mapped_after_apply, mapped_after_delete);

    assert!(editor.undo(), "first undo should revert LLM text edits");
    assert!(editor.get_words()[0].deleted);
    assert_eq!(editor.get_words()[1].text, "wrld!");
    assert_eq!(editor.get_keep_segments(), keep_segments_after_delete);
    let mapped_after_first_undo: Vec<i64> = probe_times
        .iter()
        .map(|time| editor.map_edit_time_to_source_time(*time))
        .collect();
    assert_eq!(mapped_after_first_undo, mapped_after_delete);

    assert!(
        editor.undo(),
        "second undo should revert the delete mutation"
    );
    assert!(!editor.get_words()[0].deleted);
    assert_eq!(editor.map_edit_time_to_source_time(0), 0);

    assert!(editor.redo(), "redo should restore the delete mutation");
    assert!(editor.get_words()[0].deleted);
    assert_eq!(editor.map_edit_time_to_source_time(0), 1_000_000);

    assert!(editor.redo(), "second redo should restore LLM text edits");
    assert!(editor.get_words()[0].deleted);
    assert_eq!(editor.get_words()[1].text, "world!");
    assert_eq!(editor.get_keep_segments(), keep_segments_after_delete);
    let mapped_after_second_redo: Vec<i64> = probe_times
        .iter()
        .map(|time| editor.map_edit_time_to_source_time(*time))
        .collect();
    assert_eq!(mapped_after_second_redo, mapped_after_delete);
}

#[test]
fn local_llm_apply_rejects_invalid_proposals_without_destructive_side_effects() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());

    let before_words = editor.get_words().to_vec();
    let before_revision = editor.timing_contract_snapshot().timeline_revision;

    let result = editor.apply_local_llm_word_proposals(&[
        LocalLlmWordProposal {
            start_word_index: 1,
            end_word_index: 3,
            replacement_words: vec!["single".into()],
        },
        LocalLlmWordProposal {
            start_word_index: 5,
            end_word_index: 9,
            replacement_words: vec!["test".into(), "words".into(), "here".into(), "x".into()],
        },
        LocalLlmWordProposal {
            start_word_index: 4,
            end_word_index: 4,
            replacement_words: vec![],
        },
    ]);

    assert_eq!(result.applied_proposals, 0);
    assert!(result.applied_word_indices.is_empty());
    assert_eq!(result.rejected_proposals.len(), 3);
    assert!(result
        .rejected_proposals
        .iter()
        .any(|rejection| rejection.reason.contains("count mismatch")));
    assert!(result
        .rejected_proposals
        .iter()
        .any(|rejection| rejection.reason.contains("out of bounds")));
    assert!(result
        .rejected_proposals
        .iter()
        .any(|rejection| rejection.reason.contains("start must be < end")));

    for (before, after) in before_words.iter().zip(editor.get_words().iter()) {
        assert_eq!(before.text, after.text);
        assert_eq!(before.start_us, after.start_us);
        assert_eq!(before.end_us, after.end_us);
        assert_eq!(before.deleted, after.deleted);
        assert_eq!(before.silenced, after.silenced);
        assert_eq!(before.confidence, after.confidence);
        assert_eq!(before.speaker_id, after.speaker_id);
    }
    assert_eq!(
        editor.timing_contract_snapshot().timeline_revision,
        before_revision
    );
}

#[test]
fn local_llm_apply_supports_partial_success_with_overlap_rejection() {
    let mut editor = EditorState::new();
    editor.set_words(make_words());

    let result = editor.apply_local_llm_word_proposals(&[
        LocalLlmWordProposal {
            start_word_index: 0,
            end_word_index: 2,
            replacement_words: vec!["Hi".into(), "earth".into()],
        },
        LocalLlmWordProposal {
            start_word_index: 1,
            end_word_index: 3,
            replacement_words: vec!["middle".into(), "words".into()],
        },
    ]);

    assert_eq!(result.applied_proposals, 1);
    assert_eq!(result.applied_word_indices, vec![0, 1]);
    assert_eq!(result.rejected_proposals.len(), 1);
    assert!(result.rejected_proposals[0]
        .reason
        .contains("overlaps with another accepted proposal"));
    assert_eq!(editor.get_words()[0].text, "Hi");
    assert_eq!(editor.get_words()[1].text, "earth");
    assert_eq!(editor.get_words()[2].text, "this");
}
