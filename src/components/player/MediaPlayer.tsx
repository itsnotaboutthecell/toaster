import React, { useCallback, useEffect, useRef, useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Play, Pause, Volume2, VolumeX, Eye, EyeOff } from "lucide-react";
import { usePlayerStore } from "@/stores/playerStore";
import { useEditorStore, type Word } from "@/stores/editorStore";

interface MediaPlayerProps {
  className?: string;
  onTimeUpdate?: (time: number) => void;
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

  // Memoize deleted ranges so they aren't rebuilt every frame
  const deletedRanges = useMemo(() => getDeletedRanges(words, duration), [words, duration]);

  // Sync seek requests from the store to the media element
  const lastSeekVersion = useRef(0);
  useEffect(() => {
    const el = mediaRef.current;
    if (!el || seekVersion === lastSeekVersion.current) return;
    lastSeekVersion.current = seekVersion;
    el.currentTime = seekTarget;
  }, [seekVersion, seekTarget]);

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
    if (!el || !mediaUrl) return;
    if (isPlaying) {
      el.play().catch(() => setPlaying(false));
    } else {
      el.pause();
    }
  }, [isPlaying, mediaUrl, setPlaying]);

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
      const END_EPSILON = 0.005; // 5ms
      const mediaDuration =
        Number.isFinite(el.duration) && el.duration > 0 ? el.duration : duration;
      const maxSeekTarget =
        Number.isFinite(mediaDuration) && mediaDuration > 0
          ? Math.max(0, mediaDuration - END_EPSILON)
          : Number.POSITIVE_INFINITY;

      // Skip deleted segments when preview edits is on
      if (previewEdits && deletedRanges.length > 0) {
        for (const range of deletedRanges) {
          if (time >= range.start && time < range.end) {
            const seekTarget = Math.min(range.end, maxSeekTarget);
            if (seekTarget > time + END_EPSILON) {
              el.currentTime = seekTarget;
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
  }, [isPlaying, previewEdits, deletedRanges, setCurrentTime, onTimeUpdate]);

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

  return (
    <div className={`flex flex-col bg-neutral-900 rounded-lg ${className}`}>
      {/* Media element */}
      <MediaTag
        ref={mediaRef}
        src={mediaUrl}
        onTimeUpdate={handleTimeUpdate}
        onLoadedMetadata={handleLoadedMetadata}
        onPlay={handlePlay}
        onPause={handlePause}
        onEnded={handlePause}
        className={
          mediaType === "video"
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
              {previewEdits ? <Eye size={14} /> : <EyeOff size={14} />}
              {t("player.preview")}
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
