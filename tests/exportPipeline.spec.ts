import { test, expect, type Page } from "@playwright/test";

/**
 * Export pipeline contract: the frontend `exportTranscriptToFile` binding
 * must invoke the backend `export_transcript_to_file` command with the
 * expected argument shape (format, path, maxCharsPerLine, includeSilenced).
 *
 * We cannot drive the full UI export flow (native save-dialog + FFmpeg) in
 * Playwright. Instead we record `window.__TAURI_INTERNALS__.invoke` calls
 * and verify the binding emits the correct Tauri command with correct args.
 * This is the contract EditorView depends on at
 * `src/components/editor/EditorView.tsx:252`.
 */

const RECORD_SCRIPT = `
  window.__TAURI_OS_PLUGIN_INTERNALS__ = {
    platform: "windows", version: "10.0", os_type: "windows_nt",
    family: "windows", arch: "x86_64", exe_extension: "exe",
    eol: "\\r\\n", hostname: "test-host", locale: "en-US",
  };
  window.__invokeCalls = [];
  var _cbId = 0;
  window.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { label: "main" },
    },
    transformCallback: function(cb, once) { return _cbId++; },
    invoke: async function(cmd, args) {
      window.__invokeCalls.push({ cmd: cmd, args: args });
      if (cmd === "plugin:event|listen") return 0;
      if (cmd === "plugin:event|unlisten") return;
      if (cmd === "plugin:app|version") return "0.1.0";
      if (cmd === "get_app_settings" || cmd === "get_default_settings") return {};
      if (cmd === "export_transcript") return "mock transcript content";
      if (cmd === "export_transcript_to_file") return null;
      return null;
    },
    convertFileSrc: function(p) { return p; },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: function() {} };
`;

async function setup(page: Page) {
  await page.addInitScript(RECORD_SCRIPT);
  await page.goto("/");
}

test.describe("Export pipeline bindings contract", () => {
  test("exportTranscriptToFile invokes export_transcript_to_file with expected args", async ({
    page,
  }) => {
    await setup(page);

    const recorded = await page.evaluate(async () => {
      const { commands } = await import("@/bindings");
      await commands.exportTranscriptToFile("Srt", "C:/out/transcript.srt", null, null);
      return (window as unknown as {
        __invokeCalls: Array<{ cmd: string; args: unknown }>;
      }).__invokeCalls.filter((c) => c.cmd === "export_transcript_to_file");
    });

    expect(recorded).toHaveLength(1);
    expect(recorded[0].args).toEqual({
      format: "Srt",
      path: "C:/out/transcript.srt",
      maxCharsPerLine: null,
      includeSilenced: null,
    });
  });

  test("exportTranscript (in-memory) forwards caption config", async ({
    page,
  }) => {
    await setup(page);

    const recorded = await page.evaluate(async () => {
      const { commands } = await import("@/bindings");
      await commands.exportTranscript("Vtt", 42, true);
      return (window as unknown as {
        __invokeCalls: Array<{ cmd: string; args: unknown }>;
      }).__invokeCalls.filter((c) => c.cmd === "export_transcript");
    });

    expect(recorded).toHaveLength(1);
    expect(recorded[0].args).toEqual({
      format: "Vtt",
      maxCharsPerLine: 42,
      includeSilenced: true,
    });
  });

  // TODO: drive handleExportEditedMedia (FFmpeg splice) end-to-end. This
  // requires a Tauri runtime; Playwright alone cannot exercise it. See
  // `export_edited_media` in src-tauri/src/commands/media.rs.
  test.skip("handleExportEditedMedia invokes export_edited_media with keep_segments", () => {});
});
