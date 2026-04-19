/**
 * Tauri IPC mock for the settings UI audit spec. Injected into every
 * page load so React/Zustand stores can render without a running Tauri
 * backend. Extracted from tests/settingsUIAudit.spec.ts to keep the
 * spec under the 800-line file cap and to share the same mock with any
 * future settings-audit specs (e.g. split-out R-006 / R-008 specs).
 */
import type { Page } from "@playwright/test";

export const TAURI_MOCK_SCRIPT = `<script>
  window.__TAURI_OS_PLUGIN_INTERNALS__ = {
    platform: "windows", version: "10.0", os_type: "windows_nt",
    family: "windows", arch: "x86_64", exe_extension: "exe",
    eol: "\\r\\n", hostname: "test-host", locale: "en-US",
  };
  var _cbId = 0;
  var defaultCaptionProfile = {
    font_size: 40,
    bg_color: "#000000B3",
    text_color: "#FFFFFF",
    position: 90,
    font_family: "Inter",
    radius_px: 0,
    padding_x_px: 12,
    padding_y_px: 4,
    max_width_percent: 90,
  };
  var defaultSettings = {
    always_on_microphone: false,
    selected_microphone: "Default",
    clamshell_microphone: "Default",
    selected_output_device: "Default",
    sound_enabled: true,
    sound_theme: "default",
    start_hidden: false,
    autostart_enabled: false,
    update_checks_enabled: false,
    push_to_talk: false,
    app_language: "en",
    show_tray_icon: true,
    model_unload_timeout: 300,
    acceleration: "auto",
    simplify_mode: "basic",
    debug_mode: false,
    discard_words: "",
    allow_words: "",
    theme: "system",
    normalize_audio_on_export: false,
    caption_profiles: {
      desktop: defaultCaptionProfile,
      mobile: Object.assign({}, defaultCaptionProfile, { font_size: 48, max_width_percent: 80, position: 80, radius_px: 8, padding_x_px: 14, padding_y_px: 6 }),
    },
  };
  window.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { label: "main" },
    },
    transformCallback: function(cb, once) { return _cbId++; },
    invoke: async function(cmd, args) {
      if (cmd === "plugin:event|listen") return 0;
      if (cmd === "plugin:event|unlisten") return;
      if (cmd === "plugin:app|version") return "0.1.0";
      if (cmd === "plugin:app|name") return "toaster";
      if (cmd === "plugin:app|tauri_version") return "2.0.0";
      if (cmd === "get_app_settings") return defaultSettings;
      if (cmd === "get_default_settings") return defaultSettings;
      if (cmd === "update_app_settings") {
        if (args && args.settings) Object.assign(defaultSettings, args.settings);
        return null;
      }
      if (cmd === "get_available_models") return [];
      if (cmd === "get_downloaded_models") return [];
      if (cmd === "get_current_model") return "";
      if (cmd === "has_any_models_available") return true;
      if (cmd === "get_windows_microphone_permission_status")
        return { supported: false, overall_access: "allowed" };
      if (cmd === "get_available_microphones") return [];
      if (cmd === "get_available_output_devices") return [];
      if (cmd === "is_first_run") return false;
      if (cmd === "initialize_enigo") return null;
      if (cmd === "initialize_shortcuts") return null;
      return null;
    },
    convertFileSrc: function(p) { return p; },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: function() {} };
</script>`;

export async function setupTauriMocks(page: Page): Promise<void> {
  await page.route("**/", async (route) => {
    const response = await route.fetch();
    const html = await response.text();
    const modified = html.replace("<head>", `<head>${TAURI_MOCK_SCRIPT}`);
    await route.fulfill({ response, body: modified });
  });
}
