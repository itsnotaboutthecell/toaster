import React, { useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Play, Pause, Volume2, VolumeX } from "lucide-react";
import { usePlayerStore } from "@/stores/playerStore";

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

const MediaPlayer: React.FC<MediaPlayerProps> = ({
  className = "",
  onTimeUpdate,
}) => {
  const { t } = useTranslation();
  const mediaRef = useRef<HTMLVideoElement & HTMLAudioElement>(null);

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

  const handleTimeUpdate = useCallback(() => {
    const el = mediaRef.current;
    if (!el) return;
    setCurrentTime(el.currentTime);
    onTimeUpdate?.(el.currentTime);
  }, [setCurrentTime, onTimeUpdate]);

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
