#include "toaster.h"

#include <ctype.h>
#include <stdio.h>
#include <limits.h>
#include <stdlib.h>
#include <string.h>

typedef struct toaster_word_storage {
  char *text;
  int64_t start_us;
  int64_t end_us;
  bool deleted;
  bool silenced;
} toaster_word_storage_t;

typedef enum toaster_word_state_kind {
  TOASTER_WORD_STATE_DELETED = 0,
  TOASTER_WORD_STATE_SILENCED = 1
} toaster_word_state_kind_t;

struct toaster_transcript {
  toaster_word_storage_t *words;
  size_t word_count;
  size_t word_capacity;
  toaster_time_range_t *cut_spans;
  size_t cut_count;
  size_t cut_capacity;
};

static int g_startup_depth = 0;
static const char *g_version = "0.2.0-alpha";

static char *toaster_strdup(const char *text)
{
  size_t length;
  char *copy;

  if (!text)
    return NULL;

  length = strlen(text) + 1;
  copy = (char *)malloc(length);
  if (!copy)
    return NULL;

  memcpy(copy, text, length);
  return copy;
}

static bool replace_string(char **target, const char *text)
{
  char *copy = toaster_strdup(text ? text : "");

  if (!copy)
    return false;

  free(*target);
  *target = copy;
  return true;
}

static bool ensure_word_capacity(toaster_transcript_t *transcript, size_t desired_count)
{
  toaster_word_storage_t *new_words;
  size_t new_capacity;

  if (!transcript)
    return false;

  if (desired_count <= transcript->word_capacity)
    return true;

  new_capacity = transcript->word_capacity ? transcript->word_capacity * 2 : 8;
  while (new_capacity < desired_count)
    new_capacity *= 2;

  new_words = (toaster_word_storage_t *)realloc(transcript->words,
                                                new_capacity * sizeof(toaster_word_storage_t));
  if (!new_words)
    return false;

  transcript->words = new_words;
  transcript->word_capacity = new_capacity;
  return true;
}

static bool ensure_cut_capacity(toaster_transcript_t *transcript, size_t desired_count)
{
  toaster_time_range_t *new_spans;
  size_t new_capacity;

  if (!transcript)
    return false;

  if (desired_count <= transcript->cut_capacity)
    return true;

  new_capacity = transcript->cut_capacity ? transcript->cut_capacity * 2 : 8;
  while (new_capacity < desired_count)
    new_capacity *= 2;

  new_spans = (toaster_time_range_t *)realloc(transcript->cut_spans,
                                              new_capacity * sizeof(toaster_time_range_t));
  if (!new_spans)
    return false;

  transcript->cut_spans = new_spans;
  transcript->cut_capacity = new_capacity;
  return true;
}

static bool word_matches_state(const toaster_word_storage_t *word, toaster_word_state_kind_t state)
{
  if (!word)
    return false;

  return state == TOASTER_WORD_STATE_DELETED ? word->deleted : word->silenced;
}

static size_t count_word_ranges(const toaster_transcript_t *transcript, toaster_word_state_kind_t state)
{
  size_t count = 0;
  size_t index;

  if (!transcript)
    return 0;

  for (index = 0; index < transcript->word_count; ++index) {
    if (!word_matches_state(&transcript->words[index], state))
      continue;

    if (index == 0 || !word_matches_state(&transcript->words[index - 1], state))
      ++count;
  }

  return count;
}

static bool get_word_range_at(const toaster_transcript_t *transcript, toaster_word_state_kind_t state,
                              size_t target_index, toaster_time_range_t *out_range)
{
  size_t current_index = 0;
  size_t range_start = 0;
  bool in_range = false;
  size_t index;

  if (!transcript || !out_range)
    return false;

  for (index = 0; index < transcript->word_count; ++index) {
    bool matches = word_matches_state(&transcript->words[index], state);

    if (matches && !in_range) {
      in_range = true;
      range_start = index;
    }

    if (!matches && in_range) {
      if (current_index == target_index) {
        out_range->start_us = transcript->words[range_start].start_us;
        out_range->end_us = transcript->words[index - 1].end_us;
        return true;
      }

      in_range = false;
      ++current_index;
    }
  }

  if (in_range && current_index == target_index) {
    out_range->start_us = transcript->words[range_start].start_us;
    out_range->end_us = transcript->words[transcript->word_count - 1].end_us;
    return true;
  }

  return false;
}

