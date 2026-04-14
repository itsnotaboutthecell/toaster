import React, { useCallback, useEffect, useRef, useState } from "react";

interface WaveformProps {
  audioUrl: string | null;
  currentTime: number;
  duration: number;
  onSeek: (time: number) => void;
  className?: string;
}

const BAR_COUNT = 300;
const BAR_GAP = 1;
const PLAYED_COLOR = "#E8A838";
const UNPLAYED_COLOR = "#4A4A4A";

function downsamplePeaks(channelData: Float32Array, barCount: number): number[] {
  const blockSize = Math.floor(channelData.length / barCount);
  const peaks: number[] = [];
  for (let i = 0; i < barCount; i++) {
    let max = 0;
    const start = i * blockSize;
    const end = Math.min(start + blockSize, channelData.length);
    for (let j = start; j < end; j++) {
      const abs = Math.abs(channelData[j]);
      if (abs > max) max = abs;
    }
    peaks.push(max);
  }
  // Normalize to 0-1
  const globalMax = Math.max(...peaks, 0.01);
  return peaks.map((p) => p / globalMax);
}

const Waveform: React.FC<WaveformProps> = ({
  audioUrl,
  currentTime,
  duration,
  onSeek,
  className = "",
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [peaks, setPeaks] = useState<number[]>([]);
  const [canvasWidth, setCanvasWidth] = useState(0);
  const canvasHeight = 64;

  // Decode audio and extract waveform peaks
  useEffect(() => {
    if (!audioUrl) {
      setPeaks([]);
      return;
    }

    let cancelled = false;

    const loadAudio = async () => {
      try {
        const response = await fetch(audioUrl);
        const arrayBuffer = await response.arrayBuffer();
        const audioCtx = new AudioContext();
        const audioBuffer = await audioCtx.decodeAudioData(arrayBuffer);
        await audioCtx.close();

        if (cancelled) return;

        const channelData = audioBuffer.getChannelData(0);
        const extracted = downsamplePeaks(channelData, BAR_COUNT);
        setPeaks(extracted);
      } catch (err) {
        console.error("Failed to decode audio for waveform:", err);
        setPeaks([]);
      }
    };

    loadAudio();
    return () => {
      cancelled = true;
    };
  }, [audioUrl]);

  // Observe container resize to keep canvas responsive
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setCanvasWidth(entry.contentRect.width);
      }
    });
    observer.observe(container);
    setCanvasWidth(container.clientWidth);

    return () => observer.disconnect();
  }, []);

  // Draw waveform
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || peaks.length === 0 || canvasWidth === 0) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = canvasWidth * dpr;
    canvas.height = canvasHeight * dpr;
    ctx.scale(dpr, dpr);

    ctx.clearRect(0, 0, canvasWidth, canvasHeight);

    const barWidth = Math.max(1, (canvasWidth - (peaks.length - 1) * BAR_GAP) / peaks.length);
    const progress = duration > 0 ? currentTime / duration : 0;
    const playedBars = Math.floor(progress * peaks.length);

    const midY = canvasHeight / 2;
    const maxBarHeight = canvasHeight * 0.8;

    for (let i = 0; i < peaks.length; i++) {
      const x = i * (barWidth + BAR_GAP);
      const barH = Math.max(2, peaks[i] * maxBarHeight);
      ctx.fillStyle = i < playedBars ? PLAYED_COLOR : UNPLAYED_COLOR;
      ctx.fillRect(x, midY - barH / 2, barWidth, barH);
    }
  }, [peaks, currentTime, duration, canvasWidth]);

  const handleClick = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas || duration <= 0) return;
      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const ratio = Math.max(0, Math.min(1, x / rect.width));
      onSeek(ratio * duration);
    },
    [duration, onSeek],
  );

  if (!audioUrl) return null;

  return (
    <div ref={containerRef} className={`w-full ${className}`}>
      <canvas
        ref={canvasRef}
        onClick={handleClick}
        className="w-full cursor-pointer rounded"
        style={{ height: canvasHeight }}
      />
    </div>
  );
};

export default Waveform;
