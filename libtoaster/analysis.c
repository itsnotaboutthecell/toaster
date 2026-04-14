#include "toaster.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct toaster_suggestion_storage {
  toaster_suggestion_t suggestion;
  char *reason_storage;
} toaster_suggestion_storage_t;

struct toaster_suggestion_list {
  toaster_suggestion_storage_t *items;
  size_t count;
  size_t capacity;
};

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

static bool ensure_suggestion_capacity(toaster_suggestion_list_t *list, size_t desired_count)
{
  toaster_suggestion_storage_t *new_items;
  size_t new_capacity;

  if (!list)
    return false;

  if (desired_count <= list->capacity)
    return true;

  new_capacity = list->capacity ? list->capacity * 2 : 8;
  while (new_capacity < desired_count)
    new_capacity *= 2;

  new_items = (toaster_suggestion_storage_t *)realloc(
    list->items, new_capacity * sizeof(toaster_suggestion_storage_t));
  if (!new_items)
    return false;

  list->items = new_items;
  list->capacity = new_capacity;
  return true;
}

static bool text_equals_ci(const char *left, const char *right)
{
  unsigned char lhs;
  unsigned char rhs;

  if (!left || !right)
    return false;

  while (*left && *right) {
    lhs = (unsigned char)tolower((unsigned char)*left);
    rhs = (unsigned char)tolower((unsigned char)*right);
    if (lhs != rhs)
      return false;
    ++left;
    ++right;
  }

  return *left == '\0' && *right == '\0';
}

static bool is_single_filler(const char *word)
{
  static const char *filler_words[] = {
    "um", "uh", "erm", "hmm", "like", "actually", "basically", "literally", "well", "so"
  };
  size_t index;

  if (!word || !*word)
    return false;

  for (index = 0; index < sizeof(filler_words) / sizeof(filler_words[0]); ++index) {
    if (text_equals_ci(word, filler_words[index]))
      return true;
  }

  return false;
}

static bool is_in_word_list(const char *word, const char *const *list, size_t count)
{
  size_t index;

  if (!word || !list)
    return false;

  for (index = 0; index < count; ++index) {
    if (list[index] && text_equals_ci(word, list[index]))
      return true;
  }

  return false;
}

static bool is_single_filler_custom(const char *word, const char *const *extra_fillers,
                                    size_t extra_count, const char *const *ignore_words,
                                    size_t ignore_count)
{
  if (is_in_word_list(word, ignore_words, ignore_count))
    return false;

  if (is_single_filler(word))
    return true;

  return is_in_word_list(word, extra_fillers, extra_count);
}

static bool is_phrase_filler(const char *first, const char *second)
{
  static const char *phrases[][2] = {
    {"you", "know"},
    {"kind", "of"},
    {"sort", "of"},
    {"i", "mean"},
  };
  size_t index;

  if (!first || !second)
    return false;

  for (index = 0; index < sizeof(phrases) / sizeof(phrases[0]); ++index) {
    if (text_equals_ci(first, phrases[index][0]) && text_equals_ci(second, phrases[index][1]))
      return true;
  }

  return false;
}

static bool suggestion_list_push(toaster_suggestion_list_t *list, toaster_suggestion_kind_t kind,
                                 size_t start_index, size_t end_index, int64_t start_us,
                                 int64_t end_us, int64_t replacement_duration_us,
                                 const char *reason)
{
  toaster_suggestion_storage_t *slot;

  if (!list || !reason)
    return false;

  if (!ensure_suggestion_capacity(list, list->count + 1))
    return false;

  slot = &list->items[list->count++];
  memset(slot, 0, sizeof(*slot));

  slot->reason_storage = toaster_strdup(reason);
  if (!slot->reason_storage) {
    --list->count;
    return false;
  }

  slot->suggestion.kind = kind;
  slot->suggestion.start_index = start_index;
  slot->suggestion.end_index = end_index;
  slot->suggestion.start_us = start_us;
  slot->suggestion.end_us = end_us;
  slot->suggestion.replacement_duration_us = replacement_duration_us;
  slot->suggestion.reason = slot->reason_storage;
  return true;
}

static bool pause_already_cut(const toaster_transcript_t *transcript, int64_t cut_start_us,
                              int64_t cut_end_us)
{
  size_t index;

  for (index = 0; index < toaster_transcript_cut_span_count(transcript); ++index) {
    toaster_time_range_t range;

    if (!toaster_transcript_get_cut_span(transcript, index, &range))
      continue;

    if (range.start_us <= cut_start_us && range.end_us >= cut_end_us)
      return true;
  }

  return false;
}

toaster_suggestion_list_t *toaster_suggestion_list_create(void)
{
  return (toaster_suggestion_list_t *)calloc(1, sizeof(toaster_suggestion_list_t));
}

void toaster_suggestion_list_destroy(toaster_suggestion_list_t *list)
{
  size_t index;

  if (!list)
    return;

  for (index = 0; index < list->count; ++index)
    free(list->items[index].reason_storage);

  free(list->items);
  free(list);
}

void toaster_suggestion_list_clear(toaster_suggestion_list_t *list)
{
  size_t index;

  if (!list)
    return;

  for (index = 0; index < list->count; ++index)
    free(list->items[index].reason_storage);

  list->count = 0;
}

size_t toaster_suggestion_list_count(const toaster_suggestion_list_t *list)
{
  return list ? list->count : 0;
}

