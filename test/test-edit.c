#include <stdio.h>
#include <string.h>

#include "toaster.h"

static int failures = 0;

#define PASS(name) printf("  PASS: %s\n", name)
#define FAIL(name, msg)                                                              \
  do {                                                                               \
    printf("  FAIL: %s - %s\n", name, msg);                                          \
    failures++;                                                                      \
  } while (0)

static void expect_true(const char *name, bool condition, const char *message)
{
  if (condition)
    PASS(name);
  else
    FAIL(name, message);
}

int main(void)
{
  toaster_transcript_t *transcript;
  toaster_word_t word;
  toaster_time_range_t range;

  toaster_startup();

  transcript = toaster_transcript_create();
  expect_true("create transcript", transcript != NULL, "transcript should allocate");

  expect_true("startup active", toaster_is_started(), "startup flag should be active");
  expect_true("add word hello",
              toaster_transcript_add_word(transcript, "hello", 0, 450000),
              "first word should append");
  expect_true("add word um",
              toaster_transcript_add_word(transcript, "um", 450000, 620000),
              "second word should append");
  expect_true("add word welcome",
              toaster_transcript_add_word(transcript, "welcome", 620000, 1200000),
              "third word should append");
  expect_true("word count",
              toaster_transcript_word_count(transcript) == 3,
              "transcript should expose three words");

  expect_true("delete middle word",
              toaster_transcript_delete_range(transcript, 1, 1),
              "delete should succeed");
  expect_true("read deleted word",
              toaster_transcript_get_word(transcript, 1, &word),
              "deleted word should still be readable");
  expect_true("deleted flag", word.deleted, "deleted word should be marked");
  expect_true("word text", strcmp(word.text, "um") == 0, "word text should stay intact");

  expect_true("deleted span count",
              toaster_transcript_deleted_span_count(transcript) == 1,
              "one deleted span expected");
  expect_true("deleted span range",
              toaster_transcript_get_deleted_span(transcript, 0, &range) &&
                range.start_us == 450000 && range.end_us == 620000,
              "deleted span timestamps should match deleted word");

  expect_true("keep segment count",
              toaster_transcript_keep_segment_count(transcript) == 2,
              "deleting one word should create two keep segments");
  expect_true("first keep segment",
              toaster_transcript_get_keep_segment(transcript, 0, &range) &&
                range.start_us == 0 && range.end_us == 450000,
              "first keep segment should end before filler");
  expect_true("second keep segment",
              toaster_transcript_get_keep_segment(transcript, 1, &range) &&
                range.start_us == 620000 && range.end_us == 1200000,
              "second keep segment should resume after filler");

  expect_true("restore all",
              toaster_transcript_restore_all(transcript),
              "restore all should succeed");
  expect_true("single keep segment after restore",
              toaster_transcript_keep_segment_count(transcript) == 1 &&
                toaster_transcript_get_keep_segment(transcript, 0, &range) &&
                range.start_us == 0 && range.end_us == 1200000,
              "restore should recover one continuous segment");

  toaster_transcript_destroy(transcript);

  /* ---- Undo/Redo tests ---- */
  transcript = toaster_transcript_create();
  toaster_transcript_add_word(transcript, "alpha", 0, 100000);
  toaster_transcript_add_word(transcript, "beta", 100000, 200000);
  toaster_transcript_add_word(transcript, "gamma", 200000, 300000);

  expect_true("undo — no history",
              !toaster_transcript_can_undo(transcript),
              "fresh transcript should have nothing to undo");

  toaster_transcript_save_snapshot(transcript);
  toaster_transcript_delete_range(transcript, 1, 1);
  toaster_transcript_get_word(transcript, 1, &word);
  expect_true("undo — deleted before undo", word.deleted,
              "beta should be deleted");
  expect_true("undo — can undo after edit",
              toaster_transcript_can_undo(transcript),
              "should be able to undo after snapshot+edit");

  expect_true("undo — undo succeeds",
              toaster_transcript_undo(transcript),
              "undo should succeed");
  toaster_transcript_get_word(transcript, 1, &word);
  expect_true("undo — restored after undo", !word.deleted,
              "beta should be restored after undo");
  expect_true("undo — can redo after undo",
              toaster_transcript_can_redo(transcript),
              "should be able to redo after undo");

  expect_true("redo — redo succeeds",
              toaster_transcript_redo(transcript),
              "redo should succeed");
  toaster_transcript_get_word(transcript, 1, &word);
  expect_true("redo — deleted after redo", word.deleted,
              "beta should be deleted again after redo");

  /* Undo again, then make a new edit to discard redo stack */
  toaster_transcript_undo(transcript);
  toaster_transcript_save_snapshot(transcript);
  toaster_transcript_silence_range(transcript, 0, 0);
  expect_true("undo — no redo after new edit",
              !toaster_transcript_can_redo(transcript),
              "new edit after undo should clear redo stack");

  toaster_transcript_destroy(transcript);

  /* ---- Split word tests ---- */
  transcript = toaster_transcript_create();
  toaster_transcript_add_word(transcript, "together", 0, 1000000);
  toaster_transcript_add_word(transcript, "again", 1000000, 2000000);

  expect_true("split — initial word count",
              toaster_transcript_word_count(transcript) == 2,
              "should start with 2 words");

  expect_true("split — split succeeds",
              toaster_transcript_split_word(transcript, 0, 500000),
              "split at midpoint should succeed");
  expect_true("split — word count after split",
              toaster_transcript_word_count(transcript) == 3,
              "should have 3 words after split");

  toaster_transcript_get_word(transcript, 0, &word);
  expect_true("split — first half text",
              strcmp(word.text, "toge") == 0,
              "first half should get first half of text");
  expect_true("split — first half end",
              word.end_us == 500000,
              "first half should end at split point");

  toaster_transcript_get_word(transcript, 1, &word);
  expect_true("split — second half text",
              strcmp(word.text, "ther") == 0,
              "second half should get second half of text");
  expect_true("split — second half start",
              word.start_us == 500000,
              "second half should start at split point");
  expect_true("split — second half end",
              word.end_us == 1000000,
              "second half should end at original end");

  toaster_transcript_get_word(transcript, 2, &word);
  expect_true("split — shifted word text",
              strcmp(word.text, "again") == 0,
              "third word should be 'again' shifted from index 1 to 2");

  /* Edge case: split outside word boundaries should fail */
  expect_true("split — before word fails",
              !toaster_transcript_split_word(transcript, 2, 500000),
              "split before word start should fail");
  expect_true("split — at word start fails",
              !toaster_transcript_split_word(transcript, 2, 1000000),
              "split at exact start should fail");

  toaster_transcript_destroy(transcript);

  toaster_shutdown();

  return failures ? 1 : 0;
}
