import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface VadPrefilterToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * R-006 UI — transcription-scoped toggle for the Silero VAD ASR pre-filter.
 * See `features/reintroduce-silero-vad/PRD.md` R-006 / AC-006-a.
 *
 * Read once per transcription job; the in-flight job is unaffected when
 * the user flips this mid-job. When the Silero ONNX is not present on
 * disk, the backend silently falls back to the full-file ASR path
 * (AC-005-c) — no error is surfaced to the user.
 */
export const VadPrefilterToggle: React.FC<VadPrefilterToggleProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = getSetting("vad_prefilter_enabled") ?? true;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(v) => updateSetting("vad_prefilter_enabled", v)}
        isUpdating={isUpdating("vad_prefilter_enabled")}
        label={t("settings.controls.vadPrefilter.label")}
        description={t("settings.controls.vadPrefilter.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });
