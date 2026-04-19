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
} from "lucide-react";
import { SettingsGroup } from "@/components/ui/SettingsGroup";
import { Button } from "@/components/ui/Button";
import { commands } from "@/bindings";
import { useEditorStore } from "@/stores/editorStore";
import { usePlayerStore } from "@/stores/playerStore";
import { useSettingsStore } from "@/stores/settingsStore";
import TranscriptEditor from "@/components/editor/TranscriptEditor";
import MediaPlayer from "@/components/player/MediaPlayer";
import Waveform from "@/components/player/Waveform";
import EditorToolbar from "@/components/editor/EditorToolbar";
import ExportMenu from "@/components/editor/ExportMenu";
import { unwrapResult } from "@/components/editor/EditorView.util";
import { useEditorExports } from "@/components/editor/hooks/useEditorExports";

/**
 * Apply the LLM post-processor to a freshly-transcribed Word[] if the
 * `post_process_enabled` setting is on. Runs the `cleanup_transcription`
 * Tauri command with the joined transcript; on success, positionally
 * remaps the cleaned tokens back onto the original words while preserving
 * ASR-authoritative timestamps. If the token count diverges (which
 * shouldn't happen given the cleanup contract's no_reorder / no_paraphrase
 * invariants but is defensively guarded), the original words are returned
 * unchanged. Any backend error is logged and swallowed — cleanup never
 * blocks a successful transcription.
 */
async function maybeRunPostProcess(
  words: Array<{ text: string; start_us: number; end_us: number; deleted: boolean; silenced: boolean; confidence: number; speaker_id: number }>,
  enabled: boolean,
): Promise<typeof words> {
  if (!enabled || words.length === 0) return words;
  const transcription = words.map((w) => w.text).join(" ");
  try {
    const cleaned = (await invoke("cleanup_transcription", { transcription })) as string | null;
    if (!cleaned) return words;
    const cleanedTokens = cleaned.trim().split(/\s+/).filter(Boolean);
    if (cleanedTokens.length !== words.length) {
      console.warn(
        `[post-process] cleaned-text token count (${cleanedTokens.length}) does not match word count (${words.length}); leaving transcript unchanged.`,
      );
      return words;
    }
    return words.map((w, i) => ({ ...w, text: cleanedTokens[i] }));
  } catch (err) {
    console.warn("[post-process] cleanup_transcription failed:", err);
    return words;
  }
}

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
  const [burnCaptions, setBurnCaptions] = useState(false);
  const [isCleaningUp, setIsCleaningUp] = useState(false);
  const [modelMissing, setModelMissing] = useState(false);
  const [lastSavedPath, setLastSavedPath] = useState<string | null>(null);
  const {
    handleExport,
    handleFFmpegScript,
    handleExportEditedMedia,
    isExportingMedia,
    formatOverride,
    setFormatOverride,
    allowedFormats,
    defaultExportFormat,
  } = useEditorExports({ mediaInfo, settings, burnCaptions });
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
      const postProcessed = await maybeRunPostProcess(result, settings?.post_process_enabled ?? false);
      await setWords(postProcessed);
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
  }, [mediaInfo, setWords, settings?.post_process_enabled]);

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
            const postProcessed = await maybeRunPostProcess(result, settings?.post_process_enabled ?? false);
            await setWords(postProcessed);
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
            <div className="space-y-3">
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
              <div className="flex justify-center">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={handleLoadProject}
                  title={t("editor.loadProject")}
                  className="inline-flex items-center gap-1.5"
                >
                  <FolderOpen size={14} />
                  {t("editor.loadProject")}
                </Button>
              </div>
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
                  <ExportMenu
                    mediaType={mediaInfo?.media_type ?? null}
                    disabled={words.length === 0 || isExportingMedia}
                    isExportingMedia={isExportingMedia}
                    onExportEditedMedia={handleExportEditedMedia}
                    onExportTranscript={handleExport}
                    onExportFFmpegScript={handleFFmpegScript}
                  />
                  <div className="w-px h-5 bg-mid-gray/30" />
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={handleSaveProject}
                    title={t("editor.saveProject")}
                    className="inline-flex items-center gap-1.5"
                  >
                    <Save size={14} />
                    {t("editor.save")}
                  </Button>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={handleClose}
                    title={t("editor.close")}
                    className="inline-flex items-center gap-1.5"
                  >
                    <X size={14} />
                    {t("editor.close")}
                  </Button>
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

      {/* Project + Transcription sections collapsed per KISS pass 1 —
          the Open button now lives in the empty-state above, and the
          transcribe CTA / editor render bare without SettingsGroup
          framing. See features/editor-kiss/plan.md K3, K6. */}
      {mediaUrl && words.length === 0 && (
        <div className="flex flex-col items-center gap-2 py-6">
          <Button
            variant="brand"
            size="md"
            onClick={handleTranscribe}
            disabled={isTranscribing}
            className="inline-flex items-center gap-2"
          >
            <FileText size={16} />
            {isTranscribing
              ? t("editor.transcribing")
              : t("editor.transcribe")}
          </Button>
          {modelMissing && (
            <p className="text-xs text-amber-400">
              {t("editor.modelNotLoaded")}
            </p>
          )}
        </div>
      )}

      {mediaUrl && words.length > 0 && (
        <>
          {/* Inline transcript action row — Cleanup is a transcript
              modification, not an export action, so it lives with
              the transcript, not in an export toolbar. */}
          <div className="flex justify-end">
            <Button
              variant="secondary"
              size="sm"
              onClick={handleCleanup}
              disabled={isCleaningUp}
              className="inline-flex items-center gap-1.5"
            >
              <AudioLines size={14} />
              {t("editor.cleanup")}
            </Button>
          </div>
          <TranscriptEditor onWordClick={handleWordClick} />
        </>
      )}

      {/* Export settings — format override, burn captions, normalize,
          loudness/preflight. Export triggers themselves live in the
          header <ExportMenu>. */}
      <EditorToolbar
        words={words}
        formatOverride={formatOverride}
        onFormatOverrideChange={setFormatOverride}
        allowedFormats={allowedFormats}
        defaultExportFormat={defaultExportFormat}
        exportPickerDisabled={isExportingMedia || words.length === 0}
        burnCaptions={burnCaptions}
        onBurnCaptionsChange={setBurnCaptions}
        normalizeAudio={normalizeAudio}
        onNormalizeAudioToggle={handleNormalizeToggle}
      />
    </div>
  );
};

export default EditorView;