static size_t append_word_ranges(const toaster_transcript_t *transcript, toaster_word_state_kind_t state,
                                 toaster_time_range_t *out_ranges, size_t offset)
{
  size_t range_start = 0;
  bool in_range = false;
  size_t index;
  size_t count = offset;

  if (!transcript || !out_ranges)
    return offset;

  for (index = 0; index < transcript->word_count; ++index) {
    bool matches = word_matches_state(&transcript->words[index], state);

    if (matches && !in_range) {
      in_range = true;
      range_start = index;
    }

    if (!matches && in_range) {
      out_ranges[count].start_us = transcript->words[range_start].start_us;
      out_ranges[count].end_us = transcript->words[index - 1].end_us;
      ++count;
      in_range = false;
    }
  }

  if (in_range) {
    out_ranges[count].start_us = transcript->words[range_start].start_us;
    out_ranges[count].end_us = transcript->words[transcript->word_count - 1].end_us;
    ++count;
  }

  return count;
}

static int compare_ranges(const void *left, const void *right)
{
  const toaster_time_range_t *lhs = (const toaster_time_range_t *)left;
  const toaster_time_range_t *rhs = (const toaster_time_range_t *)right;

  if (lhs->start_us < rhs->start_us)
    return -1;
  if (lhs->start_us > rhs->start_us)
    return 1;
  if (lhs->end_us < rhs->end_us)
    return -1;
  if (lhs->end_us > rhs->end_us)
    return 1;
  return 0;
}

static size_t collect_excluded_spans(const toaster_transcript_t *transcript,
                                     toaster_time_range_t **out_spans)
{
  toaster_time_range_t *spans;
  size_t deleted_count;
  size_t total_count;
  size_t fill_count;
  size_t index;
  size_t merged_count = 0;

  if (out_spans)
    *out_spans = NULL;

  if (!transcript || !out_spans)
    return 0;

  deleted_count = count_word_ranges(transcript, TOASTER_WORD_STATE_DELETED);
  total_count = deleted_count + transcript->cut_count;
  if (total_count == 0)
    return 0;

  spans = (toaster_time_range_t *)calloc(total_count, sizeof(toaster_time_range_t));
  if (!spans)
    return 0;

  fill_count = append_word_ranges(transcript, TOASTER_WORD_STATE_DELETED, spans, 0);
  for (index = 0; index < transcript->cut_count; ++index) {
    if (transcript->cut_spans[index].end_us <= transcript->cut_spans[index].start_us)
      continue;

    spans[fill_count++] = transcript->cut_spans[index];
  }

  if (fill_count == 0) {
    free(spans);
    return 0;
  }

  qsort(spans, fill_count, sizeof(toaster_time_range_t), compare_ranges);

  for (index = 0; index < fill_count; ++index) {
    toaster_time_range_t current = spans[index];

    if (current.end_us <= current.start_us)
      continue;

    if (merged_count == 0) {
      spans[merged_count++] = current;
      continue;
    }

    if (current.start_us <= spans[merged_count - 1].end_us) {
      if (current.end_us > spans[merged_count - 1].end_us)
        spans[merged_count - 1].end_us = current.end_us;
      continue;
    }

    spans[merged_count++] = current;
  }

  if (merged_count == 0) {
    free(spans);
    return 0;
  }

  *out_spans = spans;
  return merged_count;
}

static bool export_word_has_text(const toaster_word_storage_t *word)
{
  return word && word->text && word->text[0] != '\0';
}

static bool export_word_is_audible(const toaster_word_storage_t *word)
{
  return export_word_has_text(word) && !word->deleted && !word->silenced;
}

static bool export_word_attaches_to_previous(const char *text)
{
  unsigned char first;

  if (!text || text[0] == '\0')
    return false;

  first = (unsigned char)text[0];
  return ispunct(first) != 0;
}

static bool ensure_text_capacity(char **buffer, size_t *capacity, size_t desired_size)
{
  char *resized;
  size_t new_capacity;

  if (!buffer || !capacity)
    return false;

  if (desired_size <= *capacity)
    return true;

  new_capacity = *capacity ? *capacity * 2 : 64;
  while (new_capacity < desired_size)
    new_capacity *= 2;

  resized = (char *)realloc(*buffer, new_capacity);
  if (!resized)
    return false;

  *buffer = resized;
  *capacity = new_capacity;
  return true;
}

