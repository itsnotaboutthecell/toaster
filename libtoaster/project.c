#include "toaster.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct toaster_project {
  char *media_path;
  char *language;
  toaster_transcript_t *transcript;
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

static bool replace_string(char **target, const char *text)
{
  char *copy = toaster_strdup(text ? text : "");

  if (!copy)
    return false;

  free(*target);
  *target = copy;
  return true;
}

static void trim_newline(char *line)
{
  size_t length;

  if (!line)
    return;

  length = strlen(line);
  while (length > 0 && (line[length - 1] == '\n' || line[length - 1] == '\r')) {
    line[length - 1] = '\0';
    --length;
  }
}

static void write_escaped(FILE *file, const char *text)
{
  const unsigned char *cursor = (const unsigned char *)(text ? text : "");

  while (*cursor) {
    switch (*cursor) {
    case '\\':
      fputs("\\\\", file);
      break;
    case '\t':
      fputs("\\t", file);
      break;
    case '\n':
      fputs("\\n", file);
      break;
    case '\r':
      fputs("\\r", file);
      break;
    default:
      fputc((int)*cursor, file);
      break;
    }

    ++cursor;
  }
}

static char *unescape_string(const char *text)
{
  char *out;
  size_t read_index = 0;
  size_t write_index = 0;
  size_t length;

  if (!text)
    return toaster_strdup("");

  length = strlen(text);
  out = (char *)malloc(length + 1);
  if (!out)
    return NULL;

  while (read_index < length) {
    if (text[read_index] == '\\' && read_index + 1 < length) {
      ++read_index;
      switch (text[read_index]) {
      case 'n':
        out[write_index++] = '\n';
        break;
      case 'r':
        out[write_index++] = '\r';
        break;
      case 't':
        out[write_index++] = '\t';
        break;
      case '\\':
        out[write_index++] = '\\';
        break;
      default:
        out[write_index++] = text[read_index];
        break;
      }
      ++read_index;
      continue;
    }

    out[write_index++] = text[read_index++];
  }

  out[write_index] = '\0';
  return out;
}

static char *next_field(char **cursor)
{
  char *start;
  char *separator;

  if (!cursor || !*cursor)
    return NULL;

  start = *cursor;
  separator = strchr(start, '\t');
  if (!separator) {
    *cursor = NULL;
    return start;
  }

  *separator = '\0';
  *cursor = separator + 1;
  return start;
}

toaster_project_t *toaster_project_create(void)
{
  toaster_project_t *project = (toaster_project_t *)calloc(1, sizeof(toaster_project_t));

  if (!project)
    return NULL;

  project->transcript = toaster_transcript_create();
  if (!project->transcript) {
    free(project);
    return NULL;
  }

  project->media_path = toaster_strdup("");
  project->language = toaster_strdup("en");
  if (!project->media_path || !project->language) {
    toaster_project_destroy(project);
    return NULL;
  }

  return project;
}

void toaster_project_destroy(toaster_project_t *project)
{
  if (!project)
    return;

  free(project->media_path);
  free(project->language);
  toaster_transcript_destroy(project->transcript);
  free(project);
}

toaster_transcript_t *toaster_project_get_transcript(toaster_project_t *project)
{
  return project ? project->transcript : NULL;
}

const toaster_transcript_t *toaster_project_get_transcript_const(const toaster_project_t *project)
{
  return project ? project->transcript : NULL;
}

bool toaster_project_set_media_path(toaster_project_t *project, const char *media_path)
{
  if (!project)
    return false;

  return replace_string(&project->media_path, media_path ? media_path : "");
}

const char *toaster_project_get_media_path(const toaster_project_t *project)
{
  return project ? project->media_path : NULL;
}

bool toaster_project_set_language(toaster_project_t *project, const char *language)
{
  if (!project)
    return false;

  return replace_string(&project->language, language ? language : "");
}

const char *toaster_project_get_language(const toaster_project_t *project)
{
  return project ? project->language : NULL;
}

bool toaster_project_save(const toaster_project_t *project, const char *path)
{
  FILE *file;
  size_t index;
  const toaster_transcript_t *transcript;

  if (!project || !path)
    return false;

  file = fopen(path, "wb");
  if (!file)
    return false;

  transcript = toaster_project_get_transcript_const(project);

  fputs("TOASTER_PROJECT\t1\n", file);
  fputs("MEDIA\t", file);
  write_escaped(file, toaster_project_get_media_path(project));
  fputc('\n', file);
  fputs("LANGUAGE\t", file);
  write_escaped(file, toaster_project_get_language(project));
  fputc('\n', file);

  for (index = 0; index < toaster_transcript_word_count(transcript); ++index) {
    toaster_word_t word;

    if (!toaster_transcript_get_word(transcript, index, &word))
      continue;

    fprintf(file, "WORD\t%" PRId64 "\t%" PRId64 "\t%d\t%d\t%.6f\t%d\t", word.start_us, word.end_us,
            word.deleted ? 1 : 0, word.silenced ? 1 : 0, word.confidence, word.speaker_id);
    write_escaped(file, word.text);
    fputc('\n', file);
  }

  for (index = 0; index < toaster_transcript_cut_span_count(transcript); ++index) {
    toaster_time_range_t range;

    if (!toaster_transcript_get_cut_span(transcript, index, &range))
      continue;

    fprintf(file, "CUT\t%" PRId64 "\t%" PRId64 "\n", range.start_us, range.end_us);
  }

  fclose(file);
  return true;
}

