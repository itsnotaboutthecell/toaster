import React, { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { FileVideo, Upload, FileText, Download } from "lucide-react";
import { useEditorStore } from "@/stores/editorStore";
import { usePlayerStore } from "@/stores/playerStore";
import TranscriptEditor from "@/components/editor/TranscriptEditor";
import MediaPlayer from "@/components/player/MediaPlayer";
import Waveform from "@/components/player/Waveform";

interface MediaInfo {
  path: string;
  file_name: string;
  file_size: number;
  media_type: "Video" | "Audio";
  extension: string;
}

const EditorView: React.FC = () => {
  const { t } = useTranslation();
  const { words, setWords } = useEditorStore();
  const { mediaUrl, mediaType, currentTime, duration, setMedia } =
    usePlayerStore();
  const seekTo = usePlayerStore((s) => s.seekTo);
  const [mediaInfo, setMediaInfo] = useState<MediaInfo | null>(null);
  const [isTranscribing, setIsTranscribing] = useState(false);

  const handleImportMedia = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: t("editor.mediaFiles"),
            extensions: [
              "mp4",
              "mkv",
              "webm",
              "avi",
              "mov",
              "wmv",
              "flv",
              "m4v",
              "mp3",
              "wav",
              "flac",
              "ogg",
              "aac",
              "m4a",
              "wma",
              "opus",
            ],
          },
        ],
      });

      if (!selected) return;

      const path = typeof selected === "string" ? selected : selected;
      const info = await invoke<MediaInfo>("media_import", { path });
      setMediaInfo(info);

      // Get asset URL for playback
      const assetUrl = await invoke<string | null>("media_get_asset_url");
      if (assetUrl) {
        setMedia(assetUrl, info.media_type === "Video" ? "video" : "audio");
      }
    } catch (err) {
      console.error("Failed to import media:", err);
    }
  }, [t, setMedia]);

  const handleTranscribe = useCallback(async () => {
    if (!mediaInfo) return;
    setIsTranscribing(true);
    try {
      // Transcribe the media file using the backend engine
      const words = await invoke<
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
      // The backend already populated the editor store,
      // but we also update the frontend store
      await setWords(words);
    } catch (err) {
      console.error("Transcription failed:", err);
      // Fall back to a placeholder if transcription fails
      const placeholderWords = [
        {
          text: String(err),
          start_us: 0,
          end_us: 1000000,
          deleted: false,
          silenced: false,
          confidence: 1.0,
          speaker_id: -1,
        },
      ];
      await setWords(placeholderWords);
    } finally {
      setIsTranscribing(false);
    }
  }, [mediaInfo, setWords]);

  const handleExport = useCallback(
    async (format: string) => {
      try {
        const content = await invoke<string>("export_transcript", {
          format,
        });
        // Copy to clipboard as a quick export
        await navigator.clipboard.writeText(content);
      } catch (err) {
        console.error("Export failed:", err);
      }
    },
    [],
  );

  const handleTimeUpdate = useCallback(
    (time: number) => {
      // Highlight the word at the current playback time
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
      // Seek player to the word's start time
      const word = words[index];
      if (word) {
        seekTo(word.start_us / 1_000_000);
      }
    },
    [words, seekTo],
  );

  return (
    <div className="max-w-4xl w-full mx-auto space-y-4">
      {/* Media import area */}
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
          <div className="flex items-center justify-between bg-background border border-mid-gray/20 rounded-lg px-4 py-2">
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
            <div className="flex items-center gap-2">
              <button
                onClick={handleImportMedia}
                className="text-xs text-mid-gray hover:text-foreground transition-colors px-2 py-1"
              >
                {t("editor.changeFile")}
              </button>
            </div>
          </div>

          {/* Player */}
          <MediaPlayer
            className="rounded-lg overflow-hidden"
            onTimeUpdate={handleTimeUpdate}
          />

          {/* Waveform */}
          {mediaUrl && (
            <Waveform
              audioUrl={mediaUrl}
              currentTime={currentTime}
              duration={duration}
              onSeek={seekTo}
              className="h-16 rounded-lg overflow-hidden"
            />
          )}

          {/* Transcribe / Export toolbar */}
          <div className="flex items-center gap-2">
            {words.length === 0 && (
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
            )}
            {words.length > 0 && (
              <div className="flex items-center gap-2">
                <button
                  onClick={() => handleExport("Srt")}
                  className="flex items-center gap-1 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
                >
                  <Download size={14} />
                  SRT
                </button>
                <button
                  onClick={() => handleExport("Vtt")}
                  className="flex items-center gap-1 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
                >
                  <Download size={14} />
                  VTT
                </button>
                <button
                  onClick={() => handleExport("Script")}
                  className="flex items-center gap-1 px-3 py-1.5 bg-background border border-mid-gray/20 rounded-lg text-xs hover:bg-mid-gray/10 transition-colors"
                >
                  <Download size={14} />
                  {t("editor.script")}
                </button>
              </div>
            )}
          </div>
        </>
      )}

      {/* Transcript editor */}
      {words.length > 0 && <TranscriptEditor />}
    </div>
  );
};

export default EditorView;
