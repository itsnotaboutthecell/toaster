import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import { AppDataDirectory } from "../AppDataDirectory";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { LogDirectory } from "../debug";

export const AboutSettings: React.FC = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
        setVersion("0.1.0");
      }
    };

    fetchVersion();
  }, []);

  return (
    <div className="max-w-5xl w-full mx-auto space-y-6" data-testid="settings-outer">
      <div className="mb-4">
        <h1 className="text-xl font-semibold mb-2">
          {t("settings.about.title")}
        </h1>
        <p className="text-sm text-text/60">
          {t("settings.about.description")}
        </p>
      </div>
      <SettingsGroup>
        <AppLanguageSelector descriptionMode="tooltip" grouped={true} />
        <SettingContainer
          title={t("settings.about.version.title")}
          description={t("settings.about.version.description")}
          grouped={true}
        >
          {/* eslint-disable-next-line i18next/no-literal-string */}
          <span className="text-sm font-mono">v{version}</span>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.sourceCode.title")}
          description={t("settings.about.sourceCode.description")}
          grouped={true}
        >
          <Button
            variant="secondary"
            size="md"
            onClick={() => openUrl("https://github.com/itsnotaboutthecell/toaster")}
          >
            {t("settings.about.sourceCode.button")}
          </Button>
        </SettingContainer>

        <AppDataDirectory descriptionMode="tooltip" grouped={true} />
        <LogDirectory grouped={true} />
      </SettingsGroup>

      <SettingsGroup title={t("settings.about.acknowledgments.title")}>
        <p className="text-sm text-mid-gray px-6 py-4">
          {t("settings.about.acknowledgments.intro")}
        </p>

        <SettingContainer
          title={t("settings.about.acknowledgments.handy.title")}
          description={t("settings.about.acknowledgments.handy.description")}
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-mid-gray">
            {t("settings.about.acknowledgments.handy.details")}
          </div>
          <Button
            variant="secondary"
            size="sm"
            className="mt-2"
            onClick={() => openUrl("https://github.com/cjpais/Handy")}
          >
            {t("settings.about.acknowledgments.handy.button")}
          </Button>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.acknowledgments.tauri.title")}
          description={t("settings.about.acknowledgments.tauri.description")}
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-mid-gray">
            {t("settings.about.acknowledgments.tauri.details")}
          </div>
          <Button
            variant="secondary"
            size="sm"
            className="mt-2"
            onClick={() => openUrl("https://tauri.app/")}
          >
            {t("settings.about.acknowledgments.tauri.button")}
          </Button>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.acknowledgments.whisper.title")}
          description={t("settings.about.acknowledgments.whisper.description")}
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-mid-gray">
            {t("settings.about.acknowledgments.whisper.details")}
          </div>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.acknowledgments.whisperCpp.title")}
          description={t(
            "settings.about.acknowledgments.whisperCpp.description",
          )}
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-mid-gray">
            {t("settings.about.acknowledgments.whisperCpp.details")}
          </div>
          <Button
            variant="secondary"
            size="sm"
            className="mt-2"
            onClick={() => openUrl("https://github.com/ggerganov/whisper.cpp")}
          >
            {t("settings.about.acknowledgments.whisperCpp.button")}
          </Button>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.acknowledgments.ffmpeg.title")}
          description={t("settings.about.acknowledgments.ffmpeg.description")}
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-mid-gray">
            {t("settings.about.acknowledgments.ffmpeg.details")}
          </div>
          <Button
            variant="secondary"
            size="sm"
            className="mt-2"
            onClick={() => openUrl("https://ffmpeg.org/")}
          >
            {t("settings.about.acknowledgments.ffmpeg.button")}
          </Button>
        </SettingContainer>
      </SettingsGroup>
    </div>
  );
};
