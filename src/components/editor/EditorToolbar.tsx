import React from "react";
import { useTranslation } from "react-i18next";
import {
  FileVideo,
  FileText,
  Download,
  Terminal,
} from "lucide-react";
import { SettingsGroup } from "@/components/ui/SettingsGroup";
import { useSettingsStore } from "@/stores/settingsStore";
import type { ExportFormat, MediaInfo, Word } from "@/bindings";

const FADE_OPTIONS = [0, 250, 500, 1000];

interface EditorToolbarProps {
  words: Word[];
  mediaInfo: MediaInfo | null;
  isExportingMedia: boolean;
  burnCaptions: boolean;
  onBurnCaptionsChange: (value: boolean) => void;
  onExport: (format: ExportFormat) => void;
  onExportEditedMedia: () => void;
  onFFmpegScript: () => void;
}

const EditorToolbar: React.FC<EditorToolbarProps> = React.memo(({
  words,
  mediaInfo,
  isExportingMedia,
  burnCaptions,
  onBurnCaptionsChange,
  onExport,
  onExportEditedMedia,
  onFFmpegScript,
}) => {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const updateSetting = useSettingsStore((s) => s.updateSetting);
  const normalizeAudio = settings?.normalize_audio_on_export ?? false;
  const volumeDb = settings?.export_volume_db ?? 0;
  const fadeInMs = settings?.export_fade_in_ms ?? 0;
  const fadeOutMs = settings?.export_fade_out_ms ?? 0;

  if (words.length === 0) return null;

  return (
    <SettingsGroup title={t("editor.sections.exportTools")}>
      <div className="px-4 py-3 space-y-3">
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
            <button
              onClick={onExportEditedMedia}
              disabled={!mediaInfo || isExportingMedia}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors disabled:opacity-50"
              title={t("editor.exportEditedMedia")}
            >
              <FileVideo size={14} />
              {isExportingMedia ? t("editor.exportingEditedMedia") : t("editor.exportEditedMedia")}
            </button>
            <button
              onClick={async () => {
                const text = words
                  .filter((w) => !w.deleted && !w.silenced)
                  .map((w) => w.text)
                  .join(" ");
                await navigator.clipboard.writeText(text);
              }}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
              title={t("editor.copyTranscript")}
            >
              <FileText size={14} />
              {t("editor.copyText")}
            </button>
          </div>
        </div>

        {/* Export options */}
        <div>
          <p className="text-[10px] uppercase tracking-wider text-mid-gray/60 mb-1.5">
            {t("editor.sections.exportOptions", "Options")}
          </p>
          <div className="space-y-2">
            <label className="flex items-center gap-2 text-xs cursor-pointer">
              <input
                type="checkbox"
                checked={normalizeAudio}
                onChange={(e) => updateSetting("normalize_audio_on_export", e.target.checked)}
                className="accent-primary"
              />
              {t("editor.normalizeAudio")}
            </label>
            {mediaInfo?.media_type === "Video" && (
              <label className="flex items-center gap-2 text-xs cursor-pointer">
                <input
                  type="checkbox"
                  checked={burnCaptions}
                  onChange={(e) => onBurnCaptionsChange(e.target.checked)}
                  className="accent-primary"
                />
                {t("editor.burnCaptions")}
              </label>
            )}

            {/* Volume slider */}
            <div className="flex items-center gap-2 text-xs">
              <span className="w-28 shrink-0">{t("editor.exportVolume")}</span>
              <input
                type="range"
                min={-12}
                max={12}
                step={1}
                value={volumeDb}
                onChange={(e) => updateSetting("export_volume_db", parseFloat(e.target.value))}
                className="flex-grow h-1.5 rounded-lg appearance-none cursor-pointer"
              />
              <span className="w-14 text-end text-mid-gray/80">
                {t("editor.exportVolumeFmt", { value: volumeDb > 0 ? `+${volumeDb}` : volumeDb })}
              </span>
            </div>

            {/* Fade in */}
            <div className="flex items-center gap-2 text-xs">
              <span className="w-28 shrink-0">{t("editor.exportFadeIn")}</span>
              <select
                value={fadeInMs}
                onChange={(e) => updateSetting("export_fade_in_ms", parseInt(e.target.value, 10))}
                className="flex-grow bg-background border border-mid-gray/20 rounded px-2 py-1 text-xs"
              >
                {FADE_OPTIONS.map((ms) => (
                  <option key={ms} value={ms}>
                    {ms === 0 ? t("editor.exportFadeNone") : t("editor.exportFadeMs", { value: ms })}
                  </option>
                ))}
              </select>
            </div>

            {/* Fade out */}
            <div className="flex items-center gap-2 text-xs">
              <span className="w-28 shrink-0">{t("editor.exportFadeOut")}</span>
              <select
                value={fadeOutMs}
                onChange={(e) => updateSetting("export_fade_out_ms", parseInt(e.target.value, 10))}
                className="flex-grow bg-background border border-mid-gray/20 rounded px-2 py-1 text-xs"
              >
                {FADE_OPTIONS.map((ms) => (
                  <option key={ms} value={ms}>
                    {ms === 0 ? t("editor.exportFadeNone") : t("editor.exportFadeMs", { value: ms })}
                  </option>
                ))}
              </select>
            </div>
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
