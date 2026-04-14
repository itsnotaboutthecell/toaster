#ifndef TOASTER_H
#define TOASTER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32)
#if defined(toaster_EXPORTS)
#define TOASTER_API __declspec(dllexport)
#else
#define TOASTER_API __declspec(dllimport)
#endif
#else
#define TOASTER_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct toaster_transcript toaster_transcript_t;
typedef struct toaster_project toaster_project_t;
typedef struct toaster_signal_handler toaster_signal_handler_t;
typedef struct toaster_suggestion_list toaster_suggestion_list_t;

typedef enum toaster_suggestion_kind {
  TOASTER_SUGGESTION_DELETE_FILLER = 0,
  TOASTER_SUGGESTION_SILENCE_FILLER = 1,
  TOASTER_SUGGESTION_SHORTEN_PAUSE = 2
} toaster_suggestion_kind_t;

typedef struct toaster_word {
  const char *text;
  int64_t start_us;
  int64_t end_us;
  bool deleted;
  bool silenced;
} toaster_word_t;

typedef struct toaster_time_range {
  int64_t start_us;
  int64_t end_us;
} toaster_time_range_t;

typedef enum toaster_caption_format {
  TOASTER_CAPTION_FORMAT_SRT = 0,
  TOASTER_CAPTION_FORMAT_VTT = 1
} toaster_caption_format_t;

typedef struct toaster_suggestion {
  toaster_suggestion_kind_t kind;
  size_t start_index;
  size_t end_index;
  int64_t start_us;
  int64_t end_us;
  int64_t replacement_duration_us;
  const char *reason;
} toaster_suggestion_t;

typedef void (*toaster_signal_callback_t)(const char *signal, void *param, void *user_data);

TOASTER_API bool toaster_startup(void);
TOASTER_API void toaster_shutdown(void);
TOASTER_API bool toaster_is_started(void);
TOASTER_API const char *toaster_get_version(void);

TOASTER_API toaster_transcript_t *toaster_transcript_create(void);
TOASTER_API void toaster_transcript_destroy(toaster_transcript_t *transcript);
TOASTER_API bool toaster_transcript_clear(toaster_transcript_t *transcript);
TOASTER_API bool toaster_transcript_add_word(toaster_transcript_t *transcript, const char *text,
                                             int64_t start_us, int64_t end_us);
TOASTER_API size_t toaster_transcript_word_count(const toaster_transcript_t *transcript);
TOASTER_API bool toaster_transcript_get_word(const toaster_transcript_t *transcript, size_t index,
                                             toaster_word_t *out_word);
TOASTER_API bool toaster_transcript_set_word_text(toaster_transcript_t *transcript, size_t index,
                                                  const char *text);
TOASTER_API bool toaster_transcript_set_word_times(toaster_transcript_t *transcript, size_t index,
                                                   int64_t start_us, int64_t end_us);
TOASTER_API bool toaster_transcript_delete_range(toaster_transcript_t *transcript, size_t start_index,
                                                 size_t end_index);
TOASTER_API bool toaster_transcript_silence_range(toaster_transcript_t *transcript, size_t start_index,
                                                  size_t end_index);
TOASTER_API bool toaster_transcript_unsilence_range(toaster_transcript_t *transcript,
                                                    size_t start_index, size_t end_index);
TOASTER_API bool toaster_transcript_restore_range(toaster_transcript_t *transcript, size_t start_index,
                                                  size_t end_index);
TOASTER_API bool toaster_transcript_restore_all(toaster_transcript_t *transcript);
TOASTER_API size_t toaster_transcript_deleted_span_count(const toaster_transcript_t *transcript);
TOASTER_API bool toaster_transcript_get_deleted_span(const toaster_transcript_t *transcript,
                                                     size_t span_index, toaster_time_range_t *out_range);
TOASTER_API size_t toaster_transcript_silenced_span_count(const toaster_transcript_t *transcript);
TOASTER_API bool toaster_transcript_get_silenced_span(const toaster_transcript_t *transcript,
                                                      size_t span_index, toaster_time_range_t *out_range);
TOASTER_API bool toaster_transcript_add_cut_span(toaster_transcript_t *transcript, int64_t start_us,
                                                 int64_t end_us);
