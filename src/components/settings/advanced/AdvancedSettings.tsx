import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { DiscardWords } from "../DiscardWords";
import { AllowWords } from "../AllowWords";
import { ModelUnloadTimeoutSetting } from "../ModelUnloadTimeout";
import { CaptionSettings } from "../captions/CaptionSettings";
import { ExperimentalGroup } from "./ExperimentalGroup";
import { VadPrefilterToggle } from "../VadPrefilterToggle";
import { VadRefineBoundariesToggle } from "../VadRefineBoundariesToggle";
import { VadModelStatus } from "../VadModelStatus";

export const AdvancedSettings: React.FC = () => {
  const { t } = useTranslation();

  return (
    <div className="max-w-5xl w-full mx-auto space-y-6" data-testid="settings-outer">
      <div className="mb-4">
        <h1 className="text-xl font-semibold mb-2">
          {t("settings.advanced.title")}
        </h1>
        <p className="text-sm text-text/60">
          {t("settings.advanced.description")}
        </p>
      </div>

      <SettingsGroup title={t("settings.advanced.groups.words.title")}>
        <DiscardWords descriptionMode="tooltip" grouped />
        <AllowWords descriptionMode="tooltip" grouped />
      </SettingsGroup>

      <SettingsGroup title={t("settings.advanced.groups.performance.title")}>
        <ModelUnloadTimeoutSetting descriptionMode="tooltip" grouped />
      </SettingsGroup>

      <SettingsGroup title={t("settings.advanced.groups.captions.title")}>
        <CaptionSettings descriptionMode="tooltip" grouped />
      </SettingsGroup>

      <SettingsGroup title={t("settings.advanced.groups.vad.title")}>
        <VadPrefilterToggle descriptionMode="tooltip" grouped />
        <VadRefineBoundariesToggle descriptionMode="tooltip" grouped />
        <VadModelStatus />
      </SettingsGroup>

      <SettingsGroup title={t("settings.advanced.groups.experimental.title")}>
        <ExperimentalGroup />
      </SettingsGroup>
    </div>
  );
};
