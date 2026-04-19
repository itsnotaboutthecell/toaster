import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { open, save } from "@tauri-apps/plugin-dialog";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import {
  FileVideo,
  Upload,
  FileText,
  Save,
  FolderOpen,
  X,
  AudioLines,
  Captions,
  Volume2,
} from "lucide-react";
import { SettingsGroup } from "@/components/ui/SettingsGroup";
import { commands, type ExportFormat, type Result } from "@/bindings";
import { useEditorStore } from "@/stores/editorStore";
import { usePlayerStore } from "@/stores/playerStore";
import { useSettingsStore } from "@/stores/settingsStore";
import TranscriptEditor from "@/components/editor/TranscriptEditor";
import MediaPlayer from "@/components/player/MediaPlayer";
import Waveform from "@/components/player/Waveform";
import EditorToolbar from "@/components/editor/EditorToolbar";

const unwrapResult = <T,>(result: Result<T, string>): T => {
  if (result.status === "ok") {
    return result.data;
  }
  throw new Error(result.error);
};

const EditorView: React.FC = () => {
  const { t } = useTranslation();
  const { words, setWords, deleteWord, silenceWord, splitWord, undo, redo, deleteRange, selectWord, setSelectionRange, clearHighlights, refreshFromBackend } = useEditorStore();
  const selectedIndex = useEditorStore((s) => s.selectedIndex);
  const { mediaUrl, currentTime, duration, setMedia } =
    usePlayerStore();
  const mediaInfo = usePlayerStore((s) => s.mediaInfo);
  const setMediaInfo = usePlayerStore((s) => s.setMediaInfo);
  const clearMedia = usePlayerStore((s) => s.clearMedia);
  const seekTo = usePlayerStore((s) => s.seekTo);
  const settings = useSettingsStore((s) => s.settings);
  const updateSetting = useSettingsStore((s) => s.updateSetting);
  const normalizeAudio = settings?.normalize_audio_on_export ?? false;
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [isExportingMedia, setIsExportingMedia] = useState(false);
  const [burnCaptions, setBurnCaptions] = useState(false);
  const [isCleaningUp, setIsCleaningUp] = useState(false);
  const [modelMissing, setModelMissing] = useState(false);
  const [lastSavedPath, setLastSavedPath] = useState<string | null>(null);
  // Suppress auto-select briefly after a manual word click so it doesn't get overridden
  const manualClickRef = React.useRef(false);

  // Auto-save: save project every 30 seconds when words exist and a save path is known
  useEffect(() => {
    if (!lastSavedPath || words.length === 0) return;
    const timer = setInterval(async () => {
      try {
        unwrapResult(await commands.saveProject(lastSavedPath, null));
      } catch (err) {
        console.error("Auto-save failed:", err);
      }
    }, 30_000);
    return () => clearInterval(timer);
  }, [lastSavedPath, words]);

  // Global keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't capture when typing in input/textarea
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      const { setPlaying, isPlaying } = usePlayerStore.getState();
      const { selectedIndex: selIdx, selectionRange: selRange, highlightedIndices: hlIndices, highlightType: hlType } = useEditorStore.getState();

      if (e.key === " " && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        setPlaying(!isPlaying);
      } else if ((e.key === "Delete" || e.key === "Backspace") && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        if (hlIndices.length > 0) {
          // Bulk-delete highlighted words (fillers or pause-adjacent)
          if (hlType === "filler") {
            commands.deleteFillers().then(async (result) => {
              const count = unwrapResult(result);
              if (count > 0) {
                await refreshFromBackend();
              }
              clearHighlights();
            }).catch((err) => {
              console.error("Failed to delete fillers:", err);
              clearHighlights();
            });
          } else {
            (async () => {
              for (const idx of hlIndices) {
                await deleteWord(idx);
              }
              clearHighlights();
            })();
          }
        } else if (selRange) {
          deleteRange(selRange[0], selRange[1]);
        } else if (selIdx !== null) {
          deleteWord(selIdx);
        }
      } else if (e.key === "ArrowLeft" && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        const store = usePlayerStore.getState();
        usePlayerStore.getState().seekTo(Math.max(0, store.currentTime - 5));
      } else if (e.key === "ArrowRight" && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        const store = usePlayerStore.getState();
        usePlayerStore.getState().seekTo(Math.min(store.duration, store.currentTime + 5));
      } else if (e.key === "d" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        if (selRange) {
          deleteRange(selRange[0], selRange[1]);
        } else if (selIdx !== null) {
          deleteWord(selIdx);
        }
      } else if (e.key === "m" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        if (selIdx !== null) {
          silenceWord(selIdx);
        }
      } else if (e.key === "S" && (e.ctrlKey || e.metaKey) && e.shiftKey) {
        e.preventDefault();
        if (selIdx !== null) {
          const w = useEditorStore.getState().words[selIdx];
          if (w && w.text.length > 1) {
            splitWord(selIdx, Math.floor(w.text.length / 2));
          }
        }
      } else if (e.key === "a" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        const ws = useEditorStore.getState().words;
        if (ws.length > 0) {
          selectWord(0);
          setSelectionRange([0, ws.length - 1]);
        }
      } else if (e.key === "Escape") {
        selectWord(null);
        setSelectionRange(null);
        clearHighlights();
      } else if (e.key === "z" && (e.ctrlKey || e.metaKey) && e.shiftKey) {
        e.preventDefault();
        redo();
      } else if (e.key === "z" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        undo();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [deleteWord, deleteRange, silenceWord, splitWord, undo, redo, selectWord, setSelectionRange, clearHighlights, refreshFromBackend]);

  const handleTranscribe = useCallback(async () => {
    if (!mediaInfo) return;
    setIsTranscribing(true);
    setModelMissing(false);
    try {
      const result = unwrapResult(await commands.transcribeMediaFile(mediaInfo.path));
      await setWords(result);
    } catch (err) {
      const errStr = String(err);
      if (errStr.includes("Model is not loaded")) {
        setModelMissing(true);
      } else {
        console.error("Transcription failed:", err);
        toast.error(t("editor.transcriptionError"));
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
      const info = unwrapResult(await commands.mediaImport(path));
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
            const result = unwrapResult(await commands.transcribeMediaFile(storeInfo.path));
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
    async (format: ExportFormat) => {
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
    },
    [],
  );

  const handleSaveProject = useCallback(async () => {
    try {
      const filePath = await save({
        filters: [{ name: "Toaster Project", extensions: ["toaster"] }],
        defaultPath: `${mediaInfo?.file_name ?? "project"}.toaster`,
      });
      if (!filePath) return;
      unwrapResult(await commands.saveProject(filePath, null));
      setLastSavedPath(filePath);
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
      const mediaPath = unwrapResult(await commands.loadProject(path));

      await refreshFromBackend();

      if (mediaPath) {
        const info = unwrapResult(await commands.mediaGetCurrent());
        if (info) {
          setMediaInfo(info);
          const assetUrl = convertFileSrc(info.path);
          setMedia(assetUrl, info.media_type === "Video" ? "video" : "audio");
        }
      }
    } catch (err) {
      console.error("Load project failed:", err);
    }
  }, [refreshFromBackend, setMedia, setMediaInfo]);

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

    const extension = (mediaInfo.extension || (mediaInfo.media_type === "Video" ? "mp4" : "m4a")).toLowerCase();
    const baseName = mediaInfo.file_name.replace(/\.[^/.]+$/, "");

    try {
      const filePath = await save({
        filters: [
          {
            name: mediaInfo.media_type === "Video" ? t("editor.editedVideo") : t("editor.editedAudio"),
            extensions: [extension],
          },
        ],
        defaultPath: `${baseName}-edited.${extension}`,
      });
      if (!filePath) return;
      setIsExportingMedia(true);
      unwrapResult(await commands.exportEditedMedia(mediaInfo.path, filePath, burnCaptions || null));
    } catch (err) {
      console.error("Edited media export failed:", err);
    } finally {
      setIsExportingMedia(false);
    }
  }, [mediaInfo, t, burnCaptions]);

  const handleCleanup = useCallback(async () => {
    clearHighlights();
    setIsCleaningUp(true);
    try {
      await invoke("cleanup_all", {});
      await refreshFromBackend();
    } catch (err) {
      console.error("Cleanup failed:", err);
    } finally {
      setIsCleaningUp(false);
    }
  }, [clearHighlights, refreshFromBackend]);

  const handleNormalizeToggle = useCallback(() => {
    updateSetting("normalize_audio_on_export", !normalizeAudio);
  }, [updateSetting, normalizeAudio]);

  const handleClose = useCallback(() => {
    clearMedia();
    setWords([]);
    selectWord(null);
    setSelectionRange(null);
    clearHighlights();
    setLastSavedPath(null);
    setIsTranscribing(false);
    setModelMissing(false);
  }, [clearMedia, setWords, selectWord, setSelectionRange, clearHighlights]);

  const handleTimeUpdate = useCallback(
    (time: number) => {
      if (words.length === 0) return;
      // Don't auto-select during a manual word click — let the user's selection stick
      if (manualClickRef.current) return;
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
        // Flag to suppress auto-select for a brief period
        manualClickRef.current = true;
        seekTo(word.start_us / 1_000_000);
        useEditorStore.getState().selectWord(index);
        // Clear the flag after the seek settles
        setTimeout(() => {
          manualClickRef.current = false;
        }, 300);
      }
    },
    [words, seekTo],
  );

  return (
    <div className="max-w-6xl w-full mx-auto space-y-6">
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
                  <FileVideo size={16} className="text-text/60" />
                  <span className="text-sm font-medium">
                    {mediaInfo?.file_name}
                  </span>
                  <span className="text-xs text-mid-gray">
                    {mediaInfo
                      ? `${(mediaInfo.file_size / 1024 / 1024).toFixed(1)} MB`
                      : ""}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={handleExportEditedMedia}
                    disabled={words.length === 0 || isExportingMedia}
                    className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors disabled:opacity-50"
                    title={t("editor.export")}
                  >
                    <FileVideo size={14} />
                    {isExportingMedia ? t("editor.exporting") : t("editor.export")}
                  </button>
                  <div className="w-px h-5 bg-mid-gray/30" />
                  <button
                    onClick={handleSaveProject}
                    className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
                    title={t("editor.saveProject")}
                  >
                    <Save size={14} />
                    {t("editor.save")}
                  </button>
                  <button
                    onClick={handleClose}
                    className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
                    title={t("editor.close")}
                  >
                    <X size={14} />
                    {t("editor.close")}
                  </button>
                </div>
              </div>

              {/* Player */}
              <MediaPlayer
                className="rounded-lg overflow-hidden"
                onTimeUpdate={handleTimeUpdate}
                captionsEnabled={burnCaptions}
              />

              {/* Waveform */}
              <Waveform
                audioUrl={mediaUrl}
                currentTime={currentTime}
                duration={duration}
                onSeek={seekTo}
                words={words}
                selectedWordIndex={selectedIndex}
                className="rounded-lg overflow-hidden"
              />
            </>
          )}
        </div>
      </SettingsGroup>

      {/* Project section — only visible when no media loaded */}
      {!mediaUrl && (
        <SettingsGroup title={t("editor.sections.project")}>
          <div className="px-4 py-3">
            <button
              onClick={handleLoadProject}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
              title={t("editor.loadProject")}
            >
              <FolderOpen size={14} />
              {t("editor.open")}
            </button>
          </div>
        </SettingsGroup>
      )}

      {/* Transcription section — only visible when media is loaded */}
      {mediaUrl && (
        <SettingsGroup title={t("editor.sections.transcription")}>
          {words.length === 0 ? (
            <div className="px-4 py-3 space-y-3">
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
            </div>
          ) : (
            <TranscriptEditor onWordClick={handleWordClick} />
          )}
        </SettingsGroup>
      )}

      {/* Edit section — only visible when words are loaded */}
      {words.length > 0 && (
        <SettingsGroup title={t("editor.sections.edit")}>
          <div className="px-4 py-3">
            <div className="flex items-center gap-2 flex-wrap">
              <button
                onClick={handleCleanup}
                disabled={isCleaningUp}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors disabled:opacity-50"
              >
                <AudioLines size={14} />
                {t("editor.cleanup")}
              </button>
              <button
                onClick={() => setBurnCaptions(!burnCaptions)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs border transition-colors ${
                  burnCaptions
                    ? "bg-accent text-black border-accent"
                    : "bg-background border-mid-gray/20 hover:bg-mid-gray/10"
                }`}
              >
                <Captions size={14} />
                {t("editor.addCaptions")}
              </button>
              <button
                onClick={handleNormalizeToggle}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs border transition-colors ${
                  normalizeAudio
                    ? "bg-accent text-black border-accent"
                    : "bg-background border-mid-gray/20 hover:bg-mid-gray/10"
                }`}
              >
                <Volume2 size={14} />
                {t("editor.normalizeAudio")}
              </button>
            </div>
          </div>
        </SettingsGroup>
      )}

      {/* Export & Tools section */}
      <EditorToolbar
        words={words}
        onExport={handleExport}
        onFFmpegScript={handleFFmpegScript}
      />
    </div>
  );
};

export default EditorView;