TOASTER_API bool toaster_transcript_clear_cut_spans(toaster_transcript_t *transcript);
TOASTER_API size_t toaster_transcript_cut_span_count(const toaster_transcript_t *transcript);
TOASTER_API bool toaster_transcript_get_cut_span(const toaster_transcript_t *transcript,
                                                 size_t span_index, toaster_time_range_t *out_range);
TOASTER_API size_t toaster_transcript_keep_segment_count(const toaster_transcript_t *transcript);
TOASTER_API bool toaster_transcript_get_keep_segment(const toaster_transcript_t *transcript,
                                                     size_t segment_index, toaster_time_range_t *out_range);
TOASTER_API bool toaster_transcript_get_bounds(const toaster_transcript_t *transcript,
                                               toaster_time_range_t *out_range);
TOASTER_API bool toaster_transcript_export_script(const toaster_transcript_t *transcript,
                                                  const char *path);
TOASTER_API bool toaster_transcript_export_captions(const toaster_transcript_t *transcript,
                                                    const char *path,
                                                    toaster_caption_format_t format);

TOASTER_API bool toaster_transcript_save_snapshot(toaster_transcript_t *transcript);
TOASTER_API bool toaster_transcript_undo(toaster_transcript_t *transcript);
TOASTER_API bool toaster_transcript_redo(toaster_transcript_t *transcript);
TOASTER_API bool toaster_transcript_can_undo(const toaster_transcript_t *transcript);
TOASTER_API bool toaster_transcript_can_redo(const toaster_transcript_t *transcript);
TOASTER_API void toaster_transcript_clear_history(toaster_transcript_t *transcript);

TOASTER_API bool toaster_transcript_split_word(toaster_transcript_t *transcript, size_t index,
                                               int64_t split_us);

TOASTER_API toaster_project_t *toaster_project_create(void);
TOASTER_API void toaster_project_destroy(toaster_project_t *project);
TOASTER_API toaster_transcript_t *toaster_project_get_transcript(toaster_project_t *project);
TOASTER_API const toaster_transcript_t *toaster_project_get_transcript_const(
  const toaster_project_t *project);
TOASTER_API bool toaster_project_set_media_path(toaster_project_t *project, const char *media_path);
TOASTER_API const char *toaster_project_get_media_path(const toaster_project_t *project);
TOASTER_API bool toaster_project_set_language(toaster_project_t *project, const char *language);
TOASTER_API const char *toaster_project_get_language(const toaster_project_t *project);
TOASTER_API bool toaster_project_save(const toaster_project_t *project, const char *path);
TOASTER_API toaster_project_t *toaster_project_load(const char *path);

TOASTER_API toaster_suggestion_list_t *toaster_suggestion_list_create(void);
TOASTER_API void toaster_suggestion_list_destroy(toaster_suggestion_list_t *list);
TOASTER_API void toaster_suggestion_list_clear(toaster_suggestion_list_t *list);
TOASTER_API size_t toaster_suggestion_list_count(const toaster_suggestion_list_t *list);
TOASTER_API bool toaster_suggestion_list_get(const toaster_suggestion_list_t *list, size_t index,
                                             toaster_suggestion_t *out_suggestion);
TOASTER_API bool toaster_detect_fillers(const toaster_transcript_t *transcript,
                                        toaster_suggestion_list_t *list);
TOASTER_API bool toaster_detect_fillers_custom(const toaster_transcript_t *transcript,
                                               toaster_suggestion_list_t *list,
                                               const char *const *extra_fillers,
                                               size_t extra_filler_count,
                                               const char *const *ignore_words,
                                               size_t ignore_count);
TOASTER_API bool toaster_detect_pauses(const toaster_transcript_t *transcript,
                                       toaster_suggestion_list_t *list, int64_t min_gap_us,
                                       int64_t shorten_to_us);

TOASTER_API toaster_signal_handler_t *toaster_signal_handler_create(void);
TOASTER_API void toaster_signal_handler_destroy(toaster_signal_handler_t *handler);
TOASTER_API bool toaster_signal_handler_connect(toaster_signal_handler_t *handler, const char *signal,
                                                toaster_signal_callback_t callback, void *user_data);
TOASTER_API bool toaster_signal_handler_disconnect(toaster_signal_handler_t *handler, const char *signal,
                                                   toaster_signal_callback_t callback, void *user_data);
TOASTER_API void toaster_signal_handler_emit(toaster_signal_handler_t *handler, const char *signal,
                                             void *param);

#ifdef __cplusplus
}
#endif

#endif
