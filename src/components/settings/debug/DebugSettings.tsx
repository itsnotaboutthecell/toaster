import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { DebugPaths } from "./DebugPaths";
import { LogDirectory } from "./LogDirectory";
import { LogLevelSelector } from "./LogLevelSelector";
import { WordCorrectionThreshold } from "./WordCorrectionThreshold";

/**
 * Debug-mode settings panel.
 *
 * Composes the four existing debug components that previously only
 * surfaced piecemeal inside other panels (AboutSettings embeds
 * LogDirectory). Gated by `settings.debug_mode` in Sidebar's
 * `SECTIONS_CONFIG` — toggled via Ctrl+Shift+D in App.tsx.
 *
 * Part of `unreachable-surface-purge` R-005: the `sidebar.debug` and
 * `settings.debug.*` i18n keys were live but had no panel to mount in.
 */
export const DebugSettings: React.FC = () => {
  const { t } = useTranslation();

  return (
    <div className="max-w-5xl w-full mx-auto space-y-6" data-testid="debug-settings-outer">
      <div className="mb-4">
        <h1 className="text-xl font-semibold mb-2">
          {t("settings.debug.title")}
        </h1>
        <p className="text-sm text-text/60">
          {t("settings.debug.description")}
        </p>
      </div>
      <SettingsGroup>
        <LogDirectory grouped />
        <LogLevelSelector grouped />
        <WordCorrectionThreshold grouped />
        <DebugPaths grouped />
      </SettingsGroup>
    </div>
  );
};