static bool append_export_text(char **buffer, size_t *length, size_t *capacity, const char *text,
                               bool insert_space)
{
  size_t text_length;
  size_t desired_size;

  if (!buffer || !length || !capacity || !text)
    return false;

  text_length = strlen(text);
  desired_size = *length + (insert_space ? 1 : 0) + text_length + 1;
  if (!ensure_text_capacity(buffer, capacity, desired_size))
    return false;

  if (insert_space)
    (*buffer)[(*length)++] = ' ';

  memcpy(*buffer + *length, text, text_length);
  *length += text_length;
  (*buffer)[*length] = '\0';
  return true;
}

static int64_t map_source_time_to_output(const toaster_time_range_t *excluded_spans,
                                         size_t excluded_count, int64_t source_us)
{
  int64_t removed_duration = 0;
  size_t index;

  for (index = 0; index < excluded_count; ++index) {
    int64_t start_us = excluded_spans[index].start_us;
    int64_t end_us = excluded_spans[index].end_us;

    if (source_us >= end_us) {
      removed_duration += end_us - start_us;
      continue;
    }

    if (source_us > start_us)
      removed_duration += source_us - start_us;
    break;
  }

  return source_us - removed_duration;
}

static bool write_caption_timestamp(FILE *file, int64_t value_us, char decimal_separator)
{
  int64_t total_ms;
  int64_t hours;
  int64_t minutes;
  int64_t seconds;
  int64_t milliseconds;

  if (!file)
    return false;

  total_ms = value_us / 1000;
  if (total_ms < 0)
    total_ms = 0;

  hours = total_ms / 3600000;
  minutes = (total_ms / 60000) % 60;
  seconds = (total_ms / 1000) % 60;
  milliseconds = total_ms % 1000;

  return fprintf(file, "%02lld:%02lld:%02lld%c%03lld",
                 (long long)hours,
                 (long long)minutes,
                 (long long)seconds,
                 decimal_separator,
                 (long long)milliseconds) >= 0;
}

static bool write_caption_cue(FILE *file, toaster_caption_format_t format, size_t cue_index,
                              int64_t start_us, int64_t end_us, const char *text)
{
  char decimal_separator;

  if (!file || !text)
    return false;

  decimal_separator = format == TOASTER_CAPTION_FORMAT_VTT ? '.' : ',';
  if (fprintf(file, "%llu\n", (unsigned long long)cue_index) < 0)
    return false;
  if (!write_caption_timestamp(file, start_us, decimal_separator))
    return false;
  if (fprintf(file, " --> ") < 0)
    return false;
  if (!write_caption_timestamp(file, end_us, decimal_separator))
    return false;
  return fprintf(file, "\n%s\n\n", text) >= 0;
}

bool toaster_startup(void)
{
  ++g_startup_depth;
  return true;
}

void toaster_shutdown(void)
{
  if (g_startup_depth > 0)
    --g_startup_depth;
}

bool toaster_is_started(void)
{
  return g_startup_depth > 0;
}

const char *toaster_get_version(void)
{
  return g_version;
}

toaster_transcript_t *toaster_transcript_create(void)
{
  return (toaster_transcript_t *)calloc(1, sizeof(toaster_transcript_t));
}

void toaster_transcript_destroy(toaster_transcript_t *transcript)
{
  size_t index;

  if (!transcript)
    return;

  for (index = 0; index < transcript->word_count; ++index)
    free(transcript->words[index].text);

  free(transcript->words);
  free(transcript->cut_spans);
  free(transcript);
}

bool toaster_transcript_clear(toaster_transcript_t *transcript)
{
  size_t index;

  if (!transcript)
    return false;

  for (index = 0; index < transcript->word_count; ++index)
    free(transcript->words[index].text);

  transcript->word_count = 0;
  transcript->cut_count = 0;
  return true;
}

