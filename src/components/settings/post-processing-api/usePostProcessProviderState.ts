import { useCallback, useEffect, useMemo, useState } from "react";
import { useSettings } from "../../../hooks/useSettings";
import { type PostProcessProvider } from "@/bindings";
import type { ModelOption } from "./types";
import type { DropdownOption } from "../../ui/Dropdown";

type PostProcessProviderState = {
  providerOptions: DropdownOption[];
  selectedProviderId: string;
  selectedProvider: PostProcessProvider | undefined;
  isCustomProvider: boolean;
  providerValidationError: string | null;
  baseUrl: string;
  handleBaseUrlChange: (value: string) => void;
  isBaseUrlUpdating: boolean;
  apiKey: string;
  handleApiKeyChange: (value: string) => void;
  isApiKeyUpdating: boolean;
  model: string;
  handleModelChange: (value: string) => void;
  modelOptions: ModelOption[];
  isModelUpdating: boolean;
  isFetchingModels: boolean;
  handleProviderSelect: (providerId: string) => void;
  handleModelSelect: (value: string) => void;
  handleModelCreate: (value: string) => void;
  handleRefreshModels: () => void;
};

const getErrorMessage = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
};

export const usePostProcessProviderState = (): PostProcessProviderState => {
  const {
    settings,
    isUpdating,
    setPostProcessProvider,
    updatePostProcessBaseUrl,
    updatePostProcessApiKey,
    updatePostProcessModel,
    fetchPostProcessModels,
    postProcessModelOptions,
  } = useSettings();

  // Settings are guaranteed to have providers after migration
  const providers = settings?.post_process_providers || [];

  const selectedProviderId = useMemo(() => {
    return settings?.post_process_provider_id || providers[0]?.id || "ollama";
  }, [providers, settings?.post_process_provider_id]);

  const selectedProvider = useMemo(() => {
    return (
      providers.find((provider) => provider.id === selectedProviderId) ||
      providers[0]
    );
  }, [providers, selectedProviderId]);

  const [providerValidationError, setProviderValidationError] = useState<
    string | null
  >(null);

  // Use settings directly as single source of truth
  const baseUrl = selectedProvider?.base_url ?? "";
  const apiKey = settings?.post_process_api_keys?.[selectedProviderId] ?? "";
  const model = settings?.post_process_models?.[selectedProviderId] ?? "";

  const providerOptions = useMemo<DropdownOption[]>(() => {
    return providers.map((provider) => ({
      value: provider.id,
      label: provider.label,
    }));
  }, [providers]);

  const handleProviderSelect = useCallback(
    async (providerId: string) => {
      setProviderValidationError(null);

      if (providerId === selectedProviderId) return;

      try {
        await setPostProcessProvider(providerId);
      } catch (error) {
        setProviderValidationError(getErrorMessage(error));
        return;
      }

      // Auto-fetch available models for the new provider so the model dropdown
      // reflects what's actually valid. Without this, a stale model value from
      // a previous provider/base_url can persist and silently 404 at runtime.
      // Skip when the provider isn't configured yet (no API key / empty base URL)
      // to avoid unnecessary backend errors.
      const provider = providers.find((p) => p.id === providerId);
      const apiKeyForProvider =
        settings?.post_process_api_keys?.[providerId] ?? "";
      const hasBaseUrl = (provider?.base_url ?? "").trim() !== "";
      const hasApiKey = apiKeyForProvider.trim() !== "";
      const requiresApiKey = provider?.requires_api_key ?? true;
      const canFetchModels = requiresApiKey ? hasApiKey : hasBaseUrl;

      if (canFetchModels) {
        try {
          await fetchPostProcessModels(providerId);
          setProviderValidationError(null);
        } catch (error) {
          setProviderValidationError(getErrorMessage(error));
        }
      }
    },
    [
      selectedProviderId,
      setPostProcessProvider,
      fetchPostProcessModels,
      providers,
      settings,
    ],
  );

  const handleBaseUrlChange = useCallback(
    (value: string) => {
      if (!selectedProvider || !selectedProvider.allow_base_url_edit) {
        return;
      }
      const trimmed = value.trim();
      if (trimmed && trimmed !== baseUrl) {
        void updatePostProcessBaseUrl(selectedProvider.id, trimmed)
          .then(() => setProviderValidationError(null))
          .catch((error) =>
            setProviderValidationError(getErrorMessage(error)),
          );
      }
    },
    [selectedProvider, baseUrl, updatePostProcessBaseUrl],
  );

  const handleApiKeyChange = useCallback(
    (value: string) => {
      const trimmed = value.trim();
      if (trimmed !== apiKey) {
        void updatePostProcessApiKey(selectedProviderId, trimmed)
          .then(() => setProviderValidationError(null))
          .catch((error) =>
            setProviderValidationError(getErrorMessage(error)),
          );
      }
    },
    [apiKey, selectedProviderId, updatePostProcessApiKey],
  );

  const handleModelChange = useCallback(
    (value: string) => {
      const trimmed = value.trim();
      if (trimmed !== model) {
        void updatePostProcessModel(selectedProviderId, trimmed)
          .then(() => setProviderValidationError(null))
          .catch((error) =>
            setProviderValidationError(getErrorMessage(error)),
          );
      }
    },
    [model, selectedProviderId, updatePostProcessModel],
  );

  const handleModelSelect = useCallback(
    (value: string) => {
      void updatePostProcessModel(selectedProviderId, value.trim())
        .then(() => setProviderValidationError(null))
        .catch((error) => setProviderValidationError(getErrorMessage(error)));
    },
    [selectedProviderId, updatePostProcessModel],
  );

  const handleModelCreate = useCallback(
    (value: string) => {
      void updatePostProcessModel(selectedProviderId, value.trim())
        .then(() => setProviderValidationError(null))
        .catch((error) => setProviderValidationError(getErrorMessage(error)));
    },
    [selectedProviderId, updatePostProcessModel],
  );

  const handleRefreshModels = useCallback(() => {
    setProviderValidationError(null);
    void fetchPostProcessModels(selectedProviderId).catch((error) =>
      setProviderValidationError(getErrorMessage(error)),
    );
  }, [fetchPostProcessModels, selectedProviderId]);

  useEffect(() => {
    setProviderValidationError(null);
  }, [selectedProviderId]);

  const availableModelsRaw = postProcessModelOptions[selectedProviderId] || [];

  const modelOptions = useMemo<ModelOption[]>(() => {
    const seen = new Set<string>();
    const options: ModelOption[] = [];

    const upsert = (value: string | null | undefined) => {
      const trimmed = value?.trim();
      if (!trimmed || seen.has(trimmed)) return;
      seen.add(trimmed);
      options.push({ value: trimmed, label: trimmed });
    };

    // Add available models from API
    for (const candidate of availableModelsRaw) {
      upsert(candidate);
    }

    // Ensure current model is in the list
    upsert(model);

    return options;
  }, [availableModelsRaw, model]);

  const isBaseUrlUpdating = isUpdating(
    `post_process_base_url:${selectedProviderId}`,
  );
  const isApiKeyUpdating = isUpdating(
    `post_process_api_key:${selectedProviderId}`,
  );
  const isModelUpdating = isUpdating(
    `post_process_model:${selectedProviderId}`,
  );
  const isFetchingModels = isUpdating(
    `post_process_models_fetch:${selectedProviderId}`,
  );

  const isCustomProvider = selectedProvider?.id === "custom";

  return {
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
    modelOptions,
    isModelUpdating,
    isFetchingModels,
    handleProviderSelect,
    handleModelSelect,
    handleModelCreate,
    handleRefreshModels,
  };
};
