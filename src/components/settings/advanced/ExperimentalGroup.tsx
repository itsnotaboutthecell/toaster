import React from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink } from "lucide-react";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { Alert } from "../../ui/Alert";
import { useSettings } from "../../../hooks/useSettings";
import { experiments } from "@/lib/experiments";
import { PostProcessingSettingsPrompts } from "../post-processing/PostProcessingSettingsPrompts";

/**
 * Experimental group body for the Advanced page.
 *
 * Renders a master `experimental_enabled` ToggleSwitch that gates the
 * per-flag list. Defence-in-depth for *reads* lives in
 * `useExperiment` / `is_experiment_enabled`; this component only
 * controls whether the per-flag UI is visible. Stored per-flag
 * values are never cleared when the master flips off, so the user's
 * prior opt-ins come back when they re-enable the master (see
 * BLUEPRINT.md R-005/R-006).
 *
 * Expert mode (`ui_expert_mode_enabled`) lives inside the master-on
 * block because the LLM post-processing controls it reveals are not
 * yet a confident first-class feature — treat it as experimental
 * until that changes.
 */
export const ExperimentalGroup: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const masterEnabled =
    (getSetting("experimental_enabled") as boolean) ?? false;
  const expertModeEnabled =
    (getSetting("ui_expert_mode_enabled") as boolean) ?? false;

  const handleOpenFeedback = (url: string) => {
    void openUrl(url).catch((error) => {
      console.error("Failed to open feedback URL:", error);
    });
  };

  return (
    <div className="space-y-4">
      <ToggleSwitch
        checked={masterEnabled}
        onChange={(value) => updateSetting("experimental_enabled", value)}
        isUpdating={isUpdating("experimental_enabled")}
        label={t("settings.advanced.experimentalMaster.title")}
        description={t("settings.advanced.experimentalMaster.description")}
        grouped
      />

      {masterEnabled && (
        <>
          <Alert variant="warning">{t("settings.experimental.banner")}</Alert>
          <ToggleSwitch
            checked={expertModeEnabled}
            onChange={(value) => updateSetting("ui_expert_mode_enabled", value)}
            isUpdating={isUpdating("ui_expert_mode_enabled")}
            label={t("settings.advanced.expertMode.title")}
            description={t("settings.advanced.expertMode.description")}
            grouped
          />
          {expertModeEnabled && <PostProcessingSettingsPrompts />}
          {experiments.map((experiment) => {
            const checked =
              (getSetting(experiment.settingsKey) as boolean) ?? false;
            return (
              <ToggleSwitch
                key={experiment.id}
                checked={checked}
                onChange={(value) =>
                  updateSetting(experiment.settingsKey, value)
                }
                isUpdating={isUpdating(experiment.settingsKey)}
                label={t(experiment.labelKey)}
                description={t(experiment.descriptionKey)}
                grouped
                rightAdornment={
                  <button
                    type="button"
                    onClick={() => handleOpenFeedback(experiment.feedbackUrl)}
                    className="inline-flex items-center gap-1 text-xs text-logo-primary hover:underline"
                  >
                    {t("experiments.feedbackLink")}
                    <ExternalLink className="w-3 h-3" />
                  </button>
                }
              />
            );
          })}
        </>
      )}
    </div>
  );
};

