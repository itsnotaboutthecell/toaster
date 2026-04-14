import React, { useCallback, useEffect, useRef, useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Play, Pause, Volume2, VolumeX, Eye, EyeOff, Loader2 } from "lucide-react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { commands, type KeepSegment } from "@/bindings";
import { usePlayerStore } from "@/stores/playerStore";
import { useEditorStore, type Word } from "@/stores/editorStore";

interface MediaPlayerProps {
  className?: string;
  onTimeUpdate?: (time: number) => void;
}

interface CachedPreviewMetadata {
  generationToken: string;
  sourceMediaFingerprint: string | null;
  editVersion: string;
}

function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
}

const PLAYBACK_RATES = [0.5, 0.75, 1, 1.25, 1.5, 2];

/** Build sorted list of deleted time ranges from words, with crossfade padding */
function getDeletedRanges(words: Word[], duration: number): Array<{ start: number; end: number }> {
  // Padding in seconds to add before/after deleted segments to prevent clicks/pops
  const CROSSFADE_PAD = 0.01; // 10ms
  const MIN_RANGE_DURATION = 0.001; // 1ms
  const maxDuration = Number.isFinite(duration) && duration > 0 ? duration : Number.POSITIVE_INFINITY;

  const ranges: Array<{ start: number; end: number }> = [];
  let rangeStart: number | null = null;
  let rangeEnd = 0;
  const pushRange = (start: number, end: number) => {
    const clampedStart = Math.min(maxDuration, Math.max(0, start));
    const clampedEnd = Math.min(maxDuration, Math.max(0, end));
    if (clampedEnd - clampedStart >= MIN_RANGE_DURATION) {
      ranges.push({ start: clampedStart, end: clampedEnd });
    }
  };

  for (const w of words) {
    if (w.deleted) {
      const startSec = w.start_us / 1_000_000;
      const endSec = w.end_us / 1_000_000;
      if (rangeStart === null) {
        rangeStart = startSec;
        rangeEnd = endSec;
      } else if (startSec <= rangeEnd + 0.05) {
        rangeEnd = Math.max(rangeEnd, endSec);
      } else {
        pushRange(rangeStart - CROSSFADE_PAD, rangeEnd + CROSSFADE_PAD);
        rangeStart = startSec;
        rangeEnd = endSec;
      }
    } else {
      if (rangeStart !== null) {
        pushRange(rangeStart - CROSSFADE_PAD, rangeEnd + CROSSFADE_PAD);
        rangeStart = null;
      }
    }
  }
  if (rangeStart !== null) {
    pushRange(rangeStart - CROSSFADE_PAD, rangeEnd + CROSSFADE_PAD);
  }
  return ranges;
}

function getDeletedRangesFromKeepSegments(
  words: Word[],
  keepSegments: KeepSegment[],
): Array<{ start: number; end: number }> {
  const MIN_RANGE_DURATION = 0.001; // 1ms
  if (words.length === 0) return [];

  const transcriptStart = words[0].start_us / 1_000_000;
  const transcriptEnd = words[words.length - 1].end_us / 1_000_000;
  if (transcriptEnd - transcriptStart < MIN_RANGE_DURATION) return [];

  const normalized = [...keepSegments]
    .map((seg) => ({
      start: seg.start_us / 1_000_000,
      end: seg.end_us / 1_000_000,
    }))
    .filter((seg) => seg.end - seg.start >= MIN_RANGE_DURATION)
    .sort((a, b) => a.start - b.start);

  const ranges: Array<{ start: number; end: number }> = [];
  let cursor = transcriptStart;

  for (const segment of normalized) {
    const segStart = Math.max(transcriptStart, segment.start);
    const segEnd = Math.min(transcriptEnd, segment.end);
    if (segEnd - segStart < MIN_RANGE_DURATION) continue;
    if (segStart - cursor >= MIN_RANGE_DURATION) {
      ranges.push({ start: cursor, end: segStart });
    }
    cursor = Math.max(cursor, segEnd);
  }

  if (transcriptEnd - cursor >= MIN_RANGE_DURATION) {
    ranges.push({ start: cursor, end: transcriptEnd });
  }

  return ranges;
}

