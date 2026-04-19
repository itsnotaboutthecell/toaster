import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface VadRefineBoundariesToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * R-006 UI — editor-scoped toggle for VAD-biased splice boundary refinement.
 * See `features/reintroduce-silero-vad/PRD.md` R-006 / AC-006-b.
 *
 * Default is `false` so preview + export stay byte-identical to the
 * pre-feature zero-crossing + energy-valley snap (AC-003-d). When
 * enabled, the P(speech) curve is consulted as an additional bias at
 * snap time to reduce phoneme leak across splice seams.
 */
export const VadRefineBoundariesToggle: React.FC<VadRefineBoundariesToggleProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = getSetting("vad_refine_boundaries") ?? false;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(v) => updateSetting("vad_refine_boundaries", v)}
        isUpdating={isUpdating("vad_refine_boundaries")}
        label={t("settings.controls.vadRefineBoundaries.label")}
        description={t("settings.controls.vadRefineBoundaries.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });
