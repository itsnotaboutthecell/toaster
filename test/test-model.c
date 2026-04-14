#include "toaster.h"

#include <stdio.h>
#include <string.h>

static int failures = 0;
#define PASS(name) printf("  PASS: %s\n", name)
#define FAIL(name, msg)                                                                                    \
  do {                                                                                                     \
    printf("  FAIL: %s — %s\n", name, msg);                                                               \
    failures++;                                                                                            \
  } while (0)

static void test_catalog_count(void)
{
  size_t count = toaster_model_catalog_count();
  if (count >= 5)
    PASS("catalog_count >= 5");
  else
    FAIL("catalog_count", "expected at least 5 models");
}

static void test_catalog_get(void)
{
  toaster_model_info_t info;
  if (!toaster_model_catalog_get(0, &info)) {
    FAIL("catalog_get(0)", "failed to get first model");
    return;
  }
  if (info.id && strlen(info.id) > 0)
    PASS("catalog_get(0) has id");
  else
    FAIL("catalog_get(0)", "missing id");

  if (info.name && strlen(info.name) > 0)
    PASS("catalog_get(0) has name");
  else
    FAIL("catalog_get(0)", "missing name");

  if (info.filename && strlen(info.filename) > 0)
    PASS("catalog_get(0) has filename");
  else
    FAIL("catalog_get(0)", "missing filename");

  if (info.size_mb > 0)
    PASS("catalog_get(0) has size");
  else
    FAIL("catalog_get(0)", "size_mb is 0");

  if (!toaster_model_catalog_get(9999, &info))
    PASS("catalog_get(9999) returns false");
  else
    FAIL("catalog_get(9999)", "should fail for out-of-range index");
}

static void test_catalog_find(void)
{
  toaster_model_info_t info;

  if (toaster_model_catalog_find("tiny.en", &info))
    PASS("catalog_find(tiny.en)");
  else
    FAIL("catalog_find(tiny.en)", "not found");

  if (strcmp(info.id, "tiny.en") == 0)
    PASS("catalog_find id matches");
  else
    FAIL("catalog_find", "id mismatch");

  if (info.language_count == 1)
    PASS("tiny.en has 1 language");
  else
    FAIL("tiny.en languages", "expected 1 language for english-only model");

  if (toaster_model_catalog_find("turbo", &info))
    PASS("catalog_find(turbo)");
  else
    FAIL("catalog_find(turbo)", "not found");

  if (info.supports_translation)
    PASS("turbo supports translation");
  else
    FAIL("turbo", "should support translation");

  if (!toaster_model_catalog_find("nonexistent", NULL))
    PASS("catalog_find(nonexistent) returns false");
  else
    FAIL("catalog_find(nonexistent)", "should fail");

  if (!toaster_model_catalog_find(NULL, &info))
    PASS("catalog_find(NULL) returns false");
  else
    FAIL("catalog_find(NULL)", "should fail");
}

static void test_active_model(void)
{
  const char *active = toaster_model_get_active();
  if (active && strcmp(active, "tiny.en") == 0)
    PASS("default active model is tiny.en");
  else
    FAIL("default active", "expected tiny.en");

  if (toaster_model_set_active("turbo"))
    PASS("set_active(turbo)");
  else
    FAIL("set_active(turbo)", "failed");

  active = toaster_model_get_active();
  if (active && strcmp(active, "turbo") == 0)
    PASS("active model is now turbo");
  else
    FAIL("active after set", "expected turbo");

  if (!toaster_model_set_active("nonexistent"))
    PASS("set_active(nonexistent) fails");
  else
    FAIL("set_active(nonexistent)", "should fail for unknown model");

  /* Active should remain turbo after failed set */
  active = toaster_model_get_active();
  if (active && strcmp(active, "turbo") == 0)
    PASS("active unchanged after failed set");
  else
    FAIL("active after failed set", "should still be turbo");

  /* Reset to default */
  toaster_model_set_active("tiny.en");
}

static void test_model_directory(void)
{
  const char *dir = toaster_model_get_directory();
  if (dir && strlen(dir) > 0)
    PASS("model directory not empty");
  else
    FAIL("model directory", "empty or null");

  if (toaster_model_set_directory("C:\\test\\models"))
    PASS("set_directory succeeds");
  else
    FAIL("set_directory", "failed");

  dir = toaster_model_get_directory();
  if (dir && strcmp(dir, "C:\\test\\models") == 0)
    PASS("directory updated correctly");
  else
    FAIL("directory after set", "mismatch");

  if (!toaster_model_set_directory(NULL))
    PASS("set_directory(NULL) fails");
  else
    FAIL("set_directory(NULL)", "should fail");

  if (!toaster_model_set_directory(""))
    PASS("set_directory empty fails");
  else
    FAIL("set_directory empty", "should fail");
}

static void test_recommended_models(void)
{
  size_t i, count, recommended = 0;
  toaster_model_info_t info;

  count = toaster_model_catalog_count();
  for (i = 0; i < count; i++) {
    if (toaster_model_catalog_get(i, &info) && info.is_recommended)
      recommended++;
  }

  if (recommended >= 1)
    PASS("at least one recommended model");
  else
    FAIL("recommended models", "expected at least 1");
}

static void test_word_confidence_speaker(void)
{
  toaster_transcript_t *t = toaster_transcript_create();
  toaster_word_t word;

  if (!t) {
    FAIL("word_confidence_speaker", "failed to create transcript");
    return;
  }

  toaster_transcript_add_word(t, "hello", 0, 500000);
  toaster_transcript_get_word(t, 0, &word);

  if (word.confidence < 0.0f)
    PASS("default confidence is negative (unset)");
  else
    FAIL("default confidence", "expected negative default");

  if (word.speaker_id == -1)
    PASS("default speaker_id is -1");
  else
    FAIL("default speaker_id", "expected -1");

  toaster_transcript_set_word_confidence(t, 0, 0.95f);
  toaster_transcript_set_word_speaker(t, 0, 2);

  toaster_transcript_get_word(t, 0, &word);

  if (word.confidence > 0.94f && word.confidence < 0.96f)
    PASS("confidence set to 0.95");
  else
    FAIL("confidence", "expected ~0.95");

  if (word.speaker_id == 2)
    PASS("speaker_id set to 2");
  else
    FAIL("speaker_id", "expected 2");

  toaster_transcript_destroy(t);
}

int main(void)
{
  printf("test-model: model catalog and word extensions\n");
  toaster_startup();

  test_catalog_count();
  test_catalog_get();
  test_catalog_find();
  test_active_model();
  test_model_directory();
  test_recommended_models();
  test_word_confidence_speaker();

  toaster_shutdown();
  printf("test-model: %s (%d failure%s)\n", failures ? "FAILED" : "ALL PASSED", failures,
         failures == 1 ? "" : "s");
  return failures ? 1 : 0;
}