const MediaPlayer: React.FC<MediaPlayerProps> = ({
  className = "",
  onTimeUpdate,
}) => {
  const { t } = useTranslation();
  const mediaRef = useRef<HTMLVideoElement & HTMLAudioElement>(null);
  const [previewEdits, setPreviewEdits] = useState(true);

  const {
    mediaUrl,
    mediaType,
    isPlaying,
    currentTime,
    duration,
    volume,
    playbackRate,
    seekVersion,
    seekTarget,
    setPlaying,
    setCurrentTime,
    setDuration,
    setVolume,
    setPlaybackRate,
  } = usePlayerStore();

  const words = useEditorStore((s) => s.words);
  const [backendDeletedRanges, setBackendDeletedRanges] = useState<
    Array<{ start: number; end: number }> | null
  >(null);
  const backendFetchSeq = useRef(0);
  const lastSkipTargetRef = useRef(0);
  const lastObservedTimeRef = useRef(0);

  // Preview cache state: tracks the result of renderTempPreviewAudio
  const [previewCacheState, setPreviewCacheState] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [previewAudioUrl, setPreviewAudioUrl] = useState<string | null>(null);
  const previewRenderTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const previewInvalidationTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const previewRenderSeq = useRef(0);
  const previewMetadataRef = useRef<CachedPreviewMetadata | null>(null);

  /** True when we have a fresh cached preview and should use it for playback */
  const usePreviewCache = previewEdits && previewCacheState === "ready" && !!previewAudioUrl;

  /** The actual src fed to the media element */
  const activeSrc = usePreviewCache ? previewAudioUrl : mediaUrl;

  // Memoize deleted ranges so they aren't rebuilt every frame
  const deletedRanges = useMemo(() => getDeletedRanges(words, duration), [words, duration]);
  const activeDeletedRanges = backendDeletedRanges ?? deletedRanges;

  const resetPreviewCache = useCallback(
    (reason: string, invalidateBackend: boolean) => {
      if (previewRenderTimerRef.current) {
        clearTimeout(previewRenderTimerRef.current);
        previewRenderTimerRef.current = null;
      }
      if (previewInvalidationTimerRef.current) {
        clearTimeout(previewInvalidationTimerRef.current);
        previewInvalidationTimerRef.current = null;
      }

      previewRenderSeq.current += 1;
      const stalePreview = previewMetadataRef.current;
      previewMetadataRef.current = null;
      setPreviewAudioUrl(null);
      setPreviewCacheState("idle");

      if (!invalidateBackend || !stalePreview?.generationToken) {
        return;
      }

      previewInvalidationTimerRef.current = setTimeout(() => {
        void invoke("invalidate_temp_preview_cache", {
          generationToken: stalePreview.generationToken,
          sourceMediaFingerprint: stalePreview.sourceMediaFingerprint,
          reason,
        }).catch((error) => {
          console.warn("Failed to invalidate preview cache:", error);
        });
      }, 250);
    },
    [],
  );

  useEffect(() => {
    let isCancelled = false;
    const seq = ++backendFetchSeq.current;

    if (words.length === 0) {
      setBackendDeletedRanges([]);
      return;
    }

    const refreshKeepSegments = async () => {
      try {
        const result = await commands.getKeepSegments();
        if (isCancelled || seq !== backendFetchSeq.current) return;
        if (result.status === "ok") {
          setBackendDeletedRanges(getDeletedRangesFromKeepSegments(words, result.data));
          return;
        }
      } catch {
        // Fallback to local deleted-ranges heuristic below
      }

      if (!isCancelled && seq === backendFetchSeq.current) {
        setBackendDeletedRanges(null);
      }
    };

    void refreshKeepSegments();
    return () => {
      isCancelled = true;
    };
  }, [words]);

  useEffect(() => {
    return () => {
      if (previewRenderTimerRef.current) {
        clearTimeout(previewRenderTimerRef.current);
      }
      if (previewInvalidationTimerRef.current) {
        clearTimeout(previewInvalidationTimerRef.current);
      }
    };
  }, []);

  const previousPreviewLifecycleRef = useRef<{ mediaUrl: string | null; words: Word[] } | null>(null);
  useEffect(() => {
    const previous = previousPreviewLifecycleRef.current;
    previousPreviewLifecycleRef.current = { mediaUrl, words };

    if (!previous) {
      return;
    }

    if (previous.mediaUrl !== mediaUrl) {
      resetPreviewCache("media-change", true);
      return;
    }

    if (previous.words !== words) {
      resetPreviewCache("edit-change", true);
    }
  }, [mediaUrl, resetPreviewCache, words]);

  // Debounced preview cache generation — fires whenever words or previewEdits change
  useEffect(() => {
    // Clear any running cache when preview is turned off or there are no words
    if (!previewEdits || words.length === 0) {
      resetPreviewCache(previewEdits ? "empty-transcript" : "preview-disabled", false);
      return;
    }

    setPreviewCacheState("loading");

    if (previewRenderTimerRef.current) {
      clearTimeout(previewRenderTimerRef.current);
    }

    const seq = ++previewRenderSeq.current;

    previewRenderTimerRef.current = setTimeout(() => {
      void (async () => {
        try {
          const result = await commands.renderTempPreviewAudio();
          if (seq !== previewRenderSeq.current) return; // superseded
          if (result.status !== "ok") {
            previewMetadataRef.current = null;
            setPreviewCacheState("error");
            return;
          }
          const meta = result.data;
          if (meta.status === "ready" && meta.preview_url_safe_path) {
            previewMetadataRef.current = {
              generationToken: meta.generation_token,
              sourceMediaFingerprint: meta.source_media_fingerprint,
              editVersion: meta.edit_version,
            };
            setPreviewAudioUrl(convertFileSrc(meta.preview_url_safe_path));
            setPreviewCacheState("ready");
          } else {
            // no_segments or missing_media — graceful fallback to live skip
            previewMetadataRef.current = null;
            setPreviewCacheState("error");
          }
        } catch {
          if (seq === previewRenderSeq.current) {
            previewMetadataRef.current = null;
            setPreviewCacheState("error");
          }
        }
      })();
    }, 500);

    return () => {
      if (previewRenderTimerRef.current) {
        clearTimeout(previewRenderTimerRef.current);
        previewRenderTimerRef.current = null;
      }
    };
  }, [previewEdits, resetPreviewCache, words]);

  // Sync seek requests from the store to the media element
  const lastSeekVersion = useRef(0);
  useEffect(() => {
    const el = mediaRef.current;
    if (!el || seekVersion === lastSeekVersion.current) return;
    lastSeekVersion.current = seekVersion;
    el.currentTime = seekTarget;
  }, [seekVersion, seekTarget]);

  // When the active source switches (preview ↔ original), reset playback position to 0
  const prevActiveSrcRef = useRef<string | null>(null);
  useEffect(() => {
    if (activeSrc === prevActiveSrcRef.current) return;
    const wasSet = prevActiveSrcRef.current !== null;
    prevActiveSrcRef.current = activeSrc ?? null;
    if (!wasSet) return; // initial mount — do nothing
    const el = mediaRef.current;
    if (el) el.currentTime = 0;
    setCurrentTime(0);
  }, [activeSrc, setCurrentTime]);

  // Sync volume and playback rate to the element
  useEffect(() => {
    const el = mediaRef.current;
    if (!el) return;
    el.volume = volume;
  }, [volume]);

  useEffect(() => {
    const el = mediaRef.current;
    if (!el) return;
    el.playbackRate = playbackRate;
  }, [playbackRate]);

  // Play/pause sync
  useEffect(() => {
    const el = mediaRef.current;
    if (!el || !activeSrc) return;
    if (isPlaying) {
      el.play().catch(() => setPlaying(false));
    } else {
      el.pause();
    }
  }, [isPlaying, activeSrc, setPlaying]);

  // RAF-based playback loop: polls ~60fps for precise deleted-segment skipping
  // instead of relying on the ~4Hz onTimeUpdate event
  const rafRef = useRef<number>(0);
  useEffect(() => {
    if (!isPlaying) {
      if (rafRef.current) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = 0;
      }
      return;
    }

    const tick = () => {
      const el = mediaRef.current;
      if (!el) return;
      const time = el.currentTime;
      if (time + 0.05 < lastObservedTimeRef.current) {
        lastSkipTargetRef.current = 0;
      }
      lastObservedTimeRef.current = time;
      const END_EPSILON = 0.005; // 5ms
      const mediaDuration =
        Number.isFinite(el.duration) && el.duration > 0 ? el.duration : duration;
      const maxSeekTarget =
        Number.isFinite(mediaDuration) && mediaDuration > 0
          ? Math.max(0, mediaDuration - END_EPSILON)
          : Number.POSITIVE_INFINITY;

      // Skip deleted segments when preview edits is on but no cached preview is available
      if (previewEdits && !usePreviewCache && activeDeletedRanges.length > 0) {
        for (const range of activeDeletedRanges) {
          if (time >= range.start && time < range.end) {
            const seekTarget = Math.min(range.end, maxSeekTarget);
            const monotonicTarget = Math.max(seekTarget, lastSkipTargetRef.current + END_EPSILON);
            const finalTarget = Math.min(monotonicTarget, maxSeekTarget);
            if (finalTarget > time + END_EPSILON) {
              lastSkipTargetRef.current = finalTarget;
              el.currentTime = finalTarget;
              // Don't update store yet — next frame will read the new position
              rafRef.current = requestAnimationFrame(tick);
              return;
            }
            break;
          }
        }
      }

      setCurrentTime(time);
      onTimeUpdate?.(time);
      rafRef.current = requestAnimationFrame(tick);
    };

    rafRef.current = requestAnimationFrame(tick);
    return () => {
      if (rafRef.current) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = 0;
      }
    };
  }, [isPlaying, previewEdits, usePreviewCache, activeDeletedRanges, duration, setCurrentTime, onTimeUpdate]);

  // Fallback onTimeUpdate for when paused (seek bar scrubbing, etc.)
  const handleTimeUpdate = useCallback(() => {
    if (isPlaying) return; // RAF loop handles this during playback
    const el = mediaRef.current;
    if (!el) return;
    setCurrentTime(el.currentTime);
    onTimeUpdate?.(el.currentTime);
  }, [isPlaying, setCurrentTime, onTimeUpdate]);

  const handleLoadedMetadata = useCallback(() => {
    const el = mediaRef.current;
    if (!el) return;
    setDuration(el.duration);
    el.volume = volume;
    el.playbackRate = playbackRate;
  }, [setDuration, volume, playbackRate]);

  const handlePlay = useCallback(() => setPlaying(true), [setPlaying]);
  const handlePause = useCallback(() => setPlaying(false), [setPlaying]);

  const togglePlay = useCallback(() => {
    setPlaying(!isPlaying);
  }, [isPlaying, setPlaying]);

  const toggleMute = useCallback(() => {
    setVolume(volume === 0 ? 1 : 0);
  }, [volume, setVolume]);

  const handleSeekBarChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const time = parseFloat(e.target.value);
      const el = mediaRef.current;
      if (el) {
        el.currentTime = time;
      }
      setCurrentTime(time);
      onTimeUpdate?.(time);
    },
    [setCurrentTime, onTimeUpdate],
  );

  const handleVolumeChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setVolume(parseFloat(e.target.value));
    },
    [setVolume],
  );

  const handleRateChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      setPlaybackRate(parseFloat(e.target.value));
    },
    [setPlaybackRate],
  );

  if (!mediaUrl || !mediaType) {
    return (
      <div
        className={`flex items-center justify-center bg-neutral-900 text-neutral-500 rounded-lg p-8 ${className}`}
      >
        {t("player.noMedia")}
      </div>
    );
  }

  const MediaTag = mediaType === "video" ? "video" : "audio";
  // When using cached preview audio on a video file, hide the video display
  const showVideoDisplay = mediaType === "video" && !usePreviewCache;

  return (
    <div className={`flex flex-col bg-neutral-900 rounded-lg ${className}`}>
      {/* Media element — src switches between original and cached preview audio */}
      <MediaTag
        ref={mediaRef}
        src={activeSrc ?? undefined}
        onTimeUpdate={handleTimeUpdate}
        onLoadedMetadata={handleLoadedMetadata}
        onPlay={handlePlay}
        onPause={handlePause}
        onEnded={handlePause}
        className={
          showVideoDisplay
            ? "w-full rounded-t-lg bg-black"
            : "hidden"
        }
        preload="metadata"
      />

      {/* Controls */}
      <div className="flex flex-col gap-2 px-3 py-2">
        {/* Seek bar */}
        <input
          type="range"
          min={0}
          max={duration || 0}
          step={0.01}
          value={currentTime}
          onChange={handleSeekBarChange}
          className="w-full h-1 appearance-none bg-neutral-700 rounded cursor-pointer accent-[#E8A838]"
          aria-label="Seek"
        />

        {/* Controls row */}
        <div className="flex items-center gap-3 text-neutral-300">
          {/* Play/Pause */}
          <button
            onClick={togglePlay}
            className="hover:text-[#E8A838] transition-colors"
            aria-label={isPlaying ? t("player.pause") : t("player.play")}
          >
            {isPlaying ? <Pause size={20} /> : <Play size={20} />}
          </button>

          {/* Time display */}
          <span className="text-xs font-mono tabular-nums min-w-[90px]">
            {formatTime(currentTime)} / {formatTime(duration)}
          </span>

          {/* Preview Edits toggle */}
          {words.length > 0 && (
            <button
              onClick={() => setPreviewEdits(!previewEdits)}
              className={`flex items-center gap-1 text-xs px-2 py-0.5 rounded transition-colors ${
                previewEdits
                  ? "text-[#E8A838] bg-[#E8A838]/10"
                  : "text-neutral-500 hover:text-neutral-300"
              }`}
              title={t("player.previewEdits")}
            >
              {previewEdits && previewCacheState === "loading" ? (
                <Loader2 size={14} className="animate-spin" />
              ) : previewEdits ? (
                <Eye size={14} />
              ) : (
                <EyeOff size={14} />
              )}
              {t("player.preview")}
              {previewEdits && usePreviewCache && (
                <span className="w-1.5 h-1.5 rounded-full bg-green-400 ml-0.5" title={t("player.previewCached")} />
              )}
            </button>
          )}

          {/* Spacer */}
          <div className="flex-1" />

          {/* Volume */}
          <button
            onClick={toggleMute}
            className="hover:text-[#E8A838] transition-colors"
            aria-label={volume === 0 ? t("player.volume") : t("player.mute")}
          >
            {volume === 0 ? <VolumeX size={18} /> : <Volume2 size={18} />}
          </button>
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            value={volume}
            onChange={handleVolumeChange}
            className="w-16 h-1 appearance-none bg-neutral-700 rounded cursor-pointer accent-[#E8A838]"
            aria-label={t("player.volume")}
          />

          {/* Playback speed */}
          <select
            value={playbackRate}
            onChange={handleRateChange}
            className="bg-neutral-800 text-neutral-300 text-xs rounded px-1.5 py-0.5 border border-neutral-700 cursor-pointer focus:outline-none focus:border-[#E8A838]"
            aria-label={t("player.speed")}
          >
            {PLAYBACK_RATES.map((rate) => (
              <option key={rate} value={rate}>
                {/* eslint-disable-next-line i18next/no-literal-string */}
                {rate}x
              </option>
            ))}
          </select>
        </div>
      </div>
    </div>
  );
};

export default MediaPlayer;
