import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "@/components/ui/SettingsGroup";
import { SettingContainer } from "@/components/ui/SettingContainer";
import { ToggleSwitch } from "@/components/ui/ToggleSwitch";
import ExportFormatPicker from "@/components/editor/ExportFormatPicker";
import { ExportGroup } from "@/components/settings/advanced/ExportGroup";
import type {
  AllowedExportFormat,
  AudioExportFormat,
  Word,
} from "@/bindings";

interface EditorToolbarProps {
  words: Word[];
  formatOverride: AudioExportFormat | null;
  onFormatOverrideChange: (next: AudioExportFormat | null) => void;
  allowedFormats: AllowedExportFormat[];
  defaultExportFormat: AudioExportFormat;
  exportPickerDisabled?: boolean;
  burnCaptions: boolean;
  onBurnCaptionsChange: (next: boolean) => void;
  normalizeAudio: boolean;
  onNormalizeAudioToggle: () => void;
}

/**
 * Export settings panel. Shown alongside the editor when words are
 * loaded. Export triggers (SRT / VTT / Script / FFmpeg / edited media)
 * live in the header `<ExportMenu>` — this component owns only the
 * knobs that affect the next export: format override, burn captions,
 * normalize audio, loudness target + preflight.
 */
const EditorToolbar: React.FC<EditorToolbarProps> = React.memo(({
  words,
  formatOverride,
  onFormatOverrideChange,
  allowedFormats,
  defaultExportFormat,
  exportPickerDisabled,
  burnCaptions,
  onBurnCaptionsChange,
  normalizeAudio,
  onNormalizeAudioToggle,
}) => {
  const { t } = useTranslation();

  if (words.length === 0) return null;

  return (
    <SettingsGroup title={t("editor.sections.exportSettings")}>
      <div className="space-y-1">
        <SettingContainer
          title={t("editor.exportFormat.label")}
          description={t("editor.exportFormat.description")}
          grouped
          layout="horizontal"
        >
          <ExportFormatPicker
            value={formatOverride}
            onChange={onFormatOverrideChange}
            options={allowedFormats}
            defaultFormat={defaultExportFormat}
            disabled={exportPickerDisabled}
          />
        </SettingContainer>

        <ToggleSwitch
          checked={burnCaptions}
          onChange={onBurnCaptionsChange}
          label={t("editor.addCaptions")}
          description={t("editor.addCaptionsDescription")}
          grouped
        />

        <ToggleSwitch
          checked={normalizeAudio}
          onChange={onNormalizeAudioToggle}
          label={t("editor.normalizeAudio")}
          description={t("editor.normalizeAudioDescription")}
          grouped
        />

        <ExportGroup />
      </div>
    </SettingsGroup>
  );
});

EditorToolbar.displayName = "EditorToolbar";

export default EditorToolbar;
