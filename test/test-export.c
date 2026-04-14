#include <stdio.h>
#include <stdlib.h>
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

static char *read_text_file(const char *path)
{
  FILE *file;
  long length;
  size_t bytes_read;
  char *buffer;

  file = fopen(path, "rb");
  if (!file)
    return NULL;

  if (fseek(file, 0, SEEK_END) != 0) {
    fclose(file);
    return NULL;
  }

  length = ftell(file);
  if (length < 0) {
    fclose(file);
    return NULL;
  }

  if (fseek(file, 0, SEEK_SET) != 0) {
    fclose(file);
    return NULL;
  }

  buffer = (char *)calloc((size_t)length + 1, sizeof(char));
  if (!buffer) {
    fclose(file);
    return NULL;
  }

  bytes_read = fread(buffer, 1, (size_t)length, file);
  fclose(file);
  if (bytes_read != (size_t)length) {
    free(buffer);
    return NULL;
  }

  buffer[length] = '\0';
  return buffer;
}

int main(void)
{
  const char *script_path = "test-export-script.txt";
  const char *srt_path = "test-export.srt";
  const char *vtt_path = "test-export.vtt";
  const char *expected_script = "Hello brave world\n";
  const char *expected_srt =
    "1\n"
    "00:00:00,000 --> 00:00:01,500\n"
    "Hello brave world\n\n";
  const char *expected_vtt =
    "WEBVTT\n\n"
    "1\n"
    "00:00:00.000 --> 00:00:01.500\n"
    "Hello brave world\n\n";
  toaster_transcript_t *transcript;
  char *script_text = NULL;
  char *srt_text = NULL;
  char *vtt_text = NULL;

  toaster_startup();

  transcript = toaster_transcript_create();
  expect_true("create transcript", transcript != NULL, "transcript should allocate");
  expect_true("add hello", toaster_transcript_add_word(transcript, "Hello", 0, 500000),
              "word should append");
  expect_true("add filler", toaster_transcript_add_word(transcript, "um", 500000, 700000),
              "word should append");
  expect_true("add brave", toaster_transcript_add_word(transcript, "brave", 700000, 1000000),
              "word should append");
  expect_true("add deleted", toaster_transcript_add_word(transcript, "old", 1000000, 1300000),
              "word should append");
  expect_true("add world", toaster_transcript_add_word(transcript, "world", 1500000, 2000000),
              "word should append");
  expect_true("silence filler", toaster_transcript_silence_range(transcript, 1, 1),
              "silence should succeed");
  expect_true("delete old", toaster_transcript_delete_range(transcript, 3, 3),
              "delete should succeed");
  expect_true("add pause cut", toaster_transcript_add_cut_span(transcript, 1300000, 1500000),
              "cut span should append");

  expect_true("export script", toaster_transcript_export_script(transcript, script_path),
              "script export should succeed");
  expect_true("export srt",
              toaster_transcript_export_captions(transcript, srt_path, TOASTER_CAPTION_FORMAT_SRT),
              "SRT export should succeed");
  expect_true("export vtt",
              toaster_transcript_export_captions(transcript, vtt_path, TOASTER_CAPTION_FORMAT_VTT),
              "VTT export should succeed");

  script_text = read_text_file(script_path);
  srt_text = read_text_file(srt_path);
  vtt_text = read_text_file(vtt_path);
  expect_true("read script output", script_text != NULL, "script output should load");
  expect_true("read srt output", srt_text != NULL, "SRT output should load");
  expect_true("read vtt output", vtt_text != NULL, "VTT output should load");
  expect_true("script content", script_text && strcmp(script_text, expected_script) == 0,
              "script export should omit silenced and deleted words");
  expect_true("srt content", srt_text && strcmp(srt_text, expected_srt) == 0,
              "SRT export should use edited timeline");
  expect_true("vtt content", vtt_text && strcmp(vtt_text, expected_vtt) == 0,
              "VTT export should use edited timeline");

  free(script_text);
  free(srt_text);
  free(vtt_text);
  toaster_transcript_destroy(transcript);
  remove(script_path);
  remove(srt_path);
  remove(vtt_path);
  toaster_shutdown();

  return failures ? 1 : 0;
}
