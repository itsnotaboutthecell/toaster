import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { PostProcessingToggle } from "../PostProcessingToggle";
import { PostProcessingSettingsPrompts } from "./PostProcessingSettingsPrompts";
import { ProviderSelect } from "../post-processing-api/ProviderSelect";
import { BaseUrlField } from "../post-processing-api/BaseUrlField";
import { ApiKeyField } from "../post-processing-api/ApiKeyField";
import { ModelSelect } from "../post-processing-api/ModelSelect";
import { usePostProcessProviderState } from "../post-processing-api/usePostProcessProviderState";
import { Button } from "../../ui/Button";
import { Alert } from "../../ui/Alert";
import { LlmModelCatalog } from "./local-models/LlmModelCatalog";
import { useSettings } from "../../../hooks/useSettings";

const LOCAL_GGUF_PROVIDER_ID = "local";

export const PostProcessingSettings: React.FC = () => {
  const { t } = useTranslation();
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

  const requiresApiKey = selectedProvider?.requires_api_key ?? true;
  const allowBaseUrlEdit = selectedProvider?.allow_base_url_edit ?? false;
  const isLocalProvider = selectedProviderId === LOCAL_GGUF_PROVIDER_ID;

  return (
    <div className="max-w-5xl w-full mx-auto space-y-6" data-testid="settings-outer">
      {(!selectedProviderId ||
        (!isLocalProvider && !model) ||
        (isLocalProvider && !settings?.local_llm_model_id)) && (
        <Alert variant="info">
          <span className="font-medium">
            {t("settings.postProcessing.localLlmAlert.title")}
          </span>
          {" — "}
          {t("settings.postProcessing.localLlmAlert.body")}
        </Alert>
      )}
      <SettingsGroup title={t("settings.postProcessing.title")}>
        <SettingContainer
          title={t("settings.debug.postProcessingToggle.label")}
          description={t("settings.debug.postProcessingToggle.description")}
          grouped={true}
        >
          <PostProcessingToggle descriptionMode="tooltip" grouped={false} />
        </SettingContainer>

        <SettingContainer
          title={t("settings.postProcessing.api.provider.title")}
          description={t("settings.postProcessing.api.provider.description")}
          grouped={true}
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
            grouped={true}
            layout="stacked"
          >
            <LlmModelCatalog />
          </SettingContainer>
        ) : (
          <>
            <SettingContainer
              title={t("settings.postProcessing.api.baseUrl.title")}
              description={t("settings.postProcessing.api.baseUrl.description")}
              grouped={true}
            >
              <BaseUrlField
                value={baseUrl}
                onBlur={handleBaseUrlChange}
                disabled={!allowBaseUrlEdit || isBaseUrlUpdating}
                placeholder={t(
                  "settings.postProcessing.api.baseUrl.placeholder",
                )}
              />
            </SettingContainer>

            {requiresApiKey && (
              <SettingContainer
                title={t("settings.postProcessing.api.apiKey.title")}
                description={t(
                  "settings.postProcessing.api.apiKey.description",
                )}
                grouped={true}
              >
                <ApiKeyField
                  value={apiKey}
                  onBlur={handleApiKeyChange}
                  disabled={isApiKeyUpdating}
                  placeholder={t(
                    "settings.postProcessing.api.apiKey.placeholder",
                  )}
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
              grouped={true}
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
                      ? t(
                          "settings.postProcessing.api.model.placeholderWithOptions",
                        )
                      : t(
                          "settings.postProcessing.api.model.placeholderNoOptions",
                        )
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
      </SettingsGroup>

      <SettingsGroup title={t("settings.postProcessing.prompts.title")}>
        <PostProcessingSettingsPrompts />
      </SettingsGroup>
    </div>
  );
};