bool toaster_suggestion_list_get(const toaster_suggestion_list_t *list, size_t index,
                                 toaster_suggestion_t *out_suggestion)
{
  if (!list || !out_suggestion || index >= list->count)
    return false;

  *out_suggestion = list->items[index].suggestion;
  return true;
}

bool toaster_detect_fillers(const toaster_transcript_t *transcript, toaster_suggestion_list_t *list)
{
  size_t word_count;
  size_t index = 0;

  if (!transcript || !list)
    return false;

  word_count = toaster_transcript_word_count(transcript);
  while (index < word_count) {
    toaster_word_t current;

    if (!toaster_transcript_get_word(transcript, index, &current))
      return false;

    if (current.deleted) {
      ++index;
      continue;
    }

    if (index + 1 < word_count) {
      toaster_word_t next;

      if (!toaster_transcript_get_word(transcript, index + 1, &next))
        return false;

      if (!next.deleted && is_phrase_filler(current.text, next.text)) {
        if (!suggestion_list_push(list, TOASTER_SUGGESTION_DELETE_FILLER, index, index + 1,
                                  current.start_us, next.end_us, 0, "Phrase filler")) {
          return false;
        }
        index += 2;
        continue;
      }
    }

    if (index > 0) {
      toaster_word_t previous;

      if (!toaster_transcript_get_word(transcript, index - 1, &previous))
        return false;

      if (!previous.deleted && text_equals_ci(previous.text, current.text)) {
        if (!suggestion_list_push(list, TOASTER_SUGGESTION_DELETE_FILLER, index - 1, index,
                                  previous.start_us, current.end_us, 0, "Repeated word")) {
          return false;
        }
        ++index;
        continue;
      }
    }

    if (is_single_filler(current.text)) {
      if (!suggestion_list_push(list, TOASTER_SUGGESTION_DELETE_FILLER, index, index,
                                current.start_us, current.end_us, 0, "Single-word filler")) {
        return false;
      }
    }

    ++index;
  }

  return true;
}

bool toaster_detect_fillers_custom(const toaster_transcript_t *transcript,
                                   toaster_suggestion_list_t *list,
                                   const char *const *extra_fillers, size_t extra_filler_count,
                                   const char *const *ignore_words, size_t ignore_count)
{
  size_t word_count;
  size_t index = 0;

  if (!transcript || !list)
    return false;

  word_count = toaster_transcript_word_count(transcript);
  while (index < word_count) {
    toaster_word_t current;

    if (!toaster_transcript_get_word(transcript, index, &current))
      return false;

    if (current.deleted) {
      ++index;
      continue;
    }

    if (is_in_word_list(current.text, ignore_words, ignore_count)) {
      ++index;
      continue;
    }

    if (index + 1 < word_count) {
      toaster_word_t next;

      if (!toaster_transcript_get_word(transcript, index + 1, &next))
        return false;

      if (!next.deleted && !is_in_word_list(next.text, ignore_words, ignore_count) &&
          is_phrase_filler(current.text, next.text)) {
        if (!suggestion_list_push(list, TOASTER_SUGGESTION_DELETE_FILLER, index, index + 1,
                                  current.start_us, next.end_us, 0, "Phrase filler")) {
          return false;
        }
        index += 2;
        continue;
      }
    }

    if (index > 0) {
      toaster_word_t previous;

      if (!toaster_transcript_get_word(transcript, index - 1, &previous))
        return false;

      if (!previous.deleted && text_equals_ci(previous.text, current.text)) {
        if (!suggestion_list_push(list, TOASTER_SUGGESTION_DELETE_FILLER, index - 1, index,
                                  previous.start_us, current.end_us, 0, "Repeated word")) {
          return false;
        }
        ++index;
        continue;
      }
    }

    if (is_single_filler_custom(current.text, extra_fillers, extra_filler_count,
                                ignore_words, ignore_count)) {
      if (!suggestion_list_push(list, TOASTER_SUGGESTION_DELETE_FILLER, index, index,
                                current.start_us, current.end_us, 0, "Single-word filler")) {
        return false;
      }
    }

    ++index;
  }

  return true;
}

bool toaster_detect_pauses(const toaster_transcript_t *transcript, toaster_suggestion_list_t *list,
                           int64_t min_gap_us, int64_t shorten_to_us)
{
  size_t word_count;
  size_t index;

  if (!transcript || !list || min_gap_us <= 0 || shorten_to_us < 0)
    return false;

  word_count = toaster_transcript_word_count(transcript);
  if (word_count < 2)
    return true;

  for (index = 0; index + 1 < word_count; ++index) {
    toaster_word_t current;
    toaster_word_t next;
    int64_t gap_us;
    char reason[128];

    if (!toaster_transcript_get_word(transcript, index, &current) ||
        !toaster_transcript_get_word(transcript, index + 1, &next)) {
      return false;
    }

    if (current.deleted || next.deleted)
      continue;

    gap_us = next.start_us - current.end_us;
    if (gap_us < min_gap_us || gap_us <= shorten_to_us)
      continue;
    if (pause_already_cut(transcript, current.end_us, next.start_us - shorten_to_us))
      continue;

    snprintf(reason, sizeof(reason), "Pause %.2fs -> %.2fs", (double)gap_us / 1000000.0,
             (double)shorten_to_us / 1000000.0);
    if (!suggestion_list_push(list, TOASTER_SUGGESTION_SHORTEN_PAUSE, index, index + 1,
                              current.end_us, next.start_us, shorten_to_us, reason)) {
      return false;
    }
  }

  return true;
}
