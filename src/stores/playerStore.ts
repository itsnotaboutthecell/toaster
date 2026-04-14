import { create } from "zustand";

export interface MediaInfo {
  path: string;
  file_name: string;
  file_size: number;
  media_type: "Video" | "Audio";
  extension: string;
}

interface PlayerStore {
  mediaUrl: string | null;
  mediaType: "video" | "audio" | null;
  mediaInfo: MediaInfo | null;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  playbackRate: number;

  // Incremented to signal the MediaPlayer to perform a seek
  seekVersion: number;
  seekTarget: number;

  setMedia: (url: string, type: "video" | "audio") => void;
  setMediaInfo: (info: MediaInfo | null) => void;
  clearMedia: () => void;
  setPlaying: (playing: boolean) => void;
  setCurrentTime: (time: number) => void;
  setDuration: (duration: number) => void;
  setVolume: (volume: number) => void;
  setPlaybackRate: (rate: number) => void;
  seekTo: (time: number) => void;
}

export const usePlayerStore = create<PlayerStore>()((set) => ({
  mediaUrl: null,
  mediaType: null,
  mediaInfo: null,
  isPlaying: false,
  currentTime: 0,
  duration: 0,
  volume: 1,
  playbackRate: 1,
  seekVersion: 0,
  seekTarget: 0,

  setMedia: (url, type) =>
    set({
      mediaUrl: url,
      mediaType: type,
      isPlaying: false,
      currentTime: 0,
      duration: 0,
    }),

  setMediaInfo: (info) => set({ mediaInfo: info }),

  clearMedia: () =>
    set({
      mediaUrl: null,
      mediaType: null,
      mediaInfo: null,
      isPlaying: false,
      currentTime: 0,
      duration: 0,
    }),

  setPlaying: (playing) => set({ isPlaying: playing }),
  setCurrentTime: (time) => set({ currentTime: time }),
  setDuration: (duration) => set({ duration }),
  setVolume: (volume) => set({ volume: Math.max(0, Math.min(1, volume)) }),
  setPlaybackRate: (rate) => set({ playbackRate: rate }),

  seekTo: (time) =>
    set((state) => ({
      seekTarget: time,
      seekVersion: state.seekVersion + 1,
    })),
}));
