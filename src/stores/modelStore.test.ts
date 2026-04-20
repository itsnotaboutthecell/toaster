import { describe, it, expect, vi, beforeEach } from "vitest";
import { type ModelInfo } from "@/bindings";

// Capture event listeners registered via listen()
const eventListeners: Record<string, (event: { payload: unknown }) => void> =
  {};

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((eventName: string, handler: (event: { payload: unknown }) => void) => {
    eventListeners[eventName] = handler;
    return Promise.resolve(() => {});
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  Channel: class {
    onmessage: (() => void) | null = null;
  },
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

// Mock the commands object used by the store
const mockCommands = {
  getAvailableModels: vi.fn(),
  getCurrentModel: vi.fn(),
  hasAnyModelsAvailable: vi.fn(),
  setActiveModel: vi.fn(),
  downloadModel: vi.fn(),
  cancelDownload: vi.fn(),
  deleteModel: vi.fn(),
};

vi.mock("@/bindings", async () => {
  const actual = await vi.importActual<typeof import("@/bindings")>("@/bindings");
  return {
    ...actual,
    commands: mockCommands,
  };
});

function makeModel(overrides: Partial<ModelInfo> = {}): ModelInfo {
  return {
    id: "test-model",
    name: "Test Model",
    description: "A test model",
    filename: "test.bin",
    url: null,
    sha256: null,
    size_mb: 100,
    is_downloaded: true,
    is_downloading: false,
    partial_size: 0,
    is_directory: false,
    engine_type: "Whisper",
    accuracy_score: 5,
    speed_score: 5,
    supports_translation: false,
    is_recommended: false,
    supported_languages: ["en"],
    supports_language_selection: false,
    is_custom: false,
    ...overrides,
  };
}

// Must import after mocks are set up
const { useModelStore } = await import("./modelStore");

function getStore() {
  return useModelStore.getState();
}

function resetStore() {
  useModelStore.setState({
    models: [],
    currentModel: "",
    downloadingModels: {},
    verifyingModels: {},
    extractingModels: {},
    downloadProgress: {},
    downloadStats: {},
    loading: true,
    error: null,
    hasAnyModels: false,
    isFirstRun: false,
    initialized: false,
  });
}

beforeEach(() => {
  resetStore();
  vi.clearAllMocks();
  // Clear captured event listeners
  Object.keys(eventListeners).forEach((k) => delete eventListeners[k]);
});

describe("modelStore", () => {
  describe("initial state", () => {
    it("has empty models array", () => {
      expect(getStore().models).toEqual([]);
    });

    it("has no selected model", () => {
      expect(getStore().currentModel).toBe("");
    });

    it("is not downloading anything", () => {
      expect(getStore().downloadingModels).toEqual({});
      expect(getStore().downloadProgress).toEqual({});
    });

    it("starts in loading state", () => {
      expect(getStore().loading).toBe(true);
    });

    it("has no error", () => {
      expect(getStore().error).toBeNull();
    });

    it("is not initialized", () => {
      expect(getStore().initialized).toBe(false);
    });
  });

  describe("setters", () => {
    it("setModels updates models", () => {
      const models = [makeModel({ id: "m1" }), makeModel({ id: "m2" })];
      getStore().setModels(models);
      expect(getStore().models).toEqual(models);
    });

    it("setCurrentModel updates currentModel", () => {
      getStore().setCurrentModel("my-model");
      expect(getStore().currentModel).toBe("my-model");
    });

    it("setError updates error", () => {
      getStore().setError("something broke");
      expect(getStore().error).toBe("something broke");
    });

    it("setLoading updates loading", () => {
      getStore().setLoading(false);
      expect(getStore().loading).toBe(false);
    });
  });

  describe("loadModels", () => {
    it("loads all catalog models into the store", async () => {
      const asrModel = makeModel({ id: "user-1", category: "Transcription" });
      const vadModel = makeModel({
        id: "silero-vad",
        category: "VoiceActivityDetection",
      });
      mockCommands.getAvailableModels.mockResolvedValue({
        status: "ok",
        data: [asrModel, vadModel],
      });

      await getStore().loadModels();

      expect(mockCommands.getAvailableModels).toHaveBeenCalled();
      expect(getStore().models).toEqual([asrModel, vadModel]);
      expect(getStore().error).toBeNull();
      expect(getStore().loading).toBe(false);
    });

    it("sets error on command error result", async () => {
      mockCommands.getAvailableModels.mockResolvedValue({
        status: "error",
        error: "backend broke",
      });

      await getStore().loadModels();

      expect(getStore().error).toBe("Failed to load models: backend broke");
      expect(getStore().loading).toBe(false);
    });

    it("sets error on exception", async () => {
      mockCommands.getAvailableModels.mockRejectedValue(
        new Error("network down"),
      );

      await getStore().loadModels();

      expect(getStore().error).toContain("Failed to load models:");
      expect(getStore().loading).toBe(false);
    });

    it("syncs downloading state from backend", async () => {
      const downloadingModel = makeModel({
        id: "dl-1",
        is_downloading: true,
      });
      mockCommands.getAvailableModels.mockResolvedValue({
        status: "ok",
        data: [downloadingModel],
      });

      await getStore().loadModels();

      expect(getStore().downloadingModels["dl-1"]).toBe(true);
    });
  });

  describe("loadCurrentModel", () => {
    it("sets currentModel on success", async () => {
      mockCommands.getCurrentModel.mockResolvedValue({
        status: "ok",
        data: "active-model",
      });

      await getStore().loadCurrentModel();

      expect(getStore().currentModel).toBe("active-model");
    });

    it("does not throw on failure", async () => {
      mockCommands.getCurrentModel.mockRejectedValue(new Error("fail"));

      await expect(getStore().loadCurrentModel()).resolves.toBeUndefined();
    });
  });

  describe("checkFirstRun", () => {
    it("returns true when no models available", async () => {
      mockCommands.hasAnyModelsAvailable.mockResolvedValue({
        status: "ok",
        data: false,
      });

      const result = await getStore().checkFirstRun();

      expect(result).toBe(true);
      expect(getStore().isFirstRun).toBe(true);
      expect(getStore().hasAnyModels).toBe(false);
    });

    it("returns false when models exist", async () => {
      mockCommands.hasAnyModelsAvailable.mockResolvedValue({
        status: "ok",
        data: true,
      });

      const result = await getStore().checkFirstRun();

      expect(result).toBe(false);
      expect(getStore().isFirstRun).toBe(false);
      expect(getStore().hasAnyModels).toBe(true);
    });

    it("returns false on error", async () => {
      mockCommands.hasAnyModelsAvailable.mockRejectedValue(new Error("fail"));

      const result = await getStore().checkFirstRun();

      expect(result).toBe(false);
    });
  });

  describe("selectModel", () => {
    it("updates state on success", async () => {
      mockCommands.setActiveModel.mockResolvedValue({
        status: "ok",
        data: null,
      });

      const result = await getStore().selectModel("new-model");

      expect(result).toBe(true);
      expect(getStore().currentModel).toBe("new-model");
      expect(getStore().isFirstRun).toBe(false);
      expect(getStore().hasAnyModels).toBe(true);
    });

    it("sets error on command error", async () => {
      mockCommands.setActiveModel.mockResolvedValue({
        status: "error",
        error: "invalid model",
      });

      const result = await getStore().selectModel("bad-model");

      expect(result).toBe(false);
      expect(getStore().error).toContain("Failed to switch to model");
    });

    it("sets error on exception", async () => {
      mockCommands.setActiveModel.mockRejectedValue(new Error("boom"));

      const result = await getStore().selectModel("bad-model");

      expect(result).toBe(false);
      expect(getStore().error).toContain("Failed to switch to model");
    });

    it("clears previous error before attempt", async () => {
      getStore().setError("old error");
      mockCommands.setActiveModel.mockResolvedValue({
        status: "ok",
        data: null,
      });

      await getStore().selectModel("new-model");

      expect(getStore().error).toBeNull();
    });
  });

  describe("downloadModel", () => {
    it("sets downloading state and progress immediately", async () => {
      mockCommands.downloadModel.mockResolvedValue({
        status: "ok",
        data: null,
      });

      const promise = getStore().downloadModel("dl-model");

      // State should be set before await resolves
      expect(getStore().downloadingModels["dl-model"]).toBe(true);
      expect(getStore().downloadProgress["dl-model"]).toEqual({
        model_id: "dl-model",
        downloaded: 0,
        total: 0,
        percentage: 0,
      });

      const result = await promise;
      expect(result).toBe(true);
    });

    it("cleans up state on command error", async () => {
      mockCommands.downloadModel.mockResolvedValue({
        status: "error",
        error: "disk full",
      });

      const result = await getStore().downloadModel("dl-model");

      expect(result).toBe(false);
      expect(getStore().downloadingModels["dl-model"]).toBeUndefined();
      expect(getStore().downloadProgress["dl-model"]).toBeUndefined();
    });

    it("cleans up state on exception", async () => {
      mockCommands.downloadModel.mockRejectedValue(new Error("ipc error"));

      const result = await getStore().downloadModel("dl-model");

      expect(result).toBe(false);
      expect(getStore().downloadingModels["dl-model"]).toBeUndefined();
      expect(getStore().downloadProgress["dl-model"]).toBeUndefined();
    });
  });

  describe("cancelDownload", () => {
    it("cleans up state and reloads on success", async () => {
      // Set up downloading state
      useModelStore.setState({
        downloadingModels: { "dl-model": true },
        downloadProgress: {
          "dl-model": {
            model_id: "dl-model",
            downloaded: 50,
            total: 100,
            percentage: 50,
          },
        },
      });

      mockCommands.cancelDownload.mockResolvedValue({
        status: "ok",
        data: null,
      });
      mockCommands.getAvailableModels.mockResolvedValue({
        status: "ok",
        data: [],
      });

      const result = await getStore().cancelDownload("dl-model");

      expect(result).toBe(true);
      expect(getStore().downloadingModels["dl-model"]).toBeUndefined();
      expect(getStore().downloadProgress["dl-model"]).toBeUndefined();
      expect(mockCommands.getAvailableModels).toHaveBeenCalled();
    });

    it("sets error on failure", async () => {
      mockCommands.cancelDownload.mockResolvedValue({
        status: "error",
        error: "not found",
      });

      const result = await getStore().cancelDownload("dl-model");

      expect(result).toBe(false);
      expect(getStore().error).toContain("Failed to cancel download");
    });
  });

  describe("deleteModel", () => {
    it("reloads models and current model on success", async () => {
      mockCommands.deleteModel.mockResolvedValue({
        status: "ok",
        data: null,
      });
      mockCommands.getAvailableModels.mockResolvedValue({
        status: "ok",
        data: [],
      });
      mockCommands.getCurrentModel.mockResolvedValue({
        status: "ok",
        data: "",
      });

      const result = await getStore().deleteModel("del-model");

      expect(result).toBe(true);
      expect(mockCommands.getAvailableModels).toHaveBeenCalled();
      expect(mockCommands.getCurrentModel).toHaveBeenCalled();
    });

    it("sets error on failure", async () => {
      mockCommands.deleteModel.mockResolvedValue({
        status: "error",
        error: "in use",
      });

      const result = await getStore().deleteModel("del-model");

      expect(result).toBe(false);
      expect(getStore().error).toContain("Failed to delete model");
    });
  });

  describe("pure getters", () => {
    it("getModelInfo finds model by id", () => {
      const model = makeModel({ id: "find-me" });
      getStore().setModels([makeModel({ id: "other" }), model]);
      expect(getStore().getModelInfo("find-me")).toEqual(model);
    });

    it("getModelInfo returns undefined for missing id", () => {
      expect(getStore().getModelInfo("nope")).toBeUndefined();
    });

    it("isModelDownloading checks record", () => {
      expect(getStore().isModelDownloading("x")).toBe(false);
      useModelStore.setState({ downloadingModels: { x: true } });
      expect(getStore().isModelDownloading("x")).toBe(true);
    });

    it("isModelVerifying checks record", () => {
      expect(getStore().isModelVerifying("x")).toBe(false);
      useModelStore.setState({ verifyingModels: { x: true } });
      expect(getStore().isModelVerifying("x")).toBe(true);
    });

    it("isModelExtracting checks record", () => {
      expect(getStore().isModelExtracting("x")).toBe(false);
      useModelStore.setState({ extractingModels: { x: true } });
      expect(getStore().isModelExtracting("x")).toBe(true);
    });

    it("getDownloadProgress returns progress for model", () => {
      const progress = {
        model_id: "x",
        downloaded: 10,
        total: 100,
        percentage: 10,
      };
      useModelStore.setState({ downloadProgress: { x: progress } });
      expect(getStore().getDownloadProgress("x")).toEqual(progress);
    });

    it("getDownloadProgress returns undefined for unknown model", () => {
      expect(getStore().getDownloadProgress("unknown")).toBeUndefined();
    });
  });

  describe("initialize", () => {
    beforeEach(() => {
      mockCommands.getAvailableModels.mockResolvedValue({
        status: "ok",
        data: [],
      });
      mockCommands.getCurrentModel.mockResolvedValue({
        status: "ok",
        data: "default",
      });
      mockCommands.hasAnyModelsAvailable.mockResolvedValue({
        status: "ok",
        data: true,
      });
    });

    it("loads initial data and sets initialized", async () => {
      await getStore().initialize();

      expect(getStore().initialized).toBe(true);
      expect(getStore().currentModel).toBe("default");
      expect(mockCommands.getAvailableModels).toHaveBeenCalled();
      expect(mockCommands.getCurrentModel).toHaveBeenCalled();
      expect(mockCommands.hasAnyModelsAvailable).toHaveBeenCalled();
    });

    it("does not re-initialize if already initialized", async () => {
      await getStore().initialize();
      vi.clearAllMocks();

      await getStore().initialize();

      expect(mockCommands.getAvailableModels).not.toHaveBeenCalled();
    });

    it("registers event listeners", async () => {
      await getStore().initialize();

      expect(eventListeners["model-download-progress"]).toBeDefined();
      expect(eventListeners["model-download-complete"]).toBeDefined();
      expect(eventListeners["model-download-failed"]).toBeDefined();
      expect(eventListeners["model-verification-started"]).toBeDefined();
      expect(eventListeners["model-verification-completed"]).toBeDefined();
      expect(eventListeners["model-extraction-started"]).toBeDefined();
      expect(eventListeners["model-extraction-completed"]).toBeDefined();
      expect(eventListeners["model-download-cancelled"]).toBeDefined();
      expect(eventListeners["model-deleted"]).toBeDefined();
      expect(eventListeners["model-state-changed"]).toBeDefined();
    });
  });

  describe("event handlers", () => {
    beforeEach(async () => {
      mockCommands.getAvailableModels.mockResolvedValue({
        status: "ok",
        data: [],
      });
      mockCommands.getCurrentModel.mockResolvedValue({
        status: "ok",
        data: "",
      });
      mockCommands.hasAnyModelsAvailable.mockResolvedValue({
        status: "ok",
        data: true,
      });
      await getStore().initialize();
      vi.clearAllMocks();
    });

    it("model-download-progress updates progress state", () => {
      const progress = {
        model_id: "m1",
        downloaded: 50_000_000,
        total: 100_000_000,
        percentage: 50,
      };

      eventListeners["model-download-progress"]({ payload: progress });

      expect(getStore().downloadProgress["m1"]).toEqual(progress);
      expect(getStore().downloadStats["m1"]).toBeDefined();
      expect(getStore().downloadStats["m1"].totalDownloaded).toBe(50_000_000);
    });

    it("model-download-complete cleans up all download state", () => {
      useModelStore.setState({
        downloadingModels: { m1: true },
        verifyingModels: { m1: true },
        downloadProgress: {
          m1: {
            model_id: "m1",
            downloaded: 100,
            total: 100,
            percentage: 100,
          },
        },
        downloadStats: {
          m1: { startTime: 0, lastUpdate: 0, totalDownloaded: 100, speed: 1 },
        },
      });

      mockCommands.getAvailableModels.mockResolvedValue({
        status: "ok",
        data: [],
      });

      eventListeners["model-download-complete"]({ payload: "m1" });

      expect(getStore().downloadingModels["m1"]).toBeUndefined();
      expect(getStore().verifyingModels["m1"]).toBeUndefined();
      expect(getStore().downloadProgress["m1"]).toBeUndefined();
      expect(getStore().downloadStats["m1"]).toBeUndefined();
    });

    it("model-download-failed sets error and cleans up", () => {
      useModelStore.setState({
        downloadingModels: { m1: true },
      });

      eventListeners["model-download-failed"]({
        payload: { model_id: "m1", error: "checksum mismatch" },
      });

      expect(getStore().downloadingModels["m1"]).toBeUndefined();
      expect(getStore().error).toBe("checksum mismatch");
    });

    it("model-verification-started sets verifying state", () => {
      eventListeners["model-verification-started"]({ payload: "m1" });
      expect(getStore().verifyingModels["m1"]).toBe(true);
    });

    it("model-verification-completed clears verifying state", () => {
      useModelStore.setState({ verifyingModels: { m1: true } });
      eventListeners["model-verification-completed"]({ payload: "m1" });
      expect(getStore().verifyingModels["m1"]).toBeUndefined();
    });

    it("model-extraction-started sets extracting state", () => {
      eventListeners["model-extraction-started"]({ payload: "m1" });
      expect(getStore().extractingModels["m1"]).toBe(true);
    });

    it("model-extraction-completed clears extracting state", () => {
      useModelStore.setState({ extractingModels: { m1: true } });
      mockCommands.getAvailableModels.mockResolvedValue({
        status: "ok",
        data: [],
      });

      eventListeners["model-extraction-completed"]({ payload: "m1" });
      expect(getStore().extractingModels["m1"]).toBeUndefined();
    });

    it("model-extraction-failed sets error", () => {
      useModelStore.setState({ extractingModels: { m1: true } });

      eventListeners["model-extraction-failed"]({
        payload: { model_id: "m1", error: "corrupt archive" },
      });

      expect(getStore().extractingModels["m1"]).toBeUndefined();
      expect(getStore().error).toContain("Failed to extract model");
    });

    it("model-download-cancelled cleans up state", () => {
      useModelStore.setState({
        downloadingModels: { m1: true },
        verifyingModels: { m1: true },
        downloadProgress: {
          m1: {
            model_id: "m1",
            downloaded: 50,
            total: 100,
            percentage: 50,
          },
        },
        downloadStats: {
          m1: { startTime: 0, lastUpdate: 0, totalDownloaded: 50, speed: 1 },
        },
      });

      eventListeners["model-download-cancelled"]({ payload: "m1" });

      expect(getStore().downloadingModels["m1"]).toBeUndefined();
      expect(getStore().verifyingModels["m1"]).toBeUndefined();
      expect(getStore().downloadProgress["m1"]).toBeUndefined();
      expect(getStore().downloadStats["m1"]).toBeUndefined();
    });
  });
});
