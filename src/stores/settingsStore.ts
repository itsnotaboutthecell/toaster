import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import { listen } from "@tauri-apps/api/event";
import type {
  AppSettings as Settings,
  AudioDevice,
  CaptionFontFamily,
  CaptionProfileSet,
  LogLevel,
  LoudnessTarget,
  WhisperAcceleratorSetting,
  OrtAcceleratorSetting,
} from "@/bindings";
import { commands } from "@/bindings";

/**
 * App-wide settings store. Mirrors the backend `AppSettings` struct (see
 * `src-tauri/src/settings/types.rs`) via specta-generated bindings.
 *
 * Persistence contract:
 *   - `settings` is null until `loadSettings()` hydrates from the backend.
 *   - `updateSetting(key, value)` is the only write path. It invokes the
 *     backend's setter command, which validates + persists to disk, then
 *     echoes the updated struct back via the `settings-changed` Tauri event.
 *     The listener installed by `initSettingsListener()` writes the full
 *     result into this store so optimistic UI updates stay consistent.
 *   - `isUpdating[key]` is set while a setter is in flight; UI components
 *     use it to disable controls and avoid racing writes.
 *
 * Every user-visible settings control goes through `updateSetting`; never
 * call backend setter commands directly from components — the updater-
 * coverage gate (`scripts/gate/check-settings-updater-coverage.ts`) enforces
 * one updater per persisted key.
 */
interface SettingsStore {
  settings: Settings | null;
  defaultSettings: Settings | null;
  isLoading: boolean;
  isUpdating: Record<string, boolean>;
  audioDevices: AudioDevice[];
  outputDevices: AudioDevice[];

  // Actions
  initialize: () => Promise<void>;
  loadDefaultSettings: () => Promise<void>;
  updateSetting: <K extends keyof Settings>(
    key: K,
    value: Settings[K],
  ) => Promise<void>;
  resetSetting: (key: keyof Settings) => Promise<void>;
  refreshSettings: () => Promise<void>;
  refreshAudioDevices: () => Promise<void>;
  refreshOutputDevices: () => Promise<void>;
  getSetting: <K extends keyof Settings>(key: K) => Settings[K] | undefined;
  isUpdatingKey: (key: string) => boolean;

  // Internal state setters
  setSettings: (settings: Settings | null) => void;
  setDefaultSettings: (defaultSettings: Settings | null) => void;
  setLoading: (loading: boolean) => void;
  setUpdating: (key: string, updating: boolean) => void;
  setAudioDevices: (devices: AudioDevice[]) => void;
  setOutputDevices: (devices: AudioDevice[]) => void;
}

// Note: Default settings are now fetched from Rust via commands.getDefaultSettings()
// This ensures platform-specific defaults work correctly

const DEFAULT_AUDIO_DEVICE: AudioDevice = {
  index: "default",
  name: "Default",
  is_default: true,
};