bool toaster_transcript_add_word(toaster_transcript_t *transcript, const char *text, int64_t start_us,
                                 int64_t end_us)
{
  char *copy;
  toaster_word_storage_t *word;

  if (!transcript || !text || start_us > end_us)
    return false;

  if (!ensure_word_capacity(transcript, transcript->word_count + 1))
    return false;

  copy = toaster_strdup(text);
  if (!copy)
    return false;

  word = &transcript->words[transcript->word_count++];
  word->text = copy;
  word->start_us = start_us;
  word->end_us = end_us;
  word->deleted = false;
  word->silenced = false;
  return true;
}

size_t toaster_transcript_word_count(const toaster_transcript_t *transcript)
{
  return transcript ? transcript->word_count : 0;
}

bool toaster_transcript_get_word(const toaster_transcript_t *transcript, size_t index,
                                 toaster_word_t *out_word)
{
  if (!transcript || !out_word || index >= transcript->word_count)
    return false;

  out_word->text = transcript->words[index].text;
  out_word->start_us = transcript->words[index].start_us;
  out_word->end_us = transcript->words[index].end_us;
  out_word->deleted = transcript->words[index].deleted;
  out_word->silenced = transcript->words[index].silenced;
  return true;
}

bool toaster_transcript_set_word_text(toaster_transcript_t *transcript, size_t index, const char *text)
{
  if (!transcript || index >= transcript->word_count || !text)
    return false;

  return replace_string(&transcript->words[index].text, text);
}

bool toaster_transcript_set_word_times(toaster_transcript_t *transcript, size_t index, int64_t start_us,
                                       int64_t end_us)
{
  if (!transcript || index >= transcript->word_count || start_us > end_us)
    return false;

  transcript->words[index].start_us = start_us;
  transcript->words[index].end_us = end_us;
  return true;
}

bool toaster_transcript_delete_range(toaster_transcript_t *transcript, size_t start_index,
                                     size_t end_index)
{
  size_t index;

  if (!transcript || start_index > end_index || end_index >= transcript->word_count)
    return false;

  for (index = start_index; index <= end_index; ++index)
    transcript->words[index].deleted = true;

  return true;
}

bool toaster_transcript_silence_range(toaster_transcript_t *transcript, size_t start_index,
                                      size_t end_index)
{
  size_t index;

  if (!transcript || start_index > end_index || end_index >= transcript->word_count)
    return false;

  for (index = start_index; index <= end_index; ++index)
    transcript->words[index].silenced = true;

  return true;
}

bool toaster_transcript_unsilence_range(toaster_transcript_t *transcript, size_t start_index,
                                        size_t end_index)
{
  size_t index;

  if (!transcript || start_index > end_index || end_index >= transcript->word_count)
    return false;

  for (index = start_index; index <= end_index; ++index)
    transcript->words[index].silenced = false;

  return true;
}

bool toaster_transcript_restore_range(toaster_transcript_t *transcript, size_t start_index,
                                      size_t end_index)
{
  size_t index;

  if (!transcript || start_index > end_index || end_index >= transcript->word_count)
    return false;

  for (index = start_index; index <= end_index; ++index)
    transcript->words[index].deleted = false;

  return true;
}

bool toaster_transcript_restore_all(toaster_transcript_t *transcript)
{
  size_t index;

  if (!transcript)
    return false;

  for (index = 0; index < transcript->word_count; ++index) {
    transcript->words[index].deleted = false;
    transcript->words[index].silenced = false;
  }

  transcript->cut_count = 0;
  return true;
}

size_t toaster_transcript_deleted_span_count(const toaster_transcript_t *transcript)
{
  return count_word_ranges(transcript, TOASTER_WORD_STATE_DELETED);
}

bool toaster_transcript_get_deleted_span(const toaster_transcript_t *transcript, size_t span_index,
                                         toaster_time_range_t *out_range)
{
  return get_word_range_at(transcript, TOASTER_WORD_STATE_DELETED, span_index, out_range);
}

size_t toaster_transcript_silenced_span_count(const toaster_transcript_t *transcript)
{
  return count_word_ranges(transcript, TOASTER_WORD_STATE_SILENCED);
}

bool toaster_transcript_get_silenced_span(const toaster_transcript_t *transcript, size_t span_index,
                                          toaster_time_range_t *out_range)
{
  return get_word_range_at(transcript, TOASTER_WORD_STATE_SILENCED, span_index, out_range);
}

