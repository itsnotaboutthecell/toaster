import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingContainer } from "../../ui/SettingContainer";
import { Dropdown, type DropdownOption } from "../../ui/Dropdown";
import { Alert } from "../../ui/Alert";
import { useSettings } from "../../../hooks/useSettings";
import {
  commands,
  type AudioExportFormat,
  type LoudnessPreflight,
  type LoudnessTarget,
} from "@/bindings";

const TARGETS: LoudnessTarget[] = ["off", "podcast_-16", "streaming_-14"];
// Order matches PRD R-001 / AC-001-a: video first, then four audio-only
// presets. Frontend sends only the enum; backend owns codec/bitrate
// mapping (AGENTS.md "Single source of truth for dual-path logic").
const EXPORT_FORMATS: AudioExportFormat[] = ["mp4", "mp3", "wav", "m4a", "opus"];
const PREFLIGHT_DEBOUNCE_MS = 800;
const PREFLIGHT_WARNING_LU = 12;

function formatNumber(
  value: number | null | undefined,
  suffix: string,
): string {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return "—";
  }
  return `${value.toFixed(1)} ${suffix}`;
}

/**
 * Export group body for the Advanced page. Contains every control
 * that used to live on the retired "Export" sidebar page (format
 * preset + loudness target + preflight panel). The page-level
 * heading is intentionally absent — the parent `SettingsGroup` in
 * `AdvancedSettings.tsx` owns the group title.
 */
export const ExportGroup: React.FC = () => {
  const { t } = useTranslation();
  const { settings, updateSetting, isUpdating } = useSettings();
  const target: LoudnessTarget = settings?.loudness_target ?? "off";
  const exportFormat: AudioExportFormat = settings?.export_format ?? "mp4";

  const [preflight, setPreflight] = useState<LoudnessPreflight | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const debounceRef = useRef<number | null>(null);
  const requestIdRef = useRef(0);
  const hasMeasuredRef = useRef(false);

  const runPreflight = useCallback(async (forTarget: LoudnessTarget) => {
    const reqId = ++requestIdRef.current;
    setRunning(true);
    setError(null);
    try {
      const res = await commands.loudnessPreflight(forTarget);
      if (reqId !== requestIdRef.current) return;
      if (res.status === "ok") {
        setPreflight(res.data);
        hasMeasuredRef.current = true;
      } else {
        setError(res.error);
        setPreflight(null);
      }
    } catch (e) {
      if (reqId !== requestIdRef.current) return;
      setError(String(e));
      setPreflight(null);
    } finally {
      if (reqId === requestIdRef.current) setRunning(false);
    }
  }, []);

  // Auto-rerun preflight when the target changes, but only after the user has
  // run it manually at least once. Debounced to avoid spamming during quick
  // target switches.
  useEffect(() => {
    if (!hasMeasuredRef.current) return;
    if (debounceRef.current !== null) {
      window.clearTimeout(debounceRef.current);
    }
    debounceRef.current = window.setTimeout(() => {
      void runPreflight(target);
    }, PREFLIGHT_DEBOUNCE_MS);
    return () => {
      if (debounceRef.current !== null) {
        window.clearTimeout(debounceRef.current);
      }
    };
  }, [target, runPreflight]);

  const handleTargetChange = (value: string) => {
    const next = value as LoudnessTarget;
    if (next === target) return;
    void updateSetting("loudness_target", next);
  };

  const handleFormatChange = (value: string) => {
    const next = value as AudioExportFormat;
    if (next === exportFormat) return;
    void updateSetting("export_format", next);
  };

  const targetOptions: DropdownOption[] = TARGETS.map((value) => ({
    value,
    label: t(`settings.export.loudness.options.${value}.label`),
  }));

  const formatOptions: DropdownOption[] = EXPORT_FORMATS.map((value) => ({
    value,
    label: t(`settings.export.format.options.${value}.label`),
  }));

  const showWarning =
    preflight?.delta_lu !== null &&
    preflight?.delta_lu !== undefined &&
    Number.isFinite(preflight.delta_lu) &&
    Math.abs(preflight.delta_lu) > PREFLIGHT_WARNING_LU;

  return (
    <div className="space-y-4">
      <SettingContainer
        title={t("settings.export.format.label")}
        description={t("settings.export.format.description")}
        descriptionMode="inline"
        grouped
        layout="horizontal"
      >
        <Dropdown
          options={formatOptions}
          selectedValue={exportFormat}
          onSelect={handleFormatChange}
          disabled={!settings || isUpdating("export_format")}
        />
      </SettingContainer>
      <SettingContainer
        title={t("settings.export.loudness.title")}
        description={t("settings.export.loudness.description")}
        descriptionMode="inline"
        grouped
        layout="horizontal"
      >
        <Dropdown
          options={targetOptions}
          selectedValue={target}
          onSelect={handleTargetChange}
          disabled={!settings || isUpdating("loudness_target")}
        />
      </SettingContainer>
      <SettingContainer
        title={t("settings.export.loudness.preflight.title")}
        description={t("settings.export.loudness.preflight.description")}
        descriptionMode="inline"
        grouped
        layout="horizontal"
      >
        <div className="flex flex-col items-end gap-3 min-w-[16rem]">
          <button
            type="button"
            onClick={() => void runPreflight(target)}
            disabled={running}
            className="px-3 py-1.5 rounded-md bg-logo-primary text-sm font-medium text-white hover:bg-logo-primary/90 disabled:opacity-50"
          >
            {running
              ? t("settings.export.loudness.preflight.running")
              : t("settings.export.loudness.preflight.run")}
          </button>
          {error && (
            <Alert variant="warning">
              {t("settings.export.loudness.preflight.error", { error })}
            </Alert>
          )}
          {preflight && (
            <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm w-full">
              <div className="text-mid-gray">
                {t("settings.export.loudness.preflight.integrated")}
              </div>
              <div className="font-mono text-right">
                {formatNumber(preflight.integrated_lufs, "LUFS")}
              </div>
              <div className="text-mid-gray">
                {t("settings.export.loudness.preflight.truePeak")}
              </div>
              <div className="font-mono text-right">
                {formatNumber(preflight.true_peak_dbtp, "dBTP")}
              </div>
              <div className="text-mid-gray">
                {t("settings.export.loudness.preflight.lra")}
              </div>
              <div className="font-mono text-right">
                {formatNumber(preflight.lra, "LU")}
              </div>
              <div className="text-mid-gray">
                {t("settings.export.loudness.preflight.target")}
              </div>
              <div className="font-mono text-right">
                {preflight.target_lufs === null
                  ? t("settings.export.loudness.preflight.noTarget")
                  : formatNumber(preflight.target_lufs, "LUFS")}
              </div>
              <div className="text-mid-gray">
                {t("settings.export.loudness.preflight.delta")}
              </div>
              <div className="font-mono text-right">
                {formatNumber(preflight.delta_lu, "LU")}
              </div>
            </div>
          )}
          {showWarning && (
            <Alert variant="warning">
              {t("settings.export.loudness.preflight.warningOffTarget")}
            </Alert>
          )}
        </div>
      </SettingContainer>
    </div>
  );
};