const settingUpdaters: {
  [K in keyof Settings]?: (value: Settings[K]) => Promise<unknown>;
} = {
  update_checks_enabled: (value) =>
    commands.changeUpdateChecksSetting(value as boolean),
  selected_output_device: (value) =>
    commands.setSelectedOutputDevice(
      (value as string) === "Default" || value === null
        ? "default"
        : (value as string),
    ),
  translate_to_english: (value) =>
    commands.changeTranslateToEnglishSetting(value as boolean),
  selected_language: (value) =>
    commands.changeSelectedLanguageSetting(value as string),
  debug_mode: (value) => commands.changeDebugModeSetting(value as boolean),
  custom_words: (value) => commands.updateCustomWords(value as string[]),
  custom_filler_words: (value) =>
    commands.changeCustomFillerWordsSetting(value as string[]),
  word_correction_threshold: (value) =>
    commands.changeWordCorrectionThresholdSetting(value as number),
  log_level: (value) => commands.setLogLevel(value as LogLevel),
  app_language: (value) => commands.changeAppLanguageSetting(value as string),
  lazy_stream_close: (value) =>
    commands.changeLazyStreamCloseSetting(value as boolean),
  vad_prefilter_enabled: (value) =>
    commands.changeVadPrefilterEnabledSetting(value as boolean),
  vad_refine_boundaries: (value) =>
    commands.changeVadRefineBoundariesSetting(value as boolean),
  whisper_accelerator: (value) =>
    commands.changeWhisperAcceleratorSetting(
      value as WhisperAcceleratorSetting,
    ),
  ort_accelerator: (value) =>
    commands.changeOrtAcceleratorSetting(value as OrtAcceleratorSetting),
  whisper_gpu_device: (value) =>
    commands.changeWhisperGpuDevice(value as number),
  normalize_audio_on_export: (value) =>
    commands.changeNormalizeAudioSetting(value as boolean),
  loudness_target: (value) =>
    commands.changeLoudnessTargetSetting(value as LoudnessTarget),
  export_volume_db: (value) =>
    commands.changeExportVolumeDbSetting(value as number),
  export_fade_in_ms: (value) =>
    commands.changeExportFadeInMsSetting(value as number),
  export_fade_out_ms: (value) =>
    commands.changeExportFadeOutMsSetting(value as number),
  caption_font_size: (value) =>
    commands.changeCaptionFontSizeSetting(value as number),
  caption_bg_color: (value) =>
    commands.changeCaptionBgColorSetting(value as string),
  caption_text_color: (value) =>
    commands.changeCaptionTextColorSetting(value as string),
  caption_position: (value) =>
    commands.changeCaptionPositionSetting(value as number),
  caption_font_family: (value) =>
    commands.changeCaptionFontFamilySetting(value as CaptionFontFamily),
  caption_radius_px: (value) =>
    commands.changeCaptionRadiusPxSetting(value as number),
  caption_padding_x_px: (value) =>
    commands.changeCaptionPaddingXPxSetting(value as number),
  caption_padding_y_px: (value) =>
    commands.changeCaptionPaddingYPxSetting(value as number),
  caption_max_width_percent: (value) =>
    commands.changeCaptionMaxWidthPercentSetting(value as number),
  // QC round-3 bug fix: Advanced > Captions edits write to
  // `caption_profiles`, but this map previously had no entry for it,
  // so `updateSetting("caption_profiles", …)` logged
  // "No handler for setting: caption_profiles" and the backend
  // `settings.caption_profiles` never moved. The video preview kept
  // serving stale layout from `get_caption_blocks`. Fan out to both
  // orientations here; `updateSetting`'s pendingUpdates dedup keeps
  // live-drag sliders from flooding IPC.
  caption_profiles: async (value) => {
    const set = value as CaptionProfileSet;
    await commands.setCaptionProfile("Desktop", set.desktop, "App");
    await commands.setCaptionProfile("Mobile", set.mobile, "App");
  },
};

// Tracks pending values for keys that are currently mid-update (race dedup)
const pendingUpdates = new Map<string, { key: keyof Settings; value: unknown }>();

