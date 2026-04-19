import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ask } from "@tauri-apps/plugin-dialog";
import { ChevronDown, Globe, Layers } from "lucide-react";
import type { ModelCardStatus } from "@/components/onboarding";
import { ModelCard } from "@/components/onboarding";
import { useModelStore } from "@/stores/modelStore";
import { useSettingsNavStore } from "@/stores/settingsNavStore";
import { LANGUAGES } from "@/lib/constants/languages.ts";
import { type ModelCategory, type ModelInfo } from "@/bindings";

type CategoryFilter = ModelCategory | "all";

interface ModelsSettingsProps {
  /**
   * When set, the segmented category filter is not rendered and the
   * list is pinned to this category. Used by PostProcessingSettings
   * to embed the unified picker filtered to `PostProcessor`.
   */
  lockedCategory?: ModelCategory;
  /**
   * Initial selection for the category filter when `lockedCategory`
   * is not set. Ignored when `lockedCategory` is provided.
   */
  initialFilter?: CategoryFilter;
}

// check if model supports a language based on its supported_languages list
const modelSupportsLanguage = (model: ModelInfo, langCode: string): boolean => {
  return model.supported_languages.includes(langCode);
};

const categoryBadgeKey = (category: ModelCategory | null | undefined): string | null => {
  if (category === "Transcription") return "settings.models.badge.transcription";
  if (category === "PostProcessor") return "settings.models.badge.postProcessing";
  return null;
};

