import React from "react";
import { useTranslation } from "react-i18next";
import { useModelStore } from "@/stores/modelStore";
import { useSettingsStore } from "@/stores/settingsStore";

const SILERO_VAD_ID = "silero-vad";

/**
 * Footer pill that mirrors the ModelStatusButton visual language for the
 * Silero VAD model. Visible only when:
 *   - the Silero ONNX is downloaded, AND
 *   - at least one VAD consumer toggle is enabled
 *     (vad_prefilter_enabled || vad_refine_boundaries).
 *
 * Kept intentionally minimal (read-only pill, no dropdown). The Models
 * settings tab remains the download entry point; this component is the
 * "active / downloading / verifying" runtime surface per the user's
 * footer-first UX direction.
 */
export const VadStatusPill: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const getModelInfo = useModelStore((s) => s.getModelInfo);
  const isModelDownloading = useModelStore((s) => s.isModelDownloading);
  const isModelVerifying = useModelStore((s) => s.isModelVerifying);
  const getDownloadProgress = useModelStore((s) => s.getDownloadProgress);
  const getSetting = useSettingsStore((s) => s.getSetting);

  const prefilterEnabled = getSetting("vad_prefilter_enabled") ?? true;
  const refineEnabled = getSetting("vad_refine_boundaries") ?? false;
  const consumerActive = prefilterEnabled || refineEnabled;

  const model = getModelInfo(SILERO_VAD_ID);
  const downloading = isModelDownloading(SILERO_VAD_ID);
  const verifying = isModelVerifying(SILERO_VAD_ID);

  if (!consumerActive) return null;

  if (verifying) {
    return (
      <div
        className="flex items-center gap-2 text-text/60"
        role="status"
        data-testid="vad-status-pill"
      >
        <div className="w-2 h-2 rounded-full bg-orange-400 animate-pulse" />
        <span>{t("modelSelector.vad.verifying")}</span>
      </div>
    );
  }

  if (downloading) {
    const progress = getDownloadProgress(SILERO_VAD_ID);
    const pct = Math.round(progress?.percentage ?? 0);
    return (
      <div
        className="flex items-center gap-2 text-text/60"
        role="status"
        data-testid="vad-status-pill"
      >
        <div className="w-2 h-2 rounded-full bg-logo-primary animate-pulse" />
        <span>{t("modelSelector.vad.downloading", { percent: pct })}</span>
      </div>
    );
  }

  if (model?.is_downloaded) {
    return (
      <div
        className="flex items-center gap-2 text-text/60"
        data-testid="vad-status-pill"
      >
        <div className="w-2 h-2 rounded-full bg-green-400" />
        <span>{t("modelSelector.vad.active")}</span>
      </div>
    );
  }

  return null;
});

VadStatusPill.displayName = "VadStatusPill";

export default VadStatusPill;
