import React, { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import {
  FileVideo,
  Upload,
  FileText,
  Download,
  Trash2,
  Save,
  FolderOpen,
  Terminal,
  AudioLines,
} from "lucide-react";
import { SettingsGroup } from "@/components/ui/SettingsGroup";
import { useEditorStore } from "@/stores/editorStore";
import { usePlayerStore, type MediaInfo } from "@/stores/playerStore";
import TranscriptEditor from "@/components/editor/TranscriptEditor";
import MediaPlayer from "@/components/player/MediaPlayer";
import Waveform from "@/components/player/Waveform";

interface FillerAnalysis {
  filler_indices: number[];
  pauses: { after_word_index: number; gap_duration_us: number }[];
  filler_count: number;
  pause_count: number;
}

const EditorView: React.FC = () => {
  const { t } = useTranslation();
  const { words, setWords } = useEditorStore();
  const { mediaUrl, mediaType, currentTime, duration, setMedia } =
    usePlayerStore();
  const mediaInfo = usePlayerStore((s) => s.mediaInfo);
  const setMediaInfo = usePlayerStore((s) => s.setMediaInfo);
  const seekTo = usePlayerStore((s) => s.seekTo);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [fillerInfo, setFillerInfo] = useState<FillerAnalysis | null>(null);
  const [modelMissing, setModelMissing] = useState(false);

  const handleTranscribe = useCallback(async () => {
    if (!mediaInfo) return;
    setIsTranscribing(true);
    setModelMissing(false);
    try {
      const result = await invoke<
        Array<{
          text: string;
          start_us: number;
          end_us: number;
          deleted: boolean;
          silenced: boolean;
          confidence: number;
          speaker_id: number;
        }>
      >("transcribe_media_file", { path: mediaInfo.path });
      await setWords(result);
    } catch (err) {
      const errStr = String(err);
      if (errStr.includes("Model is not loaded")) {
        setModelMissing(true);
      } else {
        console.error("Transcription failed:", err);
        const placeholderWords = [
          {
            text: errStr,
            start_us: 0,
            end_us: 1000000,
            deleted: false,
            silenced: false,
            confidence: 1.0,
            speaker_id: -1,
          },
        ];
        await setWords(placeholderWords);
      }
    } finally {
      setIsTranscribing(false);
    }
  }, [mediaInfo, setWords]);

  const handleImportMedia = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: t("editor.mediaFiles"),
            extensions: [
              "mp4", "mkv", "webm", "avi", "mov", "wmv", "flv", "m4v",
              "mp3", "wav", "flac", "ogg", "aac", "m4a", "wma", "opus",
            ],
          },
        ],
      });

      if (!selected) return;

      const path = typeof selected === "string" ? selected : selected;
      const info = await invoke<MediaInfo>("media_import", { path });
      setMediaInfo(info);

      const assetUrl = convertFileSrc(info.path);
      setMedia(assetUrl, info.media_type === "Video" ? "video" : "audio");

      // Auto-transcribe if no words loaded yet
      // We call handleTranscribe after setting media — it will auto-load model if possible
      setTimeout(async () => {
        try {
          const storeInfo = usePlayerStore.getState().mediaInfo;
          if (storeInfo) {
            setIsTranscribing(true);
            setModelMissing(false);
            const result = await invoke<
              Array<{
                text: string;
                start_us: number;
                end_us: number;
                deleted: boolean;
                silenced: boolean;
                confidence: number;
                speaker_id: number;
              }>
            >("transcribe_media_file", { path: storeInfo.path });
            await setWords(result);
            setIsTranscribing(false);
          }
        } catch (err) {
          setIsTranscribing(false);
          if (String(err).includes("Model is not loaded")) {
            setModelMissing(true);
          }
        }
      }, 100);
    } catch (err) {
      console.error("Failed to import media:", err);
    }
  }, [t, setMedia, setMediaInfo, setWords]);

  const handleExport = useCallback(
    async (format: string) => {
      const ext = format === "Srt" ? "srt" : format === "Vtt" ? "vtt" : "txt";
      try {
        const filePath = await save({
          filters: [{ name: format, extensions: [ext] }],
          defaultPath: `transcript.${ext}`,
        });
        if (!filePath) return;
        await invoke("export_transcript_to_file", { format, path: filePath });
      } catch (err) {
        console.error("Export failed:", err);
      }
    },
    [],
  );

  const handleAnalyzeFillers = useCallback(async () => {
    try {
      const analysis = await invoke<FillerAnalysis>("analyze_fillers", {});
      setFillerInfo(analysis);
    } catch (err) {
      console.error("Filler analysis failed:", err);
    }
  }, []);

  const handleDeleteFillers = useCallback(async () => {
    try {
      const count = await invoke<number>("delete_fillers", {});
      setFillerInfo(null);
      if (count > 0) {
        const updated = await invoke<typeof words>("editor_get_words", {});
        await setWords(updated);
      }
    } catch (err) {
      console.error("Delete fillers failed:", err);
    }
  }, [setWords]);

  const handleSaveProject = useCallback(async () => {
    try {
      const filePath = await save({
        filters: [{ name: "Toaster Project", extensions: ["toaster"] }],
        defaultPath: `${mediaInfo?.file_name ?? "project"}.toaster`,
      });
      if (!filePath) return;
      await invoke("save_project", { path: filePath });
    } catch (err) {
      console.error("Save project failed:", err);
    }
  }, [mediaInfo]);

  const handleLoadProject = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Toaster Project", extensions: ["toaster"] }],
      });
      if (!selected) return;
      const path = typeof selected === "string" ? selected : selected;
      const mediaPath = await invoke<string>("load_project", { path });

      const loadedWords = await invoke<typeof words>("editor_get_words", {});
      await setWords(loadedWords);

      if (mediaPath) {
        const info = await invoke<MediaInfo | null>("media_get_current", {});
        if (info) {
          setMediaInfo(info);
          const assetUrl = convertFileSrc(info.path);
          setMedia(assetUrl, info.media_type === "Video" ? "video" : "audio");
        }
      }
    } catch (err) {
      console.error("Load project failed:", err);
    }
  }, [setWords, setMedia, setMediaInfo]);

  const handleFFmpegScript = useCallback(async () => {
    if (!mediaInfo) return;
    try {
      const script = await invoke<string>("generate_ffmpeg_edit_script", {
        inputPath: mediaInfo.path,
      });
      await navigator.clipboard.writeText(script);
    } catch (err) {
      console.error("FFmpeg script generation failed:", err);
    }
  }, [mediaInfo]);

  const handleTimeUpdate = useCallback(
    (time: number) => {
      if (words.length === 0) return;
      const timeUs = time * 1_000_000;
      const idx = words.findIndex(
        (w) => !w.deleted && timeUs >= w.start_us && timeUs <= w.end_us,
      );
      if (idx >= 0) {
        useEditorStore.getState().selectWord(idx);
      }
    },
    [words],
  );

  const handleWordClick = useCallback(
    (index: number) => {
      const word = words[index];
      if (word) {
        seekTo(word.start_us / 1_000_000);
      }
    },
    [words, seekTo],
  );

  return (
    <div className="max-w-4xl w-full mx-auto space-y-6">
      {/* Project section */}
      <SettingsGroup title={t("editor.sections.project")}>
        <div className="flex items-center gap-2 px-4 py-3">
          <button
            onClick={handleLoadProject}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
            title={t("editor.loadProject")}
          >
            <FolderOpen size={14} />
            {t("editor.open")}
          </button>
          {words.length > 0 && (
            <button
              onClick={handleSaveProject}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
              title={t("editor.saveProject")}
            >
              <Save size={14} />
              {t("editor.save")}
            </button>
          )}
        </div>
      </SettingsGroup>

      {/* Media section */}
      <SettingsGroup title={t("editor.sections.media")}>
        <div className="px-4 py-3 space-y-3">
          {!mediaUrl ? (
            <div
              className="border-2 border-dashed border-mid-gray/30 rounded-xl p-8 flex flex-col items-center justify-center gap-3 cursor-pointer hover:border-accent/50 transition-colors"
              onClick={handleImportMedia}
            >
              <Upload size={40} className="text-mid-gray/50" />
              <p className="text-sm text-mid-gray">{t("editor.importMedia")}</p>
              <p className="text-xs text-mid-gray/60">
                {t("editor.supportedFormats")}
              </p>
            </div>
          ) : (
            <>
              {/* File info bar */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <FileVideo size={16} className="text-accent" />
                  <span className="text-sm font-medium">
                    {mediaInfo?.file_name}
                  </span>
                  <span className="text-xs text-mid-gray">
                    {mediaInfo
                      ? `${(mediaInfo.file_size / 1024 / 1024).toFixed(1)} MB`
                      : ""}
                  </span>
                </div>
                <button
                  onClick={handleImportMedia}
                  className="text-xs text-mid-gray hover:text-foreground transition-colors px-2 py-1"
                >
                  {t("editor.changeFile")}
                </button>
              </div>

              {/* Player */}
              <MediaPlayer
                className="rounded-lg overflow-hidden"
                onTimeUpdate={handleTimeUpdate}
              />

              {/* Waveform */}
              <Waveform
                audioUrl={mediaUrl}
                currentTime={currentTime}
                duration={duration}
                onSeek={seekTo}
                className="rounded-lg overflow-hidden"
              />
            </>
          )}
        </div>
      </SettingsGroup>

      {/* Transcription section */}
      <SettingsGroup title={t("editor.sections.transcription")}>
        <div className="px-4 py-3 space-y-3">
          {mediaUrl && words.length === 0 && (
            <>
              <button
                onClick={handleTranscribe}
                disabled={isTranscribing}
                className="flex items-center gap-2 px-4 py-2 bg-accent text-black rounded-lg text-sm font-medium hover:bg-accent/90 transition-colors disabled:opacity-50"
              >
                <FileText size={16} />
                {isTranscribing
                  ? t("editor.transcribing")
                  : t("editor.transcribe")}
              </button>
              {modelMissing && (
                <p className="text-xs text-amber-400">
                  {t("editor.modelNotLoaded")}
                </p>
              )}
            </>
          )}

          {words.length > 0 && (
            <div className="bg-background border border-mid-gray/20 rounded-lg overflow-hidden">
              <TranscriptEditor />
            </div>
          )}

          {!mediaUrl && words.length === 0 && (
            <p className="text-sm text-mid-gray py-2">
              {t("editor.noTranscript")}
            </p>
          )}
        </div>
      </SettingsGroup>

      {/* Export & Tools section */}
      {words.length > 0 && (
        <SettingsGroup title={t("editor.sections.exportTools")}>
          <div className="px-4 py-3 space-y-3">
            <div className="flex items-center gap-2 flex-wrap">
              <button
                onClick={() => handleExport("Srt")}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
              >
                <Download size={14} />
                SRT
              </button>
              <button
                onClick={() => handleExport("Vtt")}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
              >
                <Download size={14} />
                VTT
              </button>
              <button
                onClick={() => handleExport("Script")}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
              >
                <Download size={14} />
                {t("editor.script")}
              </button>

              <div className="w-px h-5 bg-mid-gray/20 mx-1" />

              <button
                onClick={handleAnalyzeFillers}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
                title={t("editor.analyzeFillers")}
              >
                <AudioLines size={14} />
                {t("editor.fillers")}
              </button>

              {fillerInfo && fillerInfo.filler_count > 0 && (
                <button
                  onClick={handleDeleteFillers}
                  className="flex items-center gap-1.5 px-3 py-1.5 bg-red-900/30 border border-red-500/30 rounded-lg text-xs text-red-400 hover:bg-red-900/50 transition-colors"
                >
                  <Trash2 size={14} />
                  {t("editor.deleteFillers", {
                    count: fillerInfo.filler_count,
                  })}
                </button>
              )}

              <button
                onClick={handleFFmpegScript}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
                title={t("editor.ffmpegScript")}
              >
                <Terminal size={14} />
                FFmpeg
              </button>
            </div>

            {/* Filler analysis results */}
            {fillerInfo && (
              <p className="text-xs text-mid-gray">
                {t("editor.fillerResults", {
                  fillers: fillerInfo.filler_count,
                  pauses: fillerInfo.pause_count,
                })}
              </p>
            )}
          </div>
        </SettingsGroup>
      )}
    </div>
  );
};

export default EditorView;
