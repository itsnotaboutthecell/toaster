import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { DiscardWords } from "../DiscardWords";
import { AllowWords } from "../AllowWords";
import { ModelUnloadTimeoutSetting } from "../ModelUnloadTimeout";
import { CaptionSettings } from "../captions/CaptionSettings";
import { ExportGroup } from "./ExportGroup";
import { ExperimentalGroup } from "./ExperimentalGroup";

export const AdvancedSettings: React.FC = () => {
  const { t } = useTranslation();

  return (
    <div className="max-w-5xl w-full mx-auto space-y-6" data-testid="settings-outer">
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

