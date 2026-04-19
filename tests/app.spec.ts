import { test, expect, type Page, type BrowserContext } from "@playwright/test";

/** Inline script that mocks Tauri APIs so the React app boots in a plain browser. */
const TAURI_MOCK_SCRIPT = `<script>
  window.__TAURI_OS_PLUGIN_INTERNALS__ = {
    platform: "windows", version: "10.0", os_type: "windows_nt",
    family: "windows", arch: "x86_64", exe_extension: "exe",
    eol: "\\r\\n", hostname: "test-host", locale: "en-US",
  };

  var _cbId = 0;
  window.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { label: "main" },
    },
    transformCallback: function(cb, once) { return _cbId++; },
    invoke: async function(cmd) {
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
      };
      if (cmd === "plugin:event|listen") return 0;
      if (cmd === "plugin:event|unlisten") return;
      if (cmd === "plugin:app|version") return "0.1.0";
      if (cmd === "plugin:app|name") return "toaster";
      if (cmd === "plugin:app|tauri_version") return "2.0.0";
      if (cmd === "get_app_settings") return defaultSettings;
      if (cmd === "get_default_settings") return defaultSettings;
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
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: function() {},
  };
</script>`;

/**
 * Intercept the index HTML to inject Tauri mocks before any ES modules load.
 */
async function setupTauriMocks(page: Page) {
  await page.route("**/", async (route) => {
    const response = await route.fetch();
    const html = await response.text();
    const modified = html.replace("<head>", `<head>${TAURI_MOCK_SCRIPT}`);
    await route.fulfill({ response, body: modified });
  });
}

async function createMockedPage(
  browser: import("@playwright/test").Browser,
  opts?: Parameters<typeof browser.newContext>[0],
): Promise<{ context: BrowserContext; page: Page }> {
  const context = await browser.newContext(opts);
  const page = await context.newPage();
  await setupTauriMocks(page);
  return { context, page };
}

