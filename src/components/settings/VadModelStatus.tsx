import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useModelStore } from "@/stores/modelStore";

const SILERO_VAD_ID = "silero-vad";

/**
 * Inline affordance that lives beneath the VAD settings toggles and
 * surfaces the download state of the Silero VAD ONNX. The consumers
 * (prefilter, boundary snap, filler classifier) no-op when the file
 * is missing (AC-005-c graceful absence), so this component is
 * informational only — flipping the toggles without the model
 * present is allowed but has no runtime effect.
 *
 * States:
 *   - Absent  -> "Model required" row with a Download button.
 *   - Downloading -> progress percentage.
 *   - Verifying -> "Verifying..." row.
 *   - Ready   -> single green "Ready" row.
 */
export const VadModelStatus: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const {
    getModelInfo,
    isModelDownloading,
    isModelVerifying,
    getDownloadProgress,
    downloadModel,
    initialized,
    loadModels,
  } = useModelStore();

  useEffect(() => {
    if (!initialized) {
      void loadModels();
    }
  }, [initialized, loadModels]);

  const model = getModelInfo(SILERO_VAD_ID);
  const downloading = isModelDownloading(SILERO_VAD_ID);
  const verifying = isModelVerifying(SILERO_VAD_ID);
  const progress = getDownloadProgress(SILERO_VAD_ID);

  const handleDownload = async () => {
    await downloadModel(SILERO_VAD_ID);
  };

  if (verifying) {
    return (
      <p className="text-xs text-text-muted" role="status">
        {t("settings.controls.vadModel.verifying")}
      </p>
    );
  }

  if (downloading) {
    const pct = Math.round(progress?.percentage ?? 0);
    return (
      <p className="text-xs text-text-muted" role="status">
        {t("settings.controls.vadModel.downloading", { percent: pct })}
      </p>
    );
  }

  if (model?.is_downloaded) {
    return (
      <p className="text-xs text-text-muted">
        {t("settings.controls.vadModel.ready")}
      </p>
    );
  }

  return (
    <div className="flex items-center gap-3">
      <p className="text-xs text-text-muted">
        {t("settings.controls.vadModel.required")}
      </p>
      <button
        type="button"
        onClick={handleDownload}
        disabled={!model}
        className="text-xs px-2 py-1 rounded bg-surface-interactive hover:bg-surface-interactive-hover disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {t("settings.controls.vadModel.download", {
          sizeMb: model?.size_mb ?? 2,
        })}
      </button>
    </div>
  );
});

VadModelStatus.displayName = "VadModelStatus";