bool toaster_transcript_add_cut_span(toaster_transcript_t *transcript, int64_t start_us, int64_t end_us)
{
  if (!transcript || start_us >= end_us)
    return false;

  if (!ensure_cut_capacity(transcript, transcript->cut_count + 1))
    return false;

  transcript->cut_spans[transcript->cut_count].start_us = start_us;
  transcript->cut_spans[transcript->cut_count].end_us = end_us;
  ++transcript->cut_count;
  return true;
}

bool toaster_transcript_clear_cut_spans(toaster_transcript_t *transcript)
{
  if (!transcript)
    return false;

  transcript->cut_count = 0;
  return true;
}

size_t toaster_transcript_cut_span_count(const toaster_transcript_t *transcript)
{
  return transcript ? transcript->cut_count : 0;
}

bool toaster_transcript_get_cut_span(const toaster_transcript_t *transcript, size_t span_index,
                                     toaster_time_range_t *out_range)
{
  if (!transcript || !out_range || span_index >= transcript->cut_count)
    return false;

  *out_range = transcript->cut_spans[span_index];
  return true;
}

bool toaster_transcript_get_bounds(const toaster_transcript_t *transcript, toaster_time_range_t *out_range)
{
  size_t index;
  int64_t start_us = INT64_MAX;
  int64_t end_us = INT64_MIN;

  if (!transcript || !out_range || transcript->word_count == 0)
    return false;

  for (index = 0; index < transcript->word_count; ++index) {
    if (transcript->words[index].start_us < start_us)
      start_us = transcript->words[index].start_us;
    if (transcript->words[index].end_us > end_us)
      end_us = transcript->words[index].end_us;
  }

  out_range->start_us = start_us;
  out_range->end_us = end_us;
  return true;
}

size_t toaster_transcript_keep_segment_count(const toaster_transcript_t *transcript)
{
  toaster_time_range_t bounds;
  toaster_time_range_t *excluded_spans = NULL;
  size_t excluded_count;
  size_t count = 0;
  int64_t cursor;
  size_t index;

  if (!toaster_transcript_get_bounds(transcript, &bounds))
    return 0;

  excluded_count = collect_excluded_spans(transcript, &excluded_spans);
  if (excluded_count == 0)
    return 1;

  cursor = bounds.start_us;
  for (index = 0; index < excluded_count; ++index) {
    int64_t start_us = excluded_spans[index].start_us;
    int64_t end_us = excluded_spans[index].end_us;

    if (end_us <= bounds.start_us || start_us >= bounds.end_us)
      continue;

    if (start_us < bounds.start_us)
      start_us = bounds.start_us;
    if (end_us > bounds.end_us)
      end_us = bounds.end_us;

    if (start_us > cursor)
      ++count;

    if (end_us > cursor)
      cursor = end_us;
  }

  if (cursor < bounds.end_us)
    ++count;

  free(excluded_spans);
  return count;
}

bool toaster_transcript_get_keep_segment(const toaster_transcript_t *transcript, size_t segment_index,
                                         toaster_time_range_t *out_range)
{
  toaster_time_range_t bounds;
  toaster_time_range_t *excluded_spans = NULL;
  size_t excluded_count;
  size_t current_index = 0;
  int64_t cursor;
  size_t index;

  if (!out_range || !toaster_transcript_get_bounds(transcript, &bounds))
    return false;

  excluded_count = collect_excluded_spans(transcript, &excluded_spans);
  if (excluded_count == 0) {
    if (segment_index == 0) {
      *out_range = bounds;
      return true;
    }
    return false;
  }

  cursor = bounds.start_us;
  for (index = 0; index < excluded_count; ++index) {
    int64_t start_us = excluded_spans[index].start_us;
    int64_t end_us = excluded_spans[index].end_us;

    if (end_us <= bounds.start_us || start_us >= bounds.end_us)
      continue;

    if (start_us < bounds.start_us)
      start_us = bounds.start_us;
    if (end_us > bounds.end_us)
      end_us = bounds.end_us;

    if (start_us > cursor) {
      if (current_index == segment_index) {
        out_range->start_us = cursor;
        out_range->end_us = start_us;
        free(excluded_spans);
        return true;
      }
      ++current_index;
    }

    if (end_us > cursor)
      cursor = end_us;
  }

  if (cursor < bounds.end_us && current_index == segment_index) {
    out_range->start_us = cursor;
    out_range->end_us = bounds.end_us;
    free(excluded_spans);
    return true;
  }

  free(excluded_spans);
  return false;
}