toaster_project_t *toaster_project_load(const char *path)
{
  FILE *file;
  char line[8192];
  toaster_project_t *project;
  bool saw_header = false;

  if (!path)
    return NULL;

  file = fopen(path, "rb");
  if (!file)
    return NULL;

  project = toaster_project_create();
  if (!project) {
    fclose(file);
    return NULL;
  }

  while (fgets(line, sizeof(line), file)) {
    char *cursor = line;
    char *tag;

    trim_newline(line);
    tag = next_field(&cursor);
    if (!tag || !*tag)
      continue;

    if (strcmp(tag, "TOASTER_PROJECT") == 0) {
      saw_header = true;
      continue;
    }

    if (strcmp(tag, "MEDIA") == 0) {
      char *value = unescape_string(cursor ? cursor : "");
      if (!value || !toaster_project_set_media_path(project, value)) {
        free(value);
        toaster_project_destroy(project);
        fclose(file);
        return NULL;
      }
      free(value);
      continue;
    }

    if (strcmp(tag, "LANGUAGE") == 0) {
      char *value = unescape_string(cursor ? cursor : "");
      if (!value || !toaster_project_set_language(project, value)) {
        free(value);
        toaster_project_destroy(project);
        fclose(file);
        return NULL;
      }
      free(value);
      continue;
    }

    if (strcmp(tag, "WORD") == 0) {
      char *start_field = next_field(&cursor);
      char *end_field = next_field(&cursor);
      char *deleted_field = next_field(&cursor);
      char *silenced_field = next_field(&cursor);
      char *confidence_field = NULL;
      char *speaker_field = NULL;
      char *text_field;
      char *text_value;
      size_t word_index;
      float confidence = -1.0f;
      int speaker_id = -1;

      if (!start_field || !end_field || !deleted_field || !silenced_field) {
        toaster_project_destroy(project);
        fclose(file);
        return NULL;
      }

      /* Try reading optional confidence and speaker_id fields (v2 format) */
      {
        char *maybe_conf = next_field(&cursor);
        if (maybe_conf && cursor) {
          char *maybe_spk = next_field(&cursor);
          if (maybe_spk && cursor) {
            confidence = (float)atof(maybe_conf);
            speaker_id = atoi(maybe_spk);
            text_field = cursor ? cursor : (char *)"";
          } else {
            /* Only one extra field — treat as text (v1 format) */
            text_field = maybe_conf;
          }
        } else {
          text_field = maybe_conf ? maybe_conf : (char *)"";
        }
      }

      text_value = unescape_string(text_field);
      if (!text_value) {
        toaster_project_destroy(project);
        fclose(file);
        return NULL;
      }

      if (!toaster_transcript_add_word(project->transcript, text_value, _strtoi64(start_field, NULL, 10),
                                       _strtoi64(end_field, NULL, 10))) {
        free(text_value);
        toaster_project_destroy(project);
        fclose(file);
        return NULL;
      }

      word_index = toaster_transcript_word_count(project->transcript) - 1;
      if (atoi(deleted_field) != 0)
        toaster_transcript_delete_range(project->transcript, word_index, word_index);
      if (atoi(silenced_field) != 0)
        toaster_transcript_silence_range(project->transcript, word_index, word_index);
      toaster_transcript_set_word_confidence(project->transcript, word_index, confidence);
      toaster_transcript_set_word_speaker(project->transcript, word_index, speaker_id);
      free(text_value);
      continue;
    }

    if (strcmp(tag, "CUT") == 0) {
      char *start_field = next_field(&cursor);
      char *end_field = next_field(&cursor);

      if (!start_field || !end_field ||
          !toaster_transcript_add_cut_span(project->transcript, _strtoi64(start_field, NULL, 10),
                                           _strtoi64(end_field, NULL, 10))) {
        toaster_project_destroy(project);
        fclose(file);
        return NULL;
      }
      continue;
    }
  }

  fclose(file);

  if (!saw_header) {
    toaster_project_destroy(project);
    return NULL;
  }

  return project;
}
