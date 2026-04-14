import React, { useCallback, useEffect, useRef, useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Play, Pause, Volume2, VolumeX, Eye, EyeOff, Loader2 } from "lucide-react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { commands } from "@/bindings";
import { usePlayerStore } from "@/stores/playerStore";
import { useEditorStore, type Word } from "@/stores/editorStore";
import {
  DUAL_TRACK_DRIFT_THRESHOLD,
  DUAL_TRACK_SYNC_COOLDOWN_MS,
  getDeletedRanges,
  getDeletedRangesFromKeepSegments,
  editTimeToSourceTime,
  type TimeSegment,
} from "@/lib/utils/timeline";

interface MediaPlayerProps {
  className?: string;
  onTimeUpdate?: (time: number) => void;
}

interface CachedPreviewMetadata {
  generationToken: string;
  sourceMediaFingerprint: string | null;
  editVersion: string;
}

type PreviewCacheMode = "building" | "ready" | "fallback";

function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
}

const PLAYBACK_RATES = [0.5, 0.75, 1, 1.25, 1.5, 2];

const MediaPlayer: React.FC<MediaPlayerProps> = ({
  className = "",
  onTimeUpdate,
}) => {
  const { t } = useTranslation();
  const mediaRef = useRef<HTMLVideoElement & HTMLAudioElement>(null);
  const previewAudioRef = useRef<HTMLAudioElement>(null);
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
  const [backendDeletedRanges, setBackendDeletedRanges] = useState<TimeSegment[] | null>(null);
  const [backendKeepSegments, setBackendKeepSegments] = useState<TimeSegment[]>([]);
  const backendFetchSeq = useRef(0);
  const lastSkipTargetRef = useRef(0);
  const lastObservedTimeRef = useRef(0);
  /** Real-clock timestamp (ms) of the last drift correction applied to the video element */
  const lastVideoSyncTimeRef = useRef(0);

  // Preview cache state: tracks the result of renderTempPreviewAudio
  const [previewCacheState, setPreviewCacheState] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [previewAudioUrl, setPreviewAudioUrl] = useState<string | null>(null);
  const previewRenderTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const previewInvalidationTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const previewRenderSeq = useRef(0);
  const previewMetadataRef = useRef<CachedPreviewMetadata | null>(null);

  /** Use cached preview when enabled and available; keep using stale cache while refresh is building */
  const hasPreviewAudio = !!previewAudioUrl;
  const usePreviewCache = previewEdits && hasPreviewAudio && previewCacheState !== "error";
  const previewCacheMode: PreviewCacheMode = !previewEdits || previewCacheState === "error"
    ? "fallback"
    : previewCacheState === "loading"
      ? "building"
      : usePreviewCache
        ? "ready"
        : "fallback";
  const previewToggleLabel = previewEdits ? t("player.previewEditsOn") : t("player.previewEditsOff");
  const previewCacheModeLabel = previewCacheMode === "building"
    ? t("player.cacheModeBuilding")
    : previewCacheMode === "ready"
      ? t("player.cacheModeReady")
      : t("player.cacheModeFallback");

  const isDualTrackVideoPreview = mediaType === "video" && usePreviewCache;
  const primarySrc = mediaType === "video" ? mediaUrl : usePreviewCache ? previewAudioUrl : mediaUrl;
  const activePlaybackSrc = isDualTrackVideoPreview ? previewAudioUrl : primarySrc;

  // Memoize deleted ranges so they aren't rebuilt every frame
  const deletedRanges = useMemo(() => getDeletedRanges(words, duration), [words, duration]);
  const activeDeletedRanges = backendDeletedRanges ?? deletedRanges;

  const schedulePreviewInvalidation = useCallback(
    (stalePreview: CachedPreviewMetadata | null, reason: string) => {
      if (!stalePreview?.generationToken) {
        return;
      }
      if (previewInvalidationTimerRef.current) {
        clearTimeout(previewInvalidationTimerRef.current);
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

      schedulePreviewInvalidation(stalePreview, reason);
    },
    [schedulePreviewInvalidation],
  );

  useEffect(() => {
    let isCancelled = false;
    const seq = ++backendFetchSeq.current;

    if (words.length === 0) {
      setBackendDeletedRanges([]);
      setBackendKeepSegments([]);
      return;
    }

    const refreshKeepSegments = async () => {
      try {
        const result = await commands.getKeepSegments();
        if (isCancelled || seq !== backendFetchSeq.current) return;
        if (result.status === "ok") {
          setBackendDeletedRanges(getDeletedRangesFromKeepSegments(words, result.data));
          // Normalize keep segments to seconds for timeline mapping
          const normalized = result.data
            .map((s) => ({ start: s.start_us / 1_000_000, end: s.end_us / 1_000_000 }))
            .filter((s) => s.end > s.start)
            .sort((a, b) => a.start - b.start);
          setBackendKeepSegments(normalized);
          return;
        }
      } catch {
        // Fallback to local deleted-ranges heuristic below
      }

      if (!isCancelled && seq === backendFetchSeq.current) {
        setBackendDeletedRanges(null);
        setBackendKeepSegments([]);
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
            const stalePreview = previewMetadataRef.current;
            previewMetadataRef.current = {
              generationToken: meta.generation_token,
              sourceMediaFingerprint: meta.source_media_fingerprint,
              editVersion: meta.edit_version,
            };
            setPreviewAudioUrl(convertFileSrc(meta.preview_url_safe_path));
            setPreviewCacheState("ready");
            if (stalePreview?.generationToken && stalePreview.generationToken !== meta.generation_token) {
              schedulePreviewInvalidation(stalePreview, "preview-replaced");
            }
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
  }, [previewEdits, resetPreviewCache, schedulePreviewInvalidation, words]);

  // Sync seek requests from the store to the media element(s)
  const lastSeekVersion = useRef(0);
  useEffect(() => {
    const mediaEl = mediaRef.current;
    if (!mediaEl || seekVersion === lastSeekVersion.current) return;
    lastSeekVersion.current = seekVersion;
    if (isDualTrackVideoPreview) {
      // Preview audio is the time authority: seek it to the edit-time target directly.
      // Video seeks to the corresponding source time so the correct frame is displayed.
      const previewEl = previewAudioRef.current;
      if (previewEl) previewEl.currentTime = seekTarget;
      const sourceTime =
        backendKeepSegments.length > 0
          ? editTimeToSourceTime(seekTarget, backendKeepSegments)
          : seekTarget;
      mediaEl.currentTime = sourceTime;
      lastVideoSyncTimeRef.current = performance.now();
    } else {
      mediaEl.currentTime = seekTarget;
    }
  }, [seekVersion, seekTarget, isDualTrackVideoPreview, backendKeepSegments]);

  // When playback mode/source switches, reset playback position to 0
  const prevPlaybackKeyRef = useRef<string | null>(null);
  useEffect(() => {
    const playbackKey = isDualTrackVideoPreview
      ? `dual:${previewAudioUrl ?? "none"}`
      : `single:${primarySrc ?? "none"}`;
    if (playbackKey === prevPlaybackKeyRef.current) return;
    const wasSet = prevPlaybackKeyRef.current !== null;
    prevPlaybackKeyRef.current = playbackKey;
    if (!wasSet) return; // initial mount — do nothing
    const mediaEl = mediaRef.current;
    if (mediaEl) mediaEl.currentTime = 0;
    const previewEl = previewAudioRef.current;
    if (previewEl) previewEl.currentTime = 0;
    setCurrentTime(0);
  }, [isDualTrackVideoPreview, previewAudioUrl, primarySrc, setCurrentTime]);

  // Sync volume and playback rate to the element
  useEffect(() => {
    const mediaEl = mediaRef.current;
    if (mediaEl) {
      mediaEl.volume = isDualTrackVideoPreview ? 0 : volume;
      mediaEl.muted = isDualTrackVideoPreview;
    }
    const previewEl = previewAudioRef.current;
    if (previewEl) {
      previewEl.volume = volume;
    }
  }, [volume, isDualTrackVideoPreview]);

  useEffect(() => {
    const mediaEl = mediaRef.current;
    if (mediaEl) mediaEl.playbackRate = playbackRate;
    const previewEl = previewAudioRef.current;
    if (previewEl) previewEl.playbackRate = playbackRate;
  }, [playbackRate]);

  // Play/pause sync
  useEffect(() => {
    const mediaEl = mediaRef.current;
    if (!mediaEl || !activePlaybackSrc) return;
    const previewEl = previewAudioRef.current;

    if (isPlaying) {
      if (isDualTrackVideoPreview && previewEl) {
        void Promise.all([
          mediaEl.play(),
          previewEl.play(),
        ]).catch(() => setPlaying(false));
      } else {
        mediaEl.play().catch(() => setPlaying(false));
      }
    } else {
      mediaEl.pause();
      previewEl?.pause();
    }
  }, [isPlaying, activePlaybackSrc, isDualTrackVideoPreview, setPlaying]);

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
      const el = isDualTrackVideoPreview ? previewAudioRef.current : mediaRef.current;
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

      // Dual-track: keep video element in sync with preview audio (the time authority).
      // Preview audio plays the edit timeline; video must show the matching source frame.
      // Only correct when drift exceeds the threshold and the cooldown has elapsed, to
      // avoid jitter from constant micro-corrections.
      if (isDualTrackVideoPreview && backendKeepSegments.length > 0) {
        const videoEl = mediaRef.current;
        if (videoEl) {
          const targetSourceTime = editTimeToSourceTime(time, backendKeepSegments);
          const drift = Math.abs(videoEl.currentTime - targetSourceTime);
          const now = performance.now();
          if (
            drift > DUAL_TRACK_DRIFT_THRESHOLD &&
            now - lastVideoSyncTimeRef.current > DUAL_TRACK_SYNC_COOLDOWN_MS
          ) {
            videoEl.currentTime = targetSourceTime;
            lastVideoSyncTimeRef.current = now;
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
  }, [isPlaying, previewEdits, usePreviewCache, activeDeletedRanges, duration, setCurrentTime, onTimeUpdate, isDualTrackVideoPreview, backendKeepSegments]);

  // Fallback onTimeUpdate for when paused (seek bar scrubbing, etc.)
  const handleTimeUpdate = useCallback(() => {
    if (isPlaying) return; // RAF loop handles this during playback
    const el = isDualTrackVideoPreview ? previewAudioRef.current : mediaRef.current;
    if (!el) return;
    setCurrentTime(el.currentTime);
    onTimeUpdate?.(el.currentTime);
  }, [isPlaying, setCurrentTime, onTimeUpdate, isDualTrackVideoPreview]);

  const handleLoadedMetadata = useCallback(() => {
    const el = isDualTrackVideoPreview ? previewAudioRef.current : mediaRef.current;
    if (!el) return;
    setDuration(el.duration);
    if (isDualTrackVideoPreview) {
      const mediaEl = mediaRef.current;
      if (mediaEl) {
        mediaEl.volume = 0;
        mediaEl.muted = true;
      }
    } else {
      el.volume = volume;
    }
    el.playbackRate = playbackRate;
  }, [setDuration, volume, playbackRate, isDualTrackVideoPreview]);

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
      const editTime = parseFloat(e.target.value);
      if (isDualTrackVideoPreview) {
        // Seek preview audio (edit-time authority) and video (source time) independently
        const previewEl = previewAudioRef.current;
        if (previewEl) previewEl.currentTime = editTime;
        const mediaEl = mediaRef.current;
        if (mediaEl) {
          const sourceTime =
            backendKeepSegments.length > 0
              ? editTimeToSourceTime(editTime, backendKeepSegments)
              : editTime;
          mediaEl.currentTime = sourceTime;
          lastVideoSyncTimeRef.current = performance.now();
        }
      } else {
        const mediaEl = mediaRef.current;
        if (mediaEl) mediaEl.currentTime = editTime;
      }
      setCurrentTime(editTime);
      onTimeUpdate?.(editTime);
    },
    [setCurrentTime, onTimeUpdate, isDualTrackVideoPreview, backendKeepSegments],
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
  const showVideoDisplay = mediaType === "video";

  return (
    <div className={`flex flex-col bg-neutral-900 rounded-lg ${className}`}>
      {/* Primary media element — video src remains stable even when preview cache is active */}
      <MediaTag
        ref={mediaRef}
        src={primarySrc ?? undefined}
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
      {isDualTrackVideoPreview && (
        <audio
          ref={previewAudioRef}
          src={previewAudioUrl ?? undefined}
          onLoadedMetadata={handleLoadedMetadata}
          onTimeUpdate={handleTimeUpdate}
          onEnded={handlePause}
          className="hidden"
          preload="metadata"
        />
      )}

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
              aria-pressed={previewEdits}
              aria-label={previewToggleLabel}
              className={`flex items-center gap-1 text-xs px-2 py-0.5 rounded transition-colors ${
                previewEdits
                  ? "text-[#E8A838] bg-[#E8A838]/10"
                  : "text-neutral-500 hover:text-neutral-300"
              }`}
              title={previewToggleLabel}
            >
              {previewEdits && previewCacheState === "loading" ? (
                <Loader2 size={14} className="animate-spin" />
              ) : previewEdits ? (
                <Eye size={14} />
              ) : (
                <EyeOff size={14} />
              )}
              {previewToggleLabel}
              {previewEdits && (
                <span className="text-[10px] text-neutral-400 ml-1" title={previewCacheModeLabel}>
                  {previewCacheModeLabel}
                </span>
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
