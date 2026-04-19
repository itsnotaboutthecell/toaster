import { describe, it, expect, vi, beforeEach } from "vitest";
import type { AppSettings as Settings, AudioDevice } from "@/bindings";

// ── Mock @/bindings ──────────────────────────────────────────────────
// vi.mock factories are hoisted – they cannot reference outer variables.
// We use vi.hoisted() to create mocks that are available at hoist time.
const { mockCommands } = vi.hoisted(() => {
  const fn = () => vi.fn();
  return {
    mockCommands: {
      getAppSettings: fn(),
      getDefaultSettings: fn(),
      getAvailableMicrophones: fn(),
      getAvailableOutputDevices: fn(),
      changeUpdateChecksSetting: fn(),
      setSelectedOutputDevice: fn(),
      changeTranslateToEnglishSetting: fn(),
      changeSelectedLanguageSetting: fn(),
      changeDebugModeSetting: fn(),
      updateCustomWords: fn(),
      changeWordCorrectionThresholdSetting: fn(),
      changePostProcessEnabledSetting: fn(),
      setPostProcessSelectedPrompt: fn(),
      setLogLevel: fn(),
      changeAppLanguageSetting: fn(),
      changeLazyStreamCloseSetting: fn(),
      changeWhisperAcceleratorSetting: fn(),
      changeOrtAcceleratorSetting: fn(),
      changeWhisperGpuDevice: fn(),
      changeNormalizeAudioSetting: fn(),
      changeExportVolumeDbSetting: fn(),
      changeExportFadeInMsSetting: fn(),
      changeExportFadeOutMsSetting: fn(),
      setPostProcessProvider: fn(),
      changePostProcessBaseUrlSetting: fn(),
      changePostProcessApiKeySetting: fn(),
      changePostProcessModelSetting: fn(),
      fetchPostProcessModels: fn(),
    } as Record<string, ReturnType<typeof vi.fn>>,
  };
});

