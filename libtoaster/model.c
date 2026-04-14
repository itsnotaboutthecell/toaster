#include "toaster.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <shlobj.h>
#include <windows.h>
#else
#include <sys/stat.h>
#include <unistd.h>
#endif

#define TOASTER_HF_BASE_URL "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/"

static const char *lang_en[] = {"en"};
static const char *lang_multi[] = {"en",  "zh", "de",  "es", "ru", "ko", "fr",  "ja",  "pt",  "tr",
                                   "pl",  "ca", "nl",  "ar", "sv", "it", "id",  "hi",  "fi",  "vi",
                                   "he",  "uk", "el",  "ms", "cs", "ro", "da",  "hu",  "ta",  "no",
                                   "th",  "ur", "hr",  "bg", "lt", "la", "mi",  "ml",  "cy",  "sk",
                                   "te",  "fa", "lv",  "bn", "sr", "az", "sl",  "kn",  "et",  "mk",
                                   "br",  "eu", "is",  "hy", "ne", "mn", "bs",  "kk",  "sq",  "sw",
                                   "gl",  "mr", "pa",  "si", "km", "sn", "yo",  "so",  "af",  "oc",
                                   "ka",  "be", "tg",  "sd", "gu", "am", "yi",  "lo",  "uz",  "fo",
                                   "ht",  "ps", "tk",  "nn", "mt", "sa", "lb",  "my",  "bo",  "tl",
                                   "mg",  "as", "tt",  "haw", "ln", "ha", "ba", "jw",  "su"};

typedef struct model_entry {
  const char *id;
  const char *name;
  const char *description;
  const char *filename;
  const char *sha256;
  uint64_t size_mb;
  float accuracy_score;
  float speed_score;
  bool is_recommended;
  bool supports_translation;
  const char *const *languages;
  size_t language_count;
} model_entry_t;

static const model_entry_t catalog[] = {
    {"tiny.en", "Whisper Tiny (English)", "Fastest model, English only. Good for quick drafts.",
     "ggml-tiny.en.bin", "921e4cf8985a7e0ab1fde924b1afb2ac5ab776eb3ce8199fb4e04dc1abd5307e", 75, 0.3f,
     1.0f, true, false, lang_en, 1},
    {"small", "Whisper Small", "Good balance of speed and accuracy. Supports 99 languages.",
     "ggml-small.bin", "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1c3c80ec31", 465, 0.6f,
     0.7f, false, true, lang_multi, 99},
    {"medium-q4", "Whisper Medium (Q4)", "Quantized medium model. Near-medium accuracy, smaller size.",
     "ggml-medium-q4_0.bin", NULL, 469, 0.7f, 0.5f, false, true, lang_multi, 99},
    {"turbo", "Whisper Large v3 Turbo",
     "Best speed/accuracy trade-off. Recommended for production use.",
     "ggml-large-v3-turbo.bin", NULL, 1549, 0.9f, 0.6f, true, true, lang_multi, 99},
    {"large-q5", "Whisper Large v3 (Q5)", "Highest accuracy. Quantized for reduced memory.",
     "ggml-large-v3-q5_0.bin", NULL, 1031, 1.0f, 0.3f, false, true, lang_multi, 99},
};

#define CATALOG_COUNT (sizeof(catalog) / sizeof(catalog[0]))

static char active_model_id[64] = "tiny.en";
static char models_directory[1024] = {0};

static bool ensure_models_dir(void)
{
  if (models_directory[0] != '\0')
    return true;

#ifdef _WIN32
  {
    char appdata[MAX_PATH];
    if (SHGetFolderPathA(NULL, CSIDL_APPDATA, NULL, 0, appdata) != S_OK)
      return false;
    snprintf(models_directory, sizeof(models_directory), "%s\\Toaster\\models", appdata);
  }
#else
  {
    const char *home = getenv("HOME");
    if (!home)
      return false;
    snprintf(models_directory, sizeof(models_directory), "%s/.local/share/toaster/models", home);
  }
#endif
  return true;
}

static bool file_exists(const char *path)
{
#ifdef _WIN32
  DWORD attrs = GetFileAttributesA(path);
  return (attrs != INVALID_FILE_ATTRIBUTES && !(attrs & FILE_ATTRIBUTE_DIRECTORY));
#else
  return access(path, F_OK) == 0;
#endif
}

static bool check_model_downloaded(const model_entry_t *entry)
{
  char path[2048];

  if (!ensure_models_dir())
    return false;

  snprintf(path, sizeof(path), "%s%c%s", models_directory,
#ifdef _WIN32
           '\\',
#else
           '/',
#endif
           entry->filename);

  return file_exists(path);
}

static void fill_info(const model_entry_t *entry, toaster_model_info_t *info)
{
  char url_buf[512];

  info->id = entry->id;
  info->name = entry->name;
  info->description = entry->description;
  info->filename = entry->filename;

  snprintf(url_buf, sizeof(url_buf), "%s%s", TOASTER_HF_BASE_URL, entry->filename);
  /* URL is ephemeral but constant for the catalog, just point to the base URL pattern */
  info->url = TOASTER_HF_BASE_URL;

  info->sha256 = entry->sha256;
  info->size_mb = entry->size_mb;
  info->accuracy_score = entry->accuracy_score;
  info->speed_score = entry->speed_score;
  info->is_downloaded = check_model_downloaded(entry);
  info->is_recommended = entry->is_recommended;
  info->supports_translation = entry->supports_translation;
  info->engine = TOASTER_MODEL_ENGINE_WHISPER;
  info->supported_languages = entry->languages;
  info->language_count = entry->language_count;
}

size_t toaster_model_catalog_count(void)
{
  return CATALOG_COUNT;
}

bool toaster_model_catalog_get(size_t index, toaster_model_info_t *out_info)
{
  if (index >= CATALOG_COUNT || !out_info)
    return false;

  fill_info(&catalog[index], out_info);
  return true;
}

bool toaster_model_catalog_find(const char *model_id, toaster_model_info_t *out_info)
{
  size_t i;

  if (!model_id)
    return false;

  for (i = 0; i < CATALOG_COUNT; i++) {
    if (strcmp(catalog[i].id, model_id) == 0) {
      if (out_info)
        fill_info(&catalog[i], out_info);
      return true;
    }
  }
  return false;
}

bool toaster_model_is_downloaded(const char *model_id)
{
  size_t i;

  if (!model_id)
    return false;

  for (i = 0; i < CATALOG_COUNT; i++) {
    if (strcmp(catalog[i].id, model_id) == 0)
      return check_model_downloaded(&catalog[i]);
  }
  return false;
}

const char *toaster_model_get_active(void)
{
  return active_model_id;
}

bool toaster_model_set_active(const char *model_id)
{
  size_t i;

  if (!model_id)
    return false;

  for (i = 0; i < CATALOG_COUNT; i++) {
    if (strcmp(catalog[i].id, model_id) == 0) {
      snprintf(active_model_id, sizeof(active_model_id), "%s", model_id);
      return true;
    }
  }
  return false;
}

const char *toaster_model_get_directory(void)
{
  if (!ensure_models_dir())
    return NULL;
  return models_directory;
}

bool toaster_model_set_directory(const char *dir)
{
  if (!dir || strlen(dir) == 0 || strlen(dir) >= sizeof(models_directory))
    return false;

  snprintf(models_directory, sizeof(models_directory), "%s", dir);
  return true;
}

bool toaster_model_refresh_status(void)
{
  /* Re-check download status is done lazily in fill_info/check_model_downloaded.
     This function exists so the frontend can force a re-check after a download. */
  return ensure_models_dir();
}