bool toaster_transcript_export_script(const toaster_transcript_t *transcript, const char *path)
{
  FILE *file;
  bool success = true;
  bool wrote_word = false;
  size_t index;

  if (!transcript || !path || path[0] == '\0')
    return false;

  file = fopen(path, "wb");
  if (!file)
    return false;

  for (index = 0; index < transcript->word_count; ++index) {
    const toaster_word_storage_t *word = &transcript->words[index];

    if (!export_word_is_audible(word))
      continue;

    if (wrote_word && !export_word_attaches_to_previous(word->text) && fputc(' ', file) == EOF) {
      success = false;
      break;
    }
    if (fputs(word->text, file) == EOF) {
      success = false;
      break;
    }

    wrote_word = true;
  }

  if (success && wrote_word && fputc('\n', file) == EOF)
    success = false;
  if (fclose(file) != 0)
    success = false;

  return success;
}

bool toaster_transcript_export_captions(const toaster_transcript_t *transcript, const char *path,
                                        toaster_caption_format_t format)
{
  static const int64_t max_cue_duration_us = 4000000;
  static const int64_t cue_break_gap_us = 1000000;
  static const size_t max_words_per_cue = 8;

  FILE *file;
  toaster_time_range_t *excluded_spans = NULL;
  char *cue_text = NULL;
  size_t cue_length = 0;
  size_t cue_capacity = 0;
  size_t cue_word_count = 0;
  size_t cue_index = 1;
  size_t excluded_count;
  size_t index;
  int64_t cue_start_us = 0;
  int64_t cue_end_us = 0;
  bool cue_active = false;
  bool success = true;

  if (!transcript || !path || path[0] == '\0')
    return false;
  if (format != TOASTER_CAPTION_FORMAT_SRT && format != TOASTER_CAPTION_FORMAT_VTT)
    return false;

  file = fopen(path, "wb");
  if (!file)
    return false;

  if (format == TOASTER_CAPTION_FORMAT_VTT && fputs("WEBVTT\n\n", file) == EOF)
    success = false;

  excluded_count = collect_excluded_spans(transcript, &excluded_spans);
  for (index = 0; success && index < transcript->word_count; ++index) {
    const toaster_word_storage_t *word = &transcript->words[index];
    int64_t mapped_start_us;
    int64_t mapped_end_us;
    bool insert_space;

    if (!export_word_is_audible(word))
      continue;

    mapped_start_us = map_source_time_to_output(excluded_spans, excluded_count, word->start_us);
    mapped_end_us = map_source_time_to_output(excluded_spans, excluded_count, word->end_us);
    if (mapped_end_us <= mapped_start_us)
      continue;

    if (!cue_active) {
      cue_active = true;
      cue_start_us = mapped_start_us;
      cue_end_us = mapped_end_us;
      cue_length = 0;
      cue_word_count = 0;
      if (cue_text)
        cue_text[0] = '\0';
    } else if (mapped_start_us - cue_end_us >= cue_break_gap_us ||
               mapped_end_us - cue_start_us > max_cue_duration_us ||
               cue_word_count >= max_words_per_cue) {
      if (!write_caption_cue(file, format, cue_index++, cue_start_us, cue_end_us,
                             cue_text ? cue_text : "")) {
        success = false;
        break;
      }

      cue_start_us = mapped_start_us;
      cue_end_us = mapped_end_us;
      cue_length = 0;
      cue_word_count = 0;
      if (cue_text)
        cue_text[0] = '\0';
    } else if (mapped_end_us > cue_end_us) {
      cue_end_us = mapped_end_us;
    }

    insert_space = cue_word_count > 0 && !export_word_attaches_to_previous(word->text);
    if (!append_export_text(&cue_text, &cue_length, &cue_capacity, word->text, insert_space)) {
      success = false;
      break;
    }

    ++cue_word_count;
    if (mapped_end_us > cue_end_us)
      cue_end_us = mapped_end_us;
  }

  if (success && cue_active && cue_word_count > 0)
    success = write_caption_cue(file, format, cue_index, cue_start_us, cue_end_us,
                                cue_text ? cue_text : "");

  free(excluded_spans);
  free(cue_text);
  if (fclose(file) != 0)
    success = false;

  return success;
}
