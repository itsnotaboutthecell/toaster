import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { save } from "@tauri-apps/plugin-dialog";
import {
  commands,
  type AllowedExportFormat,
  type AppSettings,
  type AudioExportFormat,
  type ExportFormat,
  type MediaInfo,
} from "@/bindings";
import { unwrapResult } from "@/components/editor/EditorView.util";

interface UseEditorExportsArgs {
  mediaInfo: MediaInfo | null;
  settings: AppSettings | null;
  burnCaptions: boolean;
}

/**
 * Owns every export-related piece of state + the three export handlers
 * (transcript / edited media / FFmpeg script) and the effect that keeps
 * `allowedFormats` in sync with the loaded media.
 *
 * Extracted from EditorView per Round-5 KISS K9 — keeps the top-level
 * component focused on layout + lifecycle.
 */
export function useEditorExports({
  mediaInfo,
  settings,
  burnCaptions,
}: UseEditorExportsArgs) {
  const { t } = useTranslation();
  const [isExportingMedia, setIsExportingMedia] = useState(false);
  const [formatOverride, setFormatOverride] =
    useState<AudioExportFormat | null>(null);
  const [allowedFormats, setAllowedFormats] = useState<AllowedExportFormat[]>(
    [],
  );

  // Fetch source-compatible formats whenever the loaded media changes so
  // the ExportFormatPicker options and extension-lookup stay authoritative
  // (AC-003-a, AC-004-a).
  useEffect(() => {
    if (!mediaInfo) {
      setAllowedFormats([]);
      setFormatOverride(null);
      return;
    }
    const ext = mediaInfo.extension ?? "";
    let cancelled = false;
    (async () => {
      try {
        const result = await commands.listAllowedExportFormats(ext);
        if (!cancelled) {
          setAllowedFormats(result);
          setFormatOverride(null);
        }
      } catch (err) {
        console.error("Failed to list allowed export formats:", err);
        if (!cancelled) setAllowedFormats([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [mediaInfo]);

  const defaultExportFormat: AudioExportFormat =
    settings?.export_format ?? "mp4";

  const handleExport = useCallback(async (format: ExportFormat) => {
    const ext = format === "Srt" ? "srt" : format === "Vtt" ? "vtt" : "txt";
    try {
      const filePath = await save({
        filters: [{ name: format, extensions: [ext] }],
        defaultPath: `transcript.${ext}`,
      });
      if (!filePath) return;
      unwrapResult(
        await commands.exportTranscriptToFile(format, filePath, null, null),
      );
    } catch (err) {
      console.error("Export failed:", err);
    }
  }, []);

  const handleFFmpegScript = useCallback(async () => {
    if (!mediaInfo) return;
    try {
      const script = unwrapResult(
        await commands.generateFfmpegEditScript(mediaInfo.path),
      );
      await navigator.clipboard.writeText(script);
    } catch (err) {
      console.error("FFmpeg script generation failed:", err);
    }
  }, [mediaInfo]);

  const handleExportEditedMedia = useCallback(async () => {
    if (!mediaInfo) return;

    const effectiveFormat: AudioExportFormat =
      formatOverride ?? defaultExportFormat;
    const allowedMatch = allowedFormats.find(
      (f) => f.format === effectiveFormat,
    );
    // Backend payload carries the canonical extension with a leading dot
    // (AC-005-a). Fall back to the format string if the list hasn't loaded
    // yet; source-derived extension is intentionally NOT used — that was
    // the pre-override bug (PRD §1).
    const extension = (
      allowedMatch?.extension.replace(/^\./, "") ?? effectiveFormat
    ).toLowerCase();
    const baseName = mediaInfo.file_name.replace(/\.[^/.]+$/, "");

    try {
      const filePath = await save({
        filters: [
          {
            name:
              mediaInfo.media_type === "Video"
                ? t("editor.editedVideo")
                : t("editor.editedAudio"),
            extensions: [extension],
          },
        ],
        defaultPath: `${baseName}-edited.${extension}`,
      });
      if (!filePath) return;
      setIsExportingMedia(true);
      unwrapResult(
        await commands.exportEditedMedia(
          mediaInfo.path,
          filePath,
          burnCaptions || null,
          formatOverride,
        ),
      );
    } catch (err) {
      console.error("Edited media export failed:", err);
    } finally {
      setIsExportingMedia(false);
    }
  }, [
    mediaInfo,
    t,
    burnCaptions,
    formatOverride,
    defaultExportFormat,
    allowedFormats,
  ]);

  return {
    handleExport,
    handleFFmpegScript,
    handleExportEditedMedia,
    isExportingMedia,
    formatOverride,
    setFormatOverride,
    allowedFormats,
    defaultExportFormat,
  };
}
