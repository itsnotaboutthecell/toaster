import React from "react";
import { useTranslation } from "react-i18next";
import {
  Download,
  Terminal,
} from "lucide-react";
import { SettingsGroup } from "@/components/ui/SettingsGroup";
import ExportFormatPicker from "@/components/editor/ExportFormatPicker";
import type {
  AllowedExportFormat,
  AudioExportFormat,
  ExportFormat,
  Word,
} from "@/bindings";

interface EditorToolbarProps {
  words: Word[];
  onExport: (format: ExportFormat) => void;
  onFFmpegScript: () => void;
  formatOverride: AudioExportFormat | null;
  onFormatOverrideChange: (next: AudioExportFormat | null) => void;
  allowedFormats: AllowedExportFormat[];
  defaultExportFormat: AudioExportFormat;
  exportPickerDisabled?: boolean;
}

const EditorToolbar: React.FC<EditorToolbarProps> = React.memo(({
  words,
  onExport,
  onFFmpegScript,
  formatOverride,
  onFormatOverrideChange,
  allowedFormats,
  defaultExportFormat,
  exportPickerDisabled,
}) => {
  const { t } = useTranslation();

  if (words.length === 0) return null;

  return (
    <SettingsGroup title={t("editor.sections.exportTools")}>
      <div className="px-4 py-3 space-y-3">
        {/* Edited-media export format */}
        <div>
          <p className="text-[10px] uppercase tracking-wider text-mid-gray/60 mb-1.5">
            {t("editor.exportFormat.label")}
          </p>
          <ExportFormatPicker
            value={formatOverride}
            onChange={onFormatOverrideChange}
            options={allowedFormats}
            defaultFormat={defaultExportFormat}
            disabled={exportPickerDisabled}
          />
        </div>

        {/* Export formats */}
        <div>
          <p className="text-[10px] uppercase tracking-wider text-mid-gray/60 mb-1.5">
            {t("editor.exportFormats")}
          </p>
          <div className="flex items-center gap-2 flex-wrap">
            <button
              onClick={() => onExport("Srt")}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
            >
              <Download size={14} />
              SRT
            </button>
            <button
              onClick={() => onExport("Vtt")}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
            >
              <Download size={14} />
              VTT
            </button>
            <button
              onClick={() => onExport("Script")}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
            >
              <Download size={14} />
              {t("editor.script")}
            </button>
          </div>
        </div>

        {/* Tools */}
        <div>
          <p className="text-[10px] uppercase tracking-wider text-mid-gray/60 mb-1.5">
            {t("editor.tools")}
          </p>
          <div className="flex items-center gap-2 flex-wrap">
            <button
              onClick={onFFmpegScript}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
              title={t("editor.ffmpegScript")}
            >
              <Terminal size={14} />
              {t("editor.ffmpegShortLabel")}
            </button>
          </div>
        </div>
      </div>
    </SettingsGroup>
  );
});

EditorToolbar.displayName = "EditorToolbar";

export default EditorToolbar;
