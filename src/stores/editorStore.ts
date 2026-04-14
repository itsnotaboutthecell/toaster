import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface Word {
  text: string;
  start_us: number;
  end_us: number;
  deleted: boolean;
  silenced: boolean;
  confidence: number;
  speaker_id: number;
}

interface EditorState {
  words: Word[];
  selectedIndex: number | null;
  selectionRange: [number, number] | null;

  setWords: (words: Word[]) => Promise<void>;
  deleteWord: (index: number) => Promise<void>;
  restoreWord: (index: number) => Promise<void>;
  deleteRange: (start: number, end: number) => Promise<void>;
  restoreAll: () => Promise<void>;
  splitWord: (index: number, position: number) => Promise<void>;
  silenceWord: (index: number) => Promise<void>;
  undo: () => Promise<void>;
  redo: () => Promise<void>;
  getKeepSegments: () => Promise<[number, number][]>;
  selectWord: (index: number | null) => void;
  setSelectionRange: (range: [number, number] | null) => void;
}

export const useEditorStore = create<EditorState>()((set) => ({
  words: [],
  selectedIndex: null,
  selectionRange: null,

  setWords: async (words: Word[]) => {
    const result = await invoke<Word[]>("editor_set_words", { words });
    set({ words: result, selectedIndex: null, selectionRange: null });
  },

  deleteWord: async (index: number) => {
    await invoke<boolean>("editor_delete_word", { index });
    const words = await invoke<Word[]>("editor_get_words");
    set({ words });
  },

  restoreWord: async (index: number) => {
    await invoke<boolean>("editor_restore_word", { index });
    const words = await invoke<Word[]>("editor_get_words");
    set({ words });
  },

  deleteRange: async (start: number, end: number) => {
    await invoke<boolean>("editor_delete_range", { start, end });
    const words = await invoke<Word[]>("editor_get_words");
    set({ words, selectedIndex: null, selectionRange: null });
  },

  restoreAll: async () => {
    await invoke<boolean>("editor_restore_all");
    const words = await invoke<Word[]>("editor_get_words");
    set({ words });
  },

  splitWord: async (index: number, position: number) => {
    await invoke<boolean>("editor_split_word", { index, position });
    const words = await invoke<Word[]>("editor_get_words");
    set({ words, selectedIndex: null });
  },

  silenceWord: async (index: number) => {
    await invoke<boolean>("editor_silence_word", { index });
    const words = await invoke<Word[]>("editor_get_words");
    set({ words });
  },

  undo: async () => {
    await invoke<boolean>("editor_undo");
    const words = await invoke<Word[]>("editor_get_words");
    set({ words });
  },

  redo: async () => {
    await invoke<boolean>("editor_redo");
    const words = await invoke<Word[]>("editor_get_words");
    set({ words });
  },

  getKeepSegments: async () => {
    return await invoke<[number, number][]>("editor_get_keep_segments");
  },

  selectWord: (index: number | null) => {
    set({ selectedIndex: index, selectionRange: null });
  },

  setSelectionRange: (range: [number, number] | null) => {
    set({ selectionRange: range });
  },
}));
