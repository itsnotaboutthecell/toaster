import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { DiscardWords } from "../DiscardWords";
import { AllowWords } from "../AllowWords";
import { ModelUnloadTimeoutSetting } from "../ModelUnloadTimeout";
import { CaptionSettings } from "../captions/CaptionSettings";
import { ExportGroup } from "./ExportGroup";
import { ExperimentalGroup } from "./ExperimentalGroup";
import { LLMConnectionGroup } from "./LLMConnectionGroup";
import { useSettings } from "../../../hooks/useSettings";

export const AdvancedSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const expertModeEnabled =
    (getSetting("ui_expert_mode_enabled") as boolean) ?? false;

  return (
    <div className="max-w-5xl w-full mx-auto space-y-6" data-testid="settings-outer">
      <SettingsGroup
        title={t("settings.advanced.groups.expertMode.title")}
        description={t("settings.advanced.groups.expertMode.description")}
      >
        <ToggleSwitch
          checked={expertModeEnabled}
          onChange={(value) => updateSetting("ui_expert_mode_enabled", value)}
          isUpdating={isUpdating("ui_expert_mode_enabled")}
          label={t("settings.advanced.expertMode.title")}
          description={t("settings.advanced.expertMode.description")}
          descriptionMode="tooltip"
          grouped
        />
      </SettingsGroup>

      {expertModeEnabled && (
        <SettingsGroup
          title={t("settings.advanced.groups.llmConnection.title")}
          description={t("settings.advanced.groups.llmConnection.description")}
        >
          <LLMConnectionGroup />
        </SettingsGroup>
      )}

      <SettingsGroup
        title={t("settings.advanced.groups.words.title")}
        description={t("settings.advanced.groups.words.description")}
      >
        <DiscardWords descriptionMode="tooltip" grouped />
        <AllowWords descriptionMode="tooltip" grouped />
      </SettingsGroup>

      <SettingsGroup
        title={t("settings.advanced.groups.performance.title")}
        description={t("settings.advanced.groups.performance.description")}
      >
        <ModelUnloadTimeoutSetting descriptionMode="tooltip" grouped />
      </SettingsGroup>

      <SettingsGroup
        title={t("settings.advanced.groups.captions.title")}
        description={t("settings.advanced.groups.captions.description")}
      >
        <CaptionSettings descriptionMode="tooltip" grouped />
      </SettingsGroup>

      <SettingsGroup
        title={t("settings.advanced.groups.export.title")}
        description={t("settings.advanced.groups.export.description")}
      >
        <ExportGroup />
      </SettingsGroup>

      <SettingsGroup
        title={t("settings.advanced.groups.experimental.title")}
        description={t("settings.advanced.groups.experimental.description")}
      >
        <ExperimentalGroup />
      </SettingsGroup>
    </div>
  );
};