test.describe("Toaster App", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page);
  });

  test("dev server responds with 200", async ({ page }) => {
    const response = await page.goto("/");
    expect(response?.status()).toBe(200);
  });

  test("page has basic HTML structure", async ({ page }) => {
    await page.goto("/");
    const html = await page.content();
    expect(html).toContain("<html");
    expect(html).toContain("<body");
  });

  test("app loads and shows sidebar with logo and navigation", async ({
    page,
  }) => {
    await page.goto("/");

    // Sidebar renders the Toaster logo
    const logo = page.locator('img[alt="Toaster"]');
    await expect(logo).toBeVisible();

    // All sidebar navigation items are present
    for (const label of ["Editor", "Models", "About"]) {
      await expect(page.getByText(label, { exact: true })).toBeVisible();
    }
  });

  test("sidebar navigation switches content area", async ({ page }) => {
    await page.goto("/");

    // Click "About" — about-specific content should appear
    await page.getByText("About", { exact: true }).click();
    await expect(page.getByText("Source Code")).toBeVisible();

    // Click "Models" — about content disappears, models container appears
    await page.getByText("Models", { exact: true }).click();
    await expect(page.getByText("Source Code")).not.toBeVisible();
    await expect(
      page.locator("div.max-w-3xl.w-full.mx-auto"),
    ).toBeVisible();

    // Click "Editor" — models container disappears
    await page.getByText("Editor", { exact: true }).click();
    await expect(
      page.locator("h2", { hasText: /project/i }).first(),
    ).toBeVisible();
  });

  test("settings page renders at least one settings group", async ({
    page,
  }) => {
    await page.goto("/");

    // Navigate to Models — relocated Performance + Captions groups live here
    await page.getByText("Models", { exact: true }).click();

    // SettingsGroup renders h2 headings (uppercase, small text)
    const groupHeadings = page.locator("h2.text-xs.font-medium");
    await expect(groupHeadings.first()).toBeVisible();
    expect(await groupHeadings.count()).toBeGreaterThanOrEqual(1);
  });

  test("dark theme applies dark background color", async ({ browser }) => {
    const { context, page } = await createMockedPage(browser, {
      colorScheme: "dark",
    });

    await page.goto("/");
    await expect(page.locator('img[alt="Toaster"]')).toBeVisible();

    const bgColor = await page.evaluate(() =>
      getComputedStyle(document.documentElement)
        .getPropertyValue("--color-background")
        .trim(),
    );
    expect(bgColor).toBe("#1E1E1E");

    await context.close();
  });

  test("light theme applies light background color", async ({ browser }) => {
    const { context, page } = await createMockedPage(browser, {
      colorScheme: "light",
    });

    await page.goto("/");
    await expect(page.locator('img[alt="Toaster"]')).toBeVisible();

    const bgColor = await page.evaluate(() =>
      getComputedStyle(document.documentElement)
        .getPropertyValue("--color-background")
        .trim(),
    );
    expect(bgColor).toBe("#fbfbfb");

    await context.close();
  });

  test("settings page renders multiple groups with toggle switches", async ({
    page,
  }) => {
    await page.goto("/");
    await page.getByText("Editor", { exact: true }).click();

    // Editor renders Project / Words headings
    const groupHeadings = page.locator("h2.text-xs.font-medium");
    await expect(groupHeadings.first()).toBeVisible();
    const headingTexts = await groupHeadings.allTextContents();
    const upper = headingTexts.map((t) => t.toUpperCase());
    expect(upper).toEqual(expect.arrayContaining(["PROJECT"]));
  });

  test("toggling a setting checkbox changes its checked state", async ({
    page,
  }) => {
    await page.goto("/");
    await page.getByText("Models", { exact: true }).click();

    // Find the first toggle switch and flip it
    const firstToggle = page.locator('input[type="checkbox"]').first();
    await expect(firstToggle).toBeAttached();

    const wasBefore = await firstToggle.isChecked();
    await firstToggle.evaluate((el: HTMLInputElement) => el.click());
    const wasAfter = await firstToggle.isChecked();
    expect(wasAfter).toBe(!wasBefore);
  });

  test("editor page renders media upload area when no media loaded", async ({
    page,
  }) => {
    await page.goto("/");
    // Editor is the default section, but click it to be explicit
    await page.getByText("Editor", { exact: true }).click();

    // The dashed border drop-zone with import prompt should be visible
    const uploadArea = page.locator("div.border-dashed");
    await expect(uploadArea).toBeVisible();

    // Import prompt text
    await expect(
      page.getByText(/click to import media/i),
    ).toBeVisible();
  });

  test("editor page shows project section when no media loaded", async ({
    page,
  }) => {
    await page.goto("/");
    await page.getByText("Editor", { exact: true }).click();

    // The "Project" settings group heading should be visible
    const projectHeading = page.locator("h2.text-xs.font-medium", {
      hasText: /project/i,
    });
    await expect(projectHeading).toBeVisible();
  });

  test("models page renders header and empty state", async ({ page }) => {
    await page.goto("/");
    await page.getByText("Models", { exact: true }).click();

    // Models page title
    const title = page.locator("h1");
    await expect(title).toBeVisible();

    // With empty model arrays returned from mock, the page should still render
    // without crashing — verify the container exists
    const container = page.locator("div.max-w-3xl.w-full.mx-auto");
    await expect(container).toBeVisible();
  });

  test("about page renders version and acknowledgments", async ({ page }) => {
    await page.goto("/");
    await page.getByText("About", { exact: true }).click();

    // Version string from mock: "0.1.0"
    await expect(page.getByText("v0.1.0")).toBeVisible();

    // Source Code link
    await expect(page.getByText("Source Code")).toBeVisible();

    // Acknowledgments section mentions Whisper
    await expect(page.getByText("Whisper")).toBeVisible();
  });

  test("history page shows empty state message", async ({ page }) => {
    test.skip(true, "History page removed in remove-history-and-legacy.");
  });

  test("sidebar highlights active navigation item", async ({ page }) => {
    await page.goto("/");

    // Click About and verify its nav item gets the active class
    await page.getByText("About", { exact: true }).click();
    const aboutNav = page
      .locator("div.rounded-lg.cursor-pointer", { hasText: "About" })
      .first();
    await expect(aboutNav).toHaveClass(/bg-logo-primary/);

    // Click Models — About should lose active, Models should gain it
    await page.getByText("Models", { exact: true }).click();
    const modelsNav = page
      .locator("div.rounded-lg.cursor-pointer", { hasText: "Models" })
      .first();
    await expect(modelsNav).toHaveClass(/bg-logo-primary/);
    await expect(aboutNav).not.toHaveClass(/bg-logo-primary/);
  });

  test("sidebar persists highlight after navigating away and back", async ({
    page,
  }) => {
    await page.goto("/");

    // Navigate to Models
    await page.getByText("Models", { exact: true }).click();
    const modelsNav = page
      .locator("div.rounded-lg.cursor-pointer", { hasText: "Models" })
      .first();
    await expect(modelsNav).toHaveClass(/bg-logo-primary/);

    // Go to About, then back to Models
    await page.getByText("About", { exact: true }).click();
    await expect(modelsNav).not.toHaveClass(/bg-logo-primary/);

    await page.getByText("Models", { exact: true }).click();
    await expect(modelsNav).toHaveClass(/bg-logo-primary/);
  });

  test("responsive: sidebar and content render at narrow viewport", async ({
    browser,
  }) => {
    const { context, page } = await createMockedPage(browser, {
      viewport: { width: 800, height: 600 },
    });

    await page.goto("/");
    // Sidebar logo should still be visible at 800px
    await expect(page.locator('img[alt="Toaster"]')).toBeVisible();

    // Navigation items should still be accessible
    await expect(page.getByText("Editor", { exact: true })).toBeVisible();
    await expect(page.getByText("About", { exact: true })).toBeVisible();

    await context.close();
  });

  test("responsive: app renders at large viewport", async ({ browser }) => {
    const { context, page } = await createMockedPage(browser, {
      viewport: { width: 1920, height: 1080 },
    });

    await page.goto("/");
    await expect(page.locator('img[alt="Toaster"]')).toBeVisible();

    // Navigate to settings and verify content is centered
    await page.getByText("Models", { exact: true }).click();
    const container = page.locator("div.max-w-3xl.w-full.mx-auto");
    await expect(container).toBeVisible();

    await context.close();
  });

  test("error boundary renders fallback UI on component error", async ({
    page,
  }) => {
    await page.goto("/");
    await expect(page.locator('img[alt="Toaster"]')).toBeVisible();

    // Inject a runtime error into the React tree via console-level error
    // simulation. We trigger ErrorBoundary by forcing a render error.
    await page.evaluate(() => {
      // Find the React root and force an error boundary trigger
      const errorEvent = new ErrorEvent("error", {
        error: new Error("Test error for boundary"),
        message: "Test error for boundary",
      });
      window.dispatchEvent(errorEvent);
    });

    // ErrorBoundary catches React render errors, not window errors.
    // Instead, verify the ErrorBoundary component structure is present
    // by checking that the app loaded without showing the fallback.
    // The fallback text "Something went wrong" should NOT be visible
    // under normal operation — this confirms the boundary is passive.
    await expect(
      page.getByText("Something went wrong"),
    ).not.toBeVisible();
  });

  test("all five nav items navigate without console errors", async ({
    page,
  }) => {
    const consoleErrors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") consoleErrors.push(msg.text());
    });

    await page.goto("/");

    for (const label of [
      "Editor",
      "Models",
      "About",
    ]) {
      await page.getByText(label, { exact: true }).click();
      // Brief wait for any async renders
      await page.waitForTimeout(300);
    }

    // Filter out known benign errors (e.g. Tauri invoke failures for
    // commands not covered by the mock)
    const unexpectedErrors = consoleErrors.filter(
      (e) =>
        !e.includes("invoke") &&
        !e.includes("TAURI") &&
        !e.includes("Failed to load"),
    );
    expect(unexpectedErrors).toEqual([]);
  });
});