export const ModelsSettings: React.FC<ModelsSettingsProps> = ({
  lockedCategory,
  initialFilter,
}) => {
  const { t } = useTranslation();
  const consumePendingModelsFilter = useSettingsNavStore(
    (s) => s.consumePendingModelsFilter,
  );
  const [switchingModelId, setSwitchingModelId] = useState<string | null>(null);
  const [categoryFilter, setCategoryFilter] = useState<CategoryFilter>(() => {
    if (lockedCategory) return lockedCategory;
    const pending = consumePendingModelsFilter();
    if (pending) return pending;
    return initialFilter ?? "all";
  });
  const [languageFilter, setLanguageFilter] = useState("all");
  const [languageDropdownOpen, setLanguageDropdownOpen] = useState(false);
  const [languageSearch, setLanguageSearch] = useState("");
  const languageDropdownRef = useRef<HTMLDivElement>(null);
  const languageSearchInputRef = useRef<HTMLInputElement>(null);
  const [categoryDropdownOpen, setCategoryDropdownOpen] = useState(false);
  const categoryDropdownRef = useRef<HTMLDivElement>(null);
  const {
    models,
    currentModel,
    downloadingModels,
    downloadProgress,
    downloadStats,
    verifyingModels,
    extractingModels,
    loading,
    downloadModel,
    cancelDownload,
    selectModel,
    deleteModel,
  } = useModelStore();

  // If the consumer changes `lockedCategory` at runtime, honor it.
  useEffect(() => {
    if (lockedCategory) {
      setCategoryFilter(lockedCategory);
    }
  }, [lockedCategory]);

  // click outside handler for language dropdown
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        languageDropdownRef.current &&
        !languageDropdownRef.current.contains(event.target as Node)
      ) {
        setLanguageDropdownOpen(false);
        setLanguageSearch("");
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  // click outside handler for category dropdown
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        categoryDropdownRef.current &&
        !categoryDropdownRef.current.contains(event.target as Node)
      ) {
        setCategoryDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  // focus search input when dropdown opens
  useEffect(() => {
    if (languageDropdownOpen && languageSearchInputRef.current) {
      languageSearchInputRef.current.focus();
    }
  }, [languageDropdownOpen]);

  // filtered languages for dropdown (exclude "auto")
  const filteredLanguages = useMemo(() => {
    return LANGUAGES.filter(
      (lang) =>
        lang.value !== "auto" &&
        lang.label.toLowerCase().includes(languageSearch.toLowerCase()),
    );
  }, [languageSearch]);

  // Get selected language label
  const selectedLanguageLabel = useMemo(() => {
    if (languageFilter === "all") {
      return t("settings.models.filters.allLanguages");
    }
    return LANGUAGES.find((lang) => lang.value === languageFilter)?.label || "";
  }, [languageFilter, t]);

  // Selected category label for the filter dropdown trigger
  const selectedCategoryLabel = useMemo(() => {
    if (categoryFilter === "all") return t("settings.models.filter.all");
    if (categoryFilter === "Transcription")
      return t("settings.models.filter.transcription");
    return t("settings.models.filter.postProcessing");
  }, [categoryFilter, t]);

  const getModelStatus = (modelId: string): ModelCardStatus => {
    if (modelId in extractingModels) {
      return "extracting";
    }
    if (modelId in verifyingModels) {
      return "verifying";
    }
    if (modelId in downloadingModels) {
      return "downloading";
    }
    if (switchingModelId === modelId) {
      return "switching";
    }
    if (modelId === currentModel) {
      return "active";
    }
    const model = models.find((m: ModelInfo) => m.id === modelId);
    if (model?.is_downloaded) {
      return "available";
    }
    return "downloadable";
  };

  const getDownloadProgress = (modelId: string): number | undefined => {
    const progress = downloadProgress[modelId];
    return progress?.percentage;
  };

  const getDownloadSpeed = (modelId: string): number | undefined => {
    const stats = downloadStats[modelId];
    return stats?.speed;
  };

  const handleModelSelect = async (modelId: string) => {
    setSwitchingModelId(modelId);
    try {
      await selectModel(modelId);
    } finally {
      setSwitchingModelId(null);
    }
  };

  const handleModelDownload = async (modelId: string) => {
    await downloadModel(modelId);
  };

  const handleModelDelete = async (modelId: string) => {
    const model = models.find((m: ModelInfo) => m.id === modelId);
    const modelName = model?.name || modelId;
    const isActive = modelId === currentModel;

    const confirmed = await ask(
      isActive
        ? t("settings.models.deleteActiveConfirm", { modelName })
        : t("settings.models.deleteConfirm", { modelName }),
      {
        title: t("settings.models.deleteTitle"),
        kind: "warning",
      },
    );

    if (confirmed) {
      try {
        await deleteModel(modelId);
      } catch (err) {
        console.error(`Failed to delete model ${modelId}:`, err);
      }
    }
  };

  const handleModelCancel = async (modelId: string) => {
    try {
      await cancelDownload(modelId);
    } catch (err) {
      console.error(`Failed to cancel download for ${modelId}:`, err);
    }
  };

  const getCategoryBadge = (model: ModelInfo): string | undefined => {
    const key = categoryBadgeKey(model.category ?? null);
    return key ? t(key) : undefined;
  };

  // Filter models based on category + language filters
  const filteredModels = useMemo(() => {
    return models.filter((model: ModelInfo) => {
      if (categoryFilter !== "all") {
        if ((model.category ?? "Transcription") !== categoryFilter) return false;
      }
      if (languageFilter !== "all") {
        if (!modelSupportsLanguage(model, languageFilter)) return false;
      }
      return true;
    });
  }, [models, categoryFilter, languageFilter]);

  // Split filtered models into downloaded (including custom) and available sections
  const { downloadedModels, availableModels } = useMemo(() => {
    const downloaded: ModelInfo[] = [];
    const available: ModelInfo[] = [];

    for (const model of filteredModels) {
      if (
        model.is_custom ||
        model.is_downloaded ||
        model.id in downloadingModels ||
        model.id in extractingModels
      ) {
        downloaded.push(model);
      } else {
        available.push(model);
      }
    }

    // Sort: active model first, then non-custom, then custom at the bottom
    downloaded.sort((a, b) => {
      if (a.id === currentModel) return -1;
      if (b.id === currentModel) return 1;
      if (a.is_custom !== b.is_custom) return a.is_custom ? 1 : -1;
      return 0;
    });

    return {
      downloadedModels: downloaded,
      availableModels: available,
    };
  }, [filteredModels, downloadingModels, extractingModels, currentModel]);

  const categoryTabs: Array<{ value: CategoryFilter; labelKey: string }> = [
    { value: "all", labelKey: "settings.models.filter.all" },
    {
      value: "Transcription",
      labelKey: "settings.models.filter.transcription",
    },
    {
      value: "PostProcessor",
      labelKey: "settings.models.filter.postProcessing",
    },
  ];

  if (loading) {
    return (
      <div className="max-w-5xl w-full mx-auto" data-testid="settings-outer">
        <div className="flex items-center justify-center py-16">
          <div className="w-8 h-8 border-2 border-logo-primary border-t-transparent rounded-full animate-spin" />
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-5xl w-full mx-auto space-y-6" data-testid="settings-outer">
      <div className="mb-4">
        <h1 className="text-xl font-semibold mb-2">
          {t("settings.models.title")}
        </h1>
        <p className="text-sm text-text/60">
          {t("settings.models.description")}
        </p>
      </div>
      {!lockedCategory && (
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium text-text/60">
            {t("settings.models.yourModels")}
          </h2>
          <div className="flex items-center gap-2">
            {/* Category filter dropdown */}
            <div className="relative" ref={categoryDropdownRef}>
              <button
                type="button"
                onClick={() => setCategoryDropdownOpen(!categoryDropdownOpen)}
                className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                  categoryFilter !== "all"
                    ? "bg-logo-primary/20 text-logo-primary"
                    : "bg-mid-gray/10 text-text/60 hover:bg-mid-gray/20"
                }`}
              >
                <Layers className="w-3.5 h-3.5" />
                <span className="max-w-[140px] truncate">
                  {selectedCategoryLabel}
                </span>
                <ChevronDown
                  className={`w-3.5 h-3.5 transition-transform ${
                    categoryDropdownOpen ? "rotate-180" : ""
                  }`}
                />
              </button>
              {categoryDropdownOpen && (
                <div className="absolute top-full right-0 mt-1 w-48 bg-background border border-mid-gray/80 rounded-lg shadow-lg z-50 overflow-hidden">
                  {categoryTabs.map((tab) => {
                    const isActive = categoryFilter === tab.value;
                    return (
                      <button
                        key={tab.value}
                        type="button"
                        onClick={() => {
                          setCategoryFilter(tab.value);
                          setCategoryDropdownOpen(false);
                        }}
                        className={`w-full px-3 py-1.5 text-sm text-left transition-colors ${
                          isActive
                            ? "bg-logo-primary/20 text-logo-primary font-semibold"
                            : "hover:bg-mid-gray/10"
                        }`}
                      >
                        {t(tab.labelKey)}
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
            {/* Language filter dropdown */}
            <div className="relative" ref={languageDropdownRef}>
              <button
                type="button"
                onClick={() => setLanguageDropdownOpen(!languageDropdownOpen)}
                className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                  languageFilter !== "all"
                    ? "bg-logo-primary/20 text-logo-primary"
                    : "bg-mid-gray/10 text-text/60 hover:bg-mid-gray/20"
                }`}
              >
                <Globe className="w-3.5 h-3.5" />
                <span className="max-w-[120px] truncate">
                  {selectedLanguageLabel}
                </span>
                <ChevronDown
                  className={`w-3.5 h-3.5 transition-transform ${
                    languageDropdownOpen ? "rotate-180" : ""
                  }`}
                />
              </button>

              {languageDropdownOpen && (
                <div className="absolute top-full right-0 mt-1 w-56 bg-background border border-mid-gray/80 rounded-lg shadow-lg z-50 overflow-hidden">
                  <div className="p-2 border-b border-mid-gray/40">
                    <input
                      ref={languageSearchInputRef}
                      type="text"
                      value={languageSearch}
                      onChange={(e) => setLanguageSearch(e.target.value)}
                      onKeyDown={(e) => {
                        if (
                          e.key === "Enter" &&
                          filteredLanguages.length > 0
                        ) {
                          setLanguageFilter(filteredLanguages[0].value);
                          setLanguageDropdownOpen(false);
                          setLanguageSearch("");
                        } else if (e.key === "Escape") {
                          setLanguageDropdownOpen(false);
                          setLanguageSearch("");
                        }
                      }}
                      placeholder={t(
                        "settings.general.language.searchPlaceholder",
                      )}
                      className="w-full px-2 py-1 text-sm bg-mid-gray/10 border border-mid-gray/40 rounded-md focus:outline-none focus:ring-1 focus:ring-logo-primary"
                    />
                  </div>
                  <div className="max-h-48 overflow-y-auto">
                    <button
                      type="button"
                      onClick={() => {
                        setLanguageFilter("all");
                        setLanguageDropdownOpen(false);
                        setLanguageSearch("");
                      }}
                      className={`w-full px-3 py-1.5 text-sm text-left transition-colors ${
                        languageFilter === "all"
                          ? "bg-logo-primary/20 text-logo-primary font-semibold"
                          : "hover:bg-mid-gray/10"
                      }`}
                    >
                      {t("settings.models.filters.allLanguages")}
                    </button>
                    {filteredLanguages.map((lang) => (
                      <button
                        key={lang.value}
                        type="button"
                        onClick={() => {
                          setLanguageFilter(lang.value);
                          setLanguageDropdownOpen(false);
                          setLanguageSearch("");
                        }}
                        className={`w-full px-3 py-1.5 text-sm text-left transition-colors ${
                          languageFilter === lang.value
                            ? "bg-logo-primary/20 text-logo-primary font-semibold"
                            : "hover:bg-mid-gray/10"
                        }`}
                      >
                        {lang.label}
                      </button>
                    ))}
                    {filteredLanguages.length === 0 && (
                      <div className="px-3 py-2 text-sm text-text/50 text-center">
                        {t("settings.general.language.noResults")}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
      {filteredModels.length > 0 ? (
        <div className="space-y-6">
          {/* Downloaded Models Section */}
          <div className="space-y-3">
            {lockedCategory && (
              <div className="flex items-center justify-between">
                <h2 className="text-sm font-medium text-text/60">
                  {t("settings.models.yourModels")}
                </h2>
                {/* Language filter dropdown (locked-category variant has only language filter) */}
                <div className="relative" ref={languageDropdownRef}>
                  <button
                    type="button"
                    onClick={() =>
                      setLanguageDropdownOpen(!languageDropdownOpen)
                    }
                    className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                      languageFilter !== "all"
                        ? "bg-logo-primary/20 text-logo-primary"
                        : "bg-mid-gray/10 text-text/60 hover:bg-mid-gray/20"
                    }`}
                  >
                    <Globe className="w-3.5 h-3.5" />
                    <span className="max-w-[120px] truncate">
                      {selectedLanguageLabel}
                    </span>
                    <ChevronDown
                      className={`w-3.5 h-3.5 transition-transform ${
                        languageDropdownOpen ? "rotate-180" : ""
                      }`}
                    />
                  </button>

                  {languageDropdownOpen && (
                    <div className="absolute top-full right-0 mt-1 w-56 bg-background border border-mid-gray/80 rounded-lg shadow-lg z-50 overflow-hidden">
                      <div className="p-2 border-b border-mid-gray/40">
                        <input
                          ref={languageSearchInputRef}
                          type="text"
                          value={languageSearch}
                          onChange={(e) => setLanguageSearch(e.target.value)}
                          onKeyDown={(e) => {
                            if (
                              e.key === "Enter" &&
                              filteredLanguages.length > 0
                            ) {
                              setLanguageFilter(filteredLanguages[0].value);
                              setLanguageDropdownOpen(false);
                              setLanguageSearch("");
                            } else if (e.key === "Escape") {
                              setLanguageDropdownOpen(false);
                              setLanguageSearch("");
                            }
                          }}
                          placeholder={t(
                            "settings.general.language.searchPlaceholder",
                          )}
                          className="w-full px-2 py-1 text-sm bg-mid-gray/10 border border-mid-gray/40 rounded-md focus:outline-none focus:ring-1 focus:ring-logo-primary"
                        />
                      </div>
                      <div className="max-h-48 overflow-y-auto">
                        <button
                          type="button"
                          onClick={() => {
                            setLanguageFilter("all");
                            setLanguageDropdownOpen(false);
                            setLanguageSearch("");
                          }}
                          className={`w-full px-3 py-1.5 text-sm text-left transition-colors ${
                            languageFilter === "all"
                              ? "bg-logo-primary/20 text-logo-primary font-semibold"
                              : "hover:bg-mid-gray/10"
                          }`}
                        >
                          {t("settings.models.filters.allLanguages")}
                        </button>
                        {filteredLanguages.map((lang) => (
                          <button
                            key={lang.value}
                            type="button"
                            onClick={() => {
                              setLanguageFilter(lang.value);
                              setLanguageDropdownOpen(false);
                              setLanguageSearch("");
                            }}
                            className={`w-full px-3 py-1.5 text-sm text-left transition-colors ${
                              languageFilter === lang.value
                                ? "bg-logo-primary/20 text-logo-primary font-semibold"
                                : "hover:bg-mid-gray/10"
                            }`}
                          >
                            {lang.label}
                          </button>
                        ))}
                        {filteredLanguages.length === 0 && (
                          <div className="px-3 py-2 text-sm text-text/50 text-center">
                            {t("settings.general.language.noResults")}
                          </div>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}
            {downloadedModels.map((model: ModelInfo) => (
              <ModelCard
                key={model.id}
                model={model}
                status={getModelStatus(model.id)}
                onSelect={handleModelSelect}
                onDownload={handleModelDownload}
                onDelete={handleModelDelete}
                onCancel={handleModelCancel}
                downloadProgress={getDownloadProgress(model.id)}
                downloadSpeed={getDownloadSpeed(model.id)}
                showRecommended={false}
                categoryLabel={getCategoryBadge(model)}
              />
            ))}
          </div>

          {/* Available Models Section */}
          {availableModels.length > 0 && (
            <div className="space-y-3">
              <h2 className="text-sm font-medium text-text/60">
                {t("settings.models.availableModels")}
              </h2>
              {availableModels.map((model: ModelInfo) => (
                <ModelCard
                  key={model.id}
                  model={model}
                  status={getModelStatus(model.id)}
                  onSelect={handleModelSelect}
                  onDownload={handleModelDownload}
                  onDelete={handleModelDelete}
                  onCancel={handleModelCancel}
                  downloadProgress={getDownloadProgress(model.id)}
                  downloadSpeed={getDownloadSpeed(model.id)}
                  showRecommended={false}
                  categoryLabel={getCategoryBadge(model)}
                />
              ))}
            </div>
          )}
        </div>
      ) : (
        <div className="text-center py-8 text-text/50">
          {t("settings.models.noModelsMatch")}
        </div>
      )}
    </div>
  );
};
