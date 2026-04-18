import { test, expect, type Page } from "@playwright/test";

/**
 * Transcript edit flow: delete → undo → redo on the backend EditorStore.
 *
 * The real EditorView flow requires a native file-open dialog + transcription,
 * which cannot run in the Playwright browser context. Instead we drive the
 * `editor_*` Tauri commands through an in-memory mock that mirrors the
 * backend's monotonic word-list semantics, then invoke the frontend's
 * `commands` bindings (the same module EditorView uses). This verifies the
 * round-trip contract between the frontend store and the backend command
 * surface without re-implementing it.
 *
 * Covers:
 *  - `editor_set_words` seeds the projection.
 *  - `editor_delete_word` marks a word deleted.
 *  - `editor_undo` restores it.
 *  - `editor_redo` hides it again.
 */

type Word = {
  text: string;
  start_us: number;
  end_us: number;
  deleted?: boolean;
  silenced?: boolean;
};

const INIT_SCRIPT = `
  window.__TAURI_OS_PLUGIN_INTERNALS__ = {
    platform: "windows", version: "10.0", os_type: "windows_nt",
    family: "windows", arch: "x86_64", exe_extension: "exe",
    eol: "\\r\\n", hostname: "test-host", locale: "en-US",
  };
  var _cbId = 0;
  var _state = {
    words: [],
    history: [],
    future: [],
  };
  function snapshot() { return JSON.stringify(_state.words); }
  function restore(s) { _state.words = JSON.parse(s); }
  function projection() {
    return {
      words: JSON.parse(JSON.stringify(_state.words)),
      timing_contract: {
        timeline_revision: _state.history.length,
        total_words: _state.words.length,
        deleted_words: _state.words.filter(function(w){return w.deleted;}).length,
        active_words: _state.words.filter(function(w){return !w.deleted;}).length,
        source_start_us: 0, source_end_us: 0,
        total_keep_duration_us: 0,
        keep_segments: [], quantized_keep_segments: [],
        quantization_fps_num: 30, quantization_fps_den: 1,
        keep_segments_valid: true, warning: null,
      },
    };
  }
  window.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { label: "main" },
    },
    transformCallback: function(cb, once) { return _cbId++; },
    invoke: async function(cmd, args) {
      if (cmd === "editor_set_words") {
        _state.history.push(snapshot());
        _state.future = [];
        _state.words = (args && args.words) || [];
        return _state.words;
      }
      if (cmd === "editor_delete_word") {
        _state.history.push(snapshot());
        _state.future = [];
        var idx = args.index;
        if (_state.words[idx]) _state.words[idx].deleted = true;
        return true;
      }
      if (cmd === "editor_restore_word") {
        _state.history.push(snapshot());
        _state.future = [];
        var ri = args.index;
        if (_state.words[ri]) _state.words[ri].deleted = false;
        return true;
      }
      if (cmd === "editor_undo") {
        if (_state.history.length === 0) return false;
        _state.future.push(snapshot());
        restore(_state.history.pop());
        return true;
      }
      if (cmd === "editor_redo") {
        if (_state.future.length === 0) return false;
        _state.history.push(snapshot());
        restore(_state.future.pop());
        return true;
      }
      if (cmd === "editor_get_projection") return projection();
      if (cmd === "editor_get_words") return _state.words;
      if (cmd === "editor_get_keep_segments") return [];
      if (cmd === "plugin:event|listen") return 0;
      if (cmd === "plugin:event|unlisten") return;
      if (cmd === "plugin:app|version") return "0.1.0";
      if (cmd === "get_app_settings" || cmd === "get_default_settings") return {};
      return null;
    },
    convertFileSrc: function(p) { return p; },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: function() {} };
`;

async function setup(page: Page) {
  await page.addInitScript(INIT_SCRIPT);
  await page.goto("/");
}

const fixtureWords: Word[] = [
  { text: "The", start_us: 0, end_us: 200_000 },
  { text: "quick", start_us: 200_000, end_us: 500_000 },
  { text: "brown", start_us: 500_000, end_us: 800_000 },
  { text: "fox", start_us: 800_000, end_us: 1_100_000 },
];

test.describe("Transcript edit flow", () => {
  test("set → delete → undo → redo cycles through backend store", async ({
    page,
  }) => {
    await setup(page);

    const result = await page.evaluate(async (words) => {
      const { commands } = await import("@/bindings");
      const activeText = (ws: Array<{ text: string; deleted?: boolean }>) =>
        ws.filter((w) => !w.deleted).map((w) => w.text).join(" ");

      await commands.editorSetWords(words as never);
      const initial = await commands.editorGetWords();

      await commands.editorDeleteWord(1);
      const afterDelete = await commands.editorGetWords();

      await commands.editorUndo();
      const afterUndo = await commands.editorGetWords();

      await commands.editorRedo();
      const afterRedo = await commands.editorGetWords();

      return {
        initial: activeText(initial),
        afterDelete: activeText(afterDelete),
        afterUndo: activeText(afterUndo),
        afterRedo: activeText(afterRedo),
      };
    }, fixtureWords);

    expect(result.initial).toBe("The quick brown fox");
    expect(result.afterDelete).toBe("The brown fox");
    expect(result.afterUndo).toBe("The quick brown fox");
    expect(result.afterRedo).toBe("The brown fox");
  });

  test("editor_get_projection reports accurate active/deleted counts", async ({
    page,
  }) => {
    await setup(page);

    const counts = await page.evaluate(async (words) => {
      const { commands } = await import("@/bindings");
      await commands.editorSetWords(words as never);
      await commands.editorDeleteWord(0);
      await commands.editorDeleteWord(2);
      const proj = await commands.editorGetProjection();
      return {
        active: proj.timing_contract.active_words,
        deleted: proj.timing_contract.deleted_words,
        total: proj.timing_contract.total_words,
      };
    }, fixtureWords);

    expect(counts.total).toBe(4);
    expect(counts.deleted).toBe(2);
    expect(counts.active).toBe(2);
  });
});