export const useSettingsStore = create<SettingsStore>()(
  subscribeWithSelector((set, get) => ({
    settings: null,
    defaultSettings: null,
    isLoading: true,
    isUpdating: {},
    audioDevices: [],
    outputDevices: [],

    // Internal setters
    setSettings: (settings) => set({ settings }),
    setDefaultSettings: (defaultSettings) => set({ defaultSettings }),
    setLoading: (isLoading) => set({ isLoading }),
    setUpdating: (key, updating) =>
      set((state) => ({
        isUpdating: { ...state.isUpdating, [key]: updating },
      })),
    setAudioDevices: (audioDevices) => set({ audioDevices }),
    setOutputDevices: (outputDevices) => set({ outputDevices }),

    // Getters
    getSetting: (key) => get().settings?.[key],
    isUpdatingKey: (key) => get().isUpdating[key] || false,

    // Load settings from store
    refreshSettings: async () => {
      try {
        const result = await commands.getAppSettings();
        if (result.status === "ok") {
          const settings = result.data;
          const normalizedSettings: Settings = {
            ...settings,
            selected_output_device:
              settings.selected_output_device ?? "Default",
          };
          set({ settings: normalizedSettings, isLoading: false });
        } else {
          console.error("Failed to load settings:", result.error);
          set({ isLoading: false });
        }
      } catch (error) {
        console.error("Failed to load settings:", error);
        set({ isLoading: false });
      }
    },

    // Load audio devices
    refreshAudioDevices: async () => {
      try {
        const result = await commands.getAvailableMicrophones();
        if (result.status === "ok") {
          const devicesWithDefault = [
            DEFAULT_AUDIO_DEVICE,
            ...result.data.filter(
              (d) => d.name !== "Default" && d.name !== "default",
            ),
          ];
          set({ audioDevices: devicesWithDefault });
        } else {
          set({ audioDevices: [DEFAULT_AUDIO_DEVICE] });
        }
      } catch (error) {
        console.error("Failed to load audio devices:", error);
        set({ audioDevices: [DEFAULT_AUDIO_DEVICE] });
      }
    },

    // Load output devices
    refreshOutputDevices: async () => {
      try {
        const result = await commands.getAvailableOutputDevices();
        if (result.status === "ok") {
          const devicesWithDefault = [
            DEFAULT_AUDIO_DEVICE,
            ...result.data.filter(
              (d) => d.name !== "Default" && d.name !== "default",
            ),
          ];
          set({ outputDevices: devicesWithDefault });
        } else {
          set({ outputDevices: [DEFAULT_AUDIO_DEVICE] });
        }
      } catch (error) {
        console.error("Failed to load output devices:", error);
        set({ outputDevices: [DEFAULT_AUDIO_DEVICE] });
      }
    },

    // Update a specific setting (with race-condition dedup per key)
    updateSetting: async <K extends keyof Settings>(
      key: K,
      value: Settings[K],
    ) => {
      const updateKey = String(key);

      // If this key is already mid-update, queue the latest value (last-write-wins)
      if (get().isUpdating[updateKey]) {
        pendingUpdates.set(updateKey, { key, value });
        // Optimistically apply the new value to the UI immediately
        set((state) => ({
          settings: state.settings
            ? { ...state.settings, [key]: value }
            : null,
        }));
        return;
      }

      const { settings, setUpdating } = get();
      const originalValue = settings?.[key];

      setUpdating(updateKey, true);

      try {
        set((state) => ({
          settings: state.settings ? { ...state.settings, [key]: value } : null,
        }));

        const updater = settingUpdaters[key];
        if (updater) {
          await updater(value);
        } else if (key !== "bindings" && key !== "selected_model") {
          console.warn(`No handler for setting: ${String(key)}`);
        }
      } catch (error) {
        console.error(`Failed to update setting ${String(key)}:`, error);
        if (settings) {
          set({ settings: { ...settings, [key]: originalValue } });
        }
      } finally {
        setUpdating(updateKey, false);

        // Drain any pending update that arrived while we were busy
        const pending = pendingUpdates.get(updateKey);
        if (pending) {
          pendingUpdates.delete(updateKey);
          await get().updateSetting(
            pending.key as K,
            pending.value as Settings[K],
          );
        }
      }
    },

    // Reset a setting to its default value
    resetSetting: async (key) => {
      const { defaultSettings } = get();
      if (defaultSettings) {
        const defaultValue = defaultSettings[key];
        if (defaultValue !== undefined) {
          await get().updateSetting(key, defaultValue as Settings[typeof key]);
        }
      }
    },

    // Load default settings from Rust
    loadDefaultSettings: async () => {
      try {
        const result = await commands.getDefaultSettings();
        if (result.status === "ok") {
          set({ defaultSettings: result.data });
        } else {
          console.error("Failed to load default settings:", result.error);
        }
      } catch (error) {
        console.error("Failed to load default settings:", error);
      }
    },

    // Initialize everything
    initialize: async () => {
      const { refreshSettings, loadDefaultSettings } = get();

      // Note: Audio devices are NOT refreshed here. The frontend (App.tsx)
      // is responsible for calling refreshAudioDevices/refreshOutputDevices
      // after onboarding completes. This avoids triggering permission dialogs
      // on macOS before the user is ready.
      await Promise.all([loadDefaultSettings(), refreshSettings()]);

      // Re-fetch settings when the backend changes them (e.g. language
      // reset during model switch). The backend is the source of truth.
      listen("model-state-changed", () => {
        get().refreshSettings();
      });
    },
  })),
);