vi.mock("@/bindings", () => ({ commands: mockCommands }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

import { useSettingsStore } from "./settingsStore";

// ── Helpers ──────────────────────────────────────────────────────────
function makeSettings(overrides: Partial<Settings> = {}): Settings {
  return {
    bindings: {},
    debug_mode: false,
    update_checks_enabled: true,
    selected_output_device: "Default",
    ...overrides,
  } as Settings;
}

function ok<T>(data: T) {
  return { status: "ok" as const, data };
}

function err(error: string) {
  return { status: "error" as const, error };
}

// ── Tests ────────────────────────────────────────────────────────────
describe("settingsStore", () => {
  beforeEach(() => {
    // Reset Zustand store to initial state
    useSettingsStore.setState({
      settings: null,
      defaultSettings: null,
      isLoading: true,
      isUpdating: {},
      audioDevices: [],
      outputDevices: [],
      postProcessModelOptions: {},
    });
    vi.clearAllMocks();
  });

  // ── Initial state ────────────────────────────────────────────────
  describe("initial state", () => {
    it("has correct defaults", () => {
      const state = useSettingsStore.getState();
      expect(state.settings).toBeNull();
      expect(state.defaultSettings).toBeNull();
      expect(state.isLoading).toBe(true);
      expect(state.isUpdating).toEqual({});
      expect(state.audioDevices).toEqual([]);
      expect(state.outputDevices).toEqual([]);
      expect(state.postProcessModelOptions).toEqual({});
    });
  });

  // ── refreshSettings ──────────────────────────────────────────────
  describe("refreshSettings", () => {
    it("loads settings from backend and normalizes nulls", async () => {
      const backendSettings = makeSettings({
        selected_output_device: null,
      });
      mockCommands.getAppSettings.mockResolvedValue(ok(backendSettings));

      await useSettingsStore.getState().refreshSettings();
      const state = useSettingsStore.getState();

      expect(state.isLoading).toBe(false);
      expect(state.settings?.selected_output_device).toBe("Default");
    });

    it("sets isLoading false on error", async () => {
      mockCommands.getAppSettings.mockResolvedValue(err("backend error"));

      await useSettingsStore.getState().refreshSettings();

      expect(useSettingsStore.getState().isLoading).toBe(false);
    });

    it("sets isLoading false on rejection", async () => {
      mockCommands.getAppSettings.mockRejectedValue(new Error("crash"));

      await useSettingsStore.getState().refreshSettings();

      expect(useSettingsStore.getState().isLoading).toBe(false);
    });
  });

  // ── loadDefaultSettings ──────────────────────────────────────────
  describe("loadDefaultSettings", () => {
    it("stores default settings from backend", async () => {
      const defaults = makeSettings({ debug_mode: true });
      mockCommands.getDefaultSettings.mockResolvedValue(ok(defaults));

      await useSettingsStore.getState().loadDefaultSettings();

      expect(useSettingsStore.getState().defaultSettings).toEqual(defaults);
    });

    it("does not throw on error response", async () => {
      mockCommands.getDefaultSettings.mockResolvedValue(err("fail"));
      await expect(
        useSettingsStore.getState().loadDefaultSettings(),
      ).resolves.not.toThrow();
    });
  });

  // ── initialize ───────────────────────────────────────────────────
  describe("initialize", () => {
    it("calls loadDefaultSettings and refreshSettings", async () => {
      mockCommands.getAppSettings.mockResolvedValue(ok(makeSettings()));
      mockCommands.getDefaultSettings.mockResolvedValue(ok(makeSettings()));

      await useSettingsStore.getState().initialize();

      expect(mockCommands.getAppSettings).toHaveBeenCalled();
      expect(mockCommands.getDefaultSettings).toHaveBeenCalled();
    });
  });

  // ── getSetting / isUpdatingKey ──────────────────────────────────
  describe("getSetting / isUpdatingKey", () => {
    it("returns undefined when settings are null", () => {
      expect(useSettingsStore.getState().getSetting("debug_mode")).toBeUndefined();
    });

    it("returns the value when settings exist", () => {
      useSettingsStore.setState({ settings: makeSettings({ debug_mode: true }) });
      expect(useSettingsStore.getState().getSetting("debug_mode")).toBe(true);
    });

    it("isUpdatingKey returns false by default", () => {
      expect(useSettingsStore.getState().isUpdatingKey("debug_mode")).toBe(false);
    });

    it("isUpdatingKey returns true after setUpdating", () => {
      useSettingsStore.getState().setUpdating("debug_mode", true);
      expect(useSettingsStore.getState().isUpdatingKey("debug_mode")).toBe(true);
    });
  });

  // ── updateSetting ────────────────────────────────────────────────
  describe("updateSetting", () => {
    beforeEach(() => {
      useSettingsStore.setState({ settings: makeSettings() });
    });

    it("optimistically updates the UI and calls the backend command", async () => {
      mockCommands.changeDebugModeSetting.mockResolvedValue(undefined);

      await useSettingsStore.getState().updateSetting("debug_mode", true);

      expect(useSettingsStore.getState().settings?.debug_mode).toBe(true);
      expect(mockCommands.changeDebugModeSetting).toHaveBeenCalledWith(true);
    });

    it("rolls back on backend failure", async () => {
      useSettingsStore.setState({
        settings: makeSettings({ update_checks_enabled: true }),
      });
      mockCommands.changeUpdateChecksSetting.mockRejectedValue(
        new Error("fail"),
      );

      await useSettingsStore
        .getState()
        .updateSetting("update_checks_enabled", false);

      // Should roll back to original value (true from makeSettings)
      expect(useSettingsStore.getState().settings?.update_checks_enabled).toBe(
        true,
      );
    });

    it("handles settings with no explicit updater gracefully", async () => {
      // "bindings" and "selected_model" are known keys with no updater and no warning
      await useSettingsStore.getState().updateSetting("selected_model", "some-model");

      expect(useSettingsStore.getState().settings?.selected_model).toBe("some-model");
    });

    it("normalizes selected_output_device 'Default' to 'default'", async () => {
      mockCommands.setSelectedOutputDevice.mockResolvedValue(undefined);

      await useSettingsStore
        .getState()
        .updateSetting("selected_output_device", "Default");

      expect(mockCommands.setSelectedOutputDevice).toHaveBeenCalledWith(
        "default",
      );
    });

    it("clears isUpdating after success", async () => {
      mockCommands.changeDebugModeSetting.mockResolvedValue(undefined);

      await useSettingsStore.getState().updateSetting("debug_mode", true);

      expect(useSettingsStore.getState().isUpdatingKey("debug_mode")).toBe(false);
    });

    it("clears isUpdating after failure", async () => {
      mockCommands.changeDebugModeSetting.mockRejectedValue(new Error("fail"));

      await useSettingsStore.getState().updateSetting("debug_mode", true);

      expect(useSettingsStore.getState().isUpdatingKey("debug_mode")).toBe(false);
    });
  });

  // ── resetSetting ─────────────────────────────────────────────────
  describe("resetSetting", () => {
    it("updates setting to its default value", async () => {
      const defaults = makeSettings({ debug_mode: true });
      useSettingsStore.setState({
        settings: makeSettings({ debug_mode: false }),
        defaultSettings: defaults,
      });
      mockCommands.changeDebugModeSetting.mockResolvedValue(undefined);

      await useSettingsStore.getState().resetSetting("debug_mode");

      expect(useSettingsStore.getState().settings?.debug_mode).toBe(true);
    });

    it("does nothing when defaultSettings is null", async () => {
      useSettingsStore.setState({
        settings: makeSettings(),
        defaultSettings: null,
      });

      await useSettingsStore.getState().resetSetting("debug_mode");

      // No command should have been called
      expect(mockCommands.changeDebugModeSetting).not.toHaveBeenCalled();
    });
  });

  // ── refreshAudioDevices ──────────────────────────────────────────
  describe("refreshAudioDevices", () => {
    it("prepends a default device and filters duplicates", async () => {
      const devices: AudioDevice[] = [
        { index: "1", name: "USB Mic", is_default: false },
        { index: "0", name: "Default", is_default: true },
      ];
      mockCommands.getAvailableMicrophones.mockResolvedValue(ok(devices));

      await useSettingsStore.getState().refreshAudioDevices();

      const result = useSettingsStore.getState().audioDevices;
      expect(result).toHaveLength(2);
      expect(result[0]).toEqual({
        index: "default",
        name: "Default",
        is_default: true,
      });
      expect(result[1].name).toBe("USB Mic");
    });

    it("falls back to default-only on error", async () => {
      mockCommands.getAvailableMicrophones.mockResolvedValue(err("fail"));

      await useSettingsStore.getState().refreshAudioDevices();

      expect(useSettingsStore.getState().audioDevices).toEqual([
        { index: "default", name: "Default", is_default: true },
      ]);
    });

    it("falls back to default-only on rejection", async () => {
      mockCommands.getAvailableMicrophones.mockRejectedValue(
        new Error("crash"),
      );

      await useSettingsStore.getState().refreshAudioDevices();

      expect(useSettingsStore.getState().audioDevices).toEqual([
        { index: "default", name: "Default", is_default: true },
      ]);
    });
  });

  // ── refreshOutputDevices ─────────────────────────────────────────
  describe("refreshOutputDevices", () => {
    it("prepends a default device and filters duplicates", async () => {
      const devices: AudioDevice[] = [
        { index: "1", name: "Speakers", is_default: false },
        { index: "0", name: "default", is_default: true },
      ];
      mockCommands.getAvailableOutputDevices.mockResolvedValue(ok(devices));

      await useSettingsStore.getState().refreshOutputDevices();

      const result = useSettingsStore.getState().outputDevices;
      expect(result).toHaveLength(2);
      expect(result[0].name).toBe("Default");
      expect(result[1].name).toBe("Speakers");
    });

    it("falls back to default-only on error", async () => {
      mockCommands.getAvailableOutputDevices.mockResolvedValue(err("fail"));

      await useSettingsStore.getState().refreshOutputDevices();

      expect(useSettingsStore.getState().outputDevices).toEqual([
        { index: "default", name: "Default", is_default: true },
      ]);
    });
  });

  // ── setPostProcessProvider ───────────────────────────────────────
  describe("setPostProcessProvider", () => {
    beforeEach(() => {
      useSettingsStore.setState({
        settings: makeSettings({ post_process_provider_id: "openai" }),
      });
    });

    it("optimistically updates provider and clears model options", async () => {
      mockCommands.setPostProcessProvider.mockResolvedValue(ok(undefined));
      mockCommands.getAppSettings.mockResolvedValue(ok(makeSettings()));

      await useSettingsStore.getState().setPostProcessProvider("anthropic");

      expect(mockCommands.setPostProcessProvider).toHaveBeenCalledWith(
        "anthropic",
      );
    });

    it("rolls back provider on failure", async () => {
      mockCommands.setPostProcessProvider.mockResolvedValue(
        err("provider error"),
      );

      await expect(
        useSettingsStore.getState().setPostProcessProvider("bad"),
      ).rejects.toThrow("provider error");

      expect(
        useSettingsStore.getState().settings?.post_process_provider_id,
      ).toBe("openai");
    });
  });

  // ── updatePostProcessBaseUrl ─────────────────────────────────────
  describe("updatePostProcessBaseUrl", () => {
    it("persists base URL, resets model, clears cached models, refreshes", async () => {
      useSettingsStore.setState({ settings: makeSettings() });
      mockCommands.changePostProcessBaseUrlSetting.mockResolvedValue(
        ok(undefined),
      );
      mockCommands.changePostProcessModelSetting.mockResolvedValue(
        ok(undefined),
      );
      mockCommands.getAppSettings.mockResolvedValue(ok(makeSettings()));

      await useSettingsStore
        .getState()
        .updatePostProcessBaseUrl("custom", "https://example.com");

      expect(
        mockCommands.changePostProcessBaseUrlSetting,
      ).toHaveBeenCalledWith("custom", "https://example.com");
      expect(mockCommands.changePostProcessModelSetting).toHaveBeenCalledWith(
        "custom",
        "",
      );
      expect(
        useSettingsStore.getState().postProcessModelOptions["custom"],
      ).toEqual([]);
    });

    it("throws when base URL persistence fails", async () => {
      useSettingsStore.setState({ settings: makeSettings() });
      mockCommands.changePostProcessBaseUrlSetting.mockResolvedValue(
        err("url fail"),
      );

      await expect(
        useSettingsStore
          .getState()
          .updatePostProcessBaseUrl("custom", "https://bad"),
      ).rejects.toThrow("url fail");
    });
  });

  // ── updatePostProcessApiKey ──────────────────────────────────────
  describe("updatePostProcessApiKey", () => {
    it("clears cached models then delegates to updatePostProcessSetting", async () => {
      useSettingsStore.setState({
        settings: makeSettings(),
        postProcessModelOptions: { openai: ["gpt-4"] },
      });
      mockCommands.changePostProcessApiKeySetting.mockResolvedValue(
        ok(undefined),
      );
      mockCommands.getAppSettings.mockResolvedValue(ok(makeSettings()));

      await useSettingsStore
        .getState()
        .updatePostProcessApiKey("openai", "sk-123");

      expect(
        useSettingsStore.getState().postProcessModelOptions["openai"],
      ).toEqual([]);
      expect(mockCommands.changePostProcessApiKeySetting).toHaveBeenCalledWith(
        "openai",
        "sk-123",
      );
    });
  });

  // ── fetchPostProcessModels ───────────────────────────────────────
  describe("fetchPostProcessModels", () => {
    it("stores returned models and returns them", async () => {
      mockCommands.fetchPostProcessModels.mockResolvedValue(
        ok(["gpt-4", "gpt-3.5"]),
      );

      const models = await useSettingsStore
        .getState()
        .fetchPostProcessModels("openai");

      expect(models).toEqual(["gpt-4", "gpt-3.5"]);
      expect(
        useSettingsStore.getState().postProcessModelOptions["openai"],
      ).toEqual(["gpt-4", "gpt-3.5"]);
    });

    it("throws on backend error", async () => {
      mockCommands.fetchPostProcessModels.mockResolvedValue(
        err("fetch failed"),
      );

      await expect(
        useSettingsStore.getState().fetchPostProcessModels("openai"),
      ).rejects.toThrow("fetch failed");
    });

    it("clears isUpdating after fetch", async () => {
      mockCommands.fetchPostProcessModels.mockResolvedValue(
        ok(["model-a"]),
      );

      await useSettingsStore.getState().fetchPostProcessModels("openai");

      expect(
        useSettingsStore
          .getState()
          .isUpdatingKey("post_process_models_fetch:openai"),
      ).toBe(false);
    });
  });

  // ── Internal setters ─────────────────────────────────────────────
  describe("internal setters", () => {
    it("setSettings updates settings", () => {
      const s = makeSettings();
      useSettingsStore.getState().setSettings(s);
      expect(useSettingsStore.getState().settings).toBe(s);
    });

    it("setLoading updates isLoading", () => {
      useSettingsStore.getState().setLoading(false);
      expect(useSettingsStore.getState().isLoading).toBe(false);
    });

    it("setAudioDevices updates audioDevices", () => {
      const devices: AudioDevice[] = [
        { index: "1", name: "Mic", is_default: false },
      ];
      useSettingsStore.getState().setAudioDevices(devices);
      expect(useSettingsStore.getState().audioDevices).toEqual(devices);
    });

    it("setPostProcessModelOptions updates for a specific provider", () => {
      useSettingsStore
        .getState()
        .setPostProcessModelOptions("openai", ["gpt-4"]);
      expect(
        useSettingsStore.getState().postProcessModelOptions["openai"],
      ).toEqual(["gpt-4"]);
    });
  });
});
