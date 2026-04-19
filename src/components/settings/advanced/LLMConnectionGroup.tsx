import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { SettingContainer } from "../../ui/SettingContainer";
import { PostProcessingToggle } from "../PostProcessingToggle";
import { ProviderSelect } from "../post-processing-api/ProviderSelect";
import { BaseUrlField } from "../post-processing-api/BaseUrlField";
import { ApiKeyField } from "../post-processing-api/ApiKeyField";
import { ModelSelect } from "../post-processing-api/ModelSelect";
import { usePostProcessProviderState } from "../post-processing-api/usePostProcessProviderState";
import { Button } from "../../ui/Button";
import { Alert } from "../../ui/Alert";
import { useModelStore } from "../../../stores/modelStore";
import { useSettingsNavStore } from "../../../stores/settingsNavStore";
import { useSettings } from "../../../hooks/useSettings";

const LOCAL_GGUF_PROVIDER_ID = "local";

/**
 * LLM connection configuration for the AI-cleanup post-processor.
 *
 * Rendered inside AdvancedSettings only when `ui_expert_mode_enabled` is
 * true. Replaces the standalone Settings > Post-Processing tab's "API"
 * section: provider selector, local-model link / base URL / API key /
 * model selector / refresh, plus the AI-cleanup execution toggle itself
 * (`post_process_enabled` via PostProcessingToggle).
 *
 * The prompt editor (`PostProcessingSettingsPrompts`) does NOT live here —
 * it moves into the Editor page as a drawer so prompt iteration happens
 * alongside the transcript. See plan.md > pp-editor-drawer.
 *
 * This group reuses the exact same hook (`usePostProcessProviderState`)
 * and primitive components (`ProviderSelect` / `BaseUrlField` etc.) that
 * the soon-to-be-deleted Post-Processing tab uses, so there is no
 * behaviour fork. The delete of the old tab is a follow-up commit.
 */
export const LLMConnectionGroup: React.FC = () => {
  const { t } = useTranslation();
  const navigateToModels = useSettingsNavStore((s) => s.navigateToModels);
  const {
    providerOptions,
    selectedProviderId,
    selectedProvider,
    isCustomProvider,
    providerValidationError,
    baseUrl,
    handleBaseUrlChange,
    isBaseUrlUpdating,
    apiKey,
    handleApiKeyChange,
    isApiKeyUpdating,
    model,
    handleModelChange,
    handleModelSelect,
    handleModelCreate,
    modelOptions,
    isModelUpdating,
    isFetchingModels,
    handleProviderSelect,
    handleRefreshModels,
  } = usePostProcessProviderState();
  const { settings } = useSettings();
  const models = useModelStore((s) => s.models);

  const requiresApiKey = selectedProvider?.requires_api_key ?? true;
  const allowBaseUrlEdit = selectedProvider?.allow_base_url_edit ?? false;
  const isLocalProvider = selectedProviderId === LOCAL_GGUF_PROVIDER_ID;

  const activeLocalModelLabel = useMemo(() => {
    const id = settings?.local_llm_model_id;
    if (!id) return null;
    const match = models.find((m) => m.id === id);
    return match?.name ?? id;
  }, [models, settings?.local_llm_model_id]);

  const needsConfiguration =
    !selectedProviderId ||
    (!isLocalProvider && !model) ||
    (isLocalProvider && !settings?.local_llm_model_id);

  return (
    <div className="space-y-2">
      <p
        className="text-xs text-mid-gray px-4"
        data-testid="advanced-llm-local-only-notice"
      >
        {t("settings.postProcessing.localOnlyNotice")}
      </p>

      {needsConfiguration && (
        <div className="px-4">
          <Alert variant="info">
            <span className="font-medium">
              {t("settings.postProcessing.localLlmAlert.title")}
            </span>
            {" — "}
            {t("settings.postProcessing.localLlmAlert.body")}
          </Alert>
        </div>
      )}

      <PostProcessingToggle descriptionMode="tooltip" grouped />

      <SettingContainer
        title={t("settings.postProcessing.api.provider.title")}
        description={t("settings.postProcessing.api.provider.description")}
        grouped
      >
        <ProviderSelect
          options={providerOptions}
          value={selectedProviderId}
          onChange={handleProviderSelect}
        />
      </SettingContainer>

      {isLocalProvider ? (
        <SettingContainer
          title={t("settings.postProcessing.localModels.title")}
          description={t("settings.postProcessing.localModels.description")}
          grouped
        >
          <div className="flex items-center gap-3">
            <span
              className="text-sm truncate max-w-[20rem]"
              data-testid="advanced-llm-active-model-label"
            >
              {activeLocalModelLabel ??
                t("settings.postProcessing.localModels.empty")}
            </span>
            <Button
              variant="brand"
              size="sm"
              onClick={() => navigateToModels("PostProcessor")}
            >
              {t("settings.postProcessing.manageModelsLink")}
            </Button>
          </div>
        </SettingContainer>
      ) : (
        <>
          <SettingContainer
            title={t("settings.postProcessing.api.baseUrl.title")}
            description={t("settings.postProcessing.api.baseUrl.description")}
            grouped
          >
            <BaseUrlField
              value={baseUrl}
              onBlur={handleBaseUrlChange}
              disabled={!allowBaseUrlEdit || isBaseUrlUpdating}
              placeholder={t("settings.postProcessing.api.baseUrl.placeholder")}
            />
          </SettingContainer>

          {requiresApiKey && (
            <SettingContainer
              title={t("settings.postProcessing.api.apiKey.title")}
              description={t("settings.postProcessing.api.apiKey.description")}
              grouped
            >
              <ApiKeyField
                value={apiKey}
                onBlur={handleApiKeyChange}
                disabled={isApiKeyUpdating}
                placeholder={t("settings.postProcessing.api.apiKey.placeholder")}
              />
            </SettingContainer>
          )}

          <SettingContainer
            title={t("settings.postProcessing.api.model.title")}
            description={
              isCustomProvider
                ? t("settings.postProcessing.api.model.descriptionCustom")
                : t("settings.postProcessing.api.model.descriptionDefault")
            }
            grouped
            layout="stacked"
          >
            <div className="flex gap-2 items-center">
              <ModelSelect
                value={model}
                options={modelOptions}
                disabled={isModelUpdating}
                isLoading={isFetchingModels}
                onSelect={handleModelSelect}
                onCreate={handleModelCreate}
                onBlur={() => handleModelChange(model)}
                placeholder={
                  modelOptions.length > 0
                    ? t("settings.postProcessing.api.model.placeholderWithOptions")
                    : t("settings.postProcessing.api.model.placeholderNoOptions")
                }
              />
              <Button
                variant="secondary"
                size="md"
                onClick={handleRefreshModels}
                disabled={isFetchingModels}
              >
                {t("settings.postProcessing.api.model.refreshModels")}
              </Button>
            </div>
          </SettingContainer>
        </>
      )}

      {providerValidationError && (
        <div className="px-4 py-2">
          <Alert variant="error">{providerValidationError}</Alert>
        </div>
      )}
    </div>
  );
};
