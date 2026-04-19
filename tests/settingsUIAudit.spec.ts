/**
 * Settings UI consistency audit — executes R-001..R-005 against every
 * settings route at desktop + mobile viewports and emits a structured
 * violations report. See features/settings-ui-consistency-audit/PRD.md.
 */
import { test, expect, type Page } from "@playwright/test";
import * as fs from "node:fs";
import * as path from "node:path";

// ----- Tauri mock (mirrors tests/app.spec.ts) --------------------------------

const TAURI_MOCK_SCRIPT = `<script>
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

async function setupTauriMocks(page: Page) {
  await page.route("**/", async (route) => {
    const response = await route.fetch();
    const html = await response.text();
    const modified = html.replace("<head>", `<head>${TAURI_MOCK_SCRIPT}`);
    await route.fulfill({ response, body: modified });
  });
}

// ----- Configuration --------------------------------------------------------

const ROUTES = [
  { id: "about", label: "About", expectMaxW5xl: true } as const,
  { id: "models", label: "Models", expectMaxW5xl: true } as const,
  { id: "post-process", label: "Post Process", expectMaxW5xl: true } as const,
  { id: "advanced", label: "Advanced", expectMaxW5xl: true } as const,
  {
    id: "captions",
    label: "Advanced",
    expectMaxW5xl: true,
    captionTab: "Desktop",
  } as const,
];

const VIEWPORTS = [
  { id: "desktop-1280x800", width: 1280, height: 800 } as const,
  { id: "mobile-portrait-390x844", width: 390, height: 844 } as const,
];

const FILE_HINTS: Record<string, string> = {
  about: "src/components/settings/about/AboutSettings.tsx",
  models: "src/components/settings/models/ModelsSettings.tsx",
  "post-process":
    "src/components/settings/post-processing/PostProcessingSettings.tsx",
  advanced: "src/components/settings/advanced/AdvancedSettings.tsx",
  captions: "src/components/settings/advanced/AdvancedSettings.tsx",
};

type Severity = "critical" | "major" | "minor";
type Violation = {
  page: string;
  viewport: string;
  rule: string;
  severity: Severity;
  selector: string;
  expected: Record<string, unknown>;
  actual: Record<string, unknown>;
  screenshotPath: string | null;
  fileHint?: string;
};

// Module-level accumulator (sole global state per spec requirement).
const violations: Violation[] = [];

const OUTPUT_DIR =
  process.env.AUDIT_OUTPUT_DIR ||
  path.join(process.cwd(), "test-results", "settings-ui-audit");
const SCREENSHOT_DIR = path.join(OUTPUT_DIR, "screenshots");

function ensureDir(p: string) {
  fs.mkdirSync(p, { recursive: true });
}

let screenshotCounter = 0;
async function captureScreenshot(
  page: Page,
  routeId: string,
  viewportId: string,
  rule: string,
  locatorSelector: string | null,
): Promise<string | null> {
  ensureDir(SCREENSHOT_DIR);
  const idx = screenshotCounter++;
  const safeRule = rule.replace(/[^a-zA-Z0-9_-]/g, "_");
  const fname = `${routeId}-${viewportId}-${safeRule}-${idx}.png`;
  const absPath = path.join(SCREENSHOT_DIR, fname);
  try {
    if (locatorSelector) {
      const loc = page.locator(locatorSelector).first();
      if ((await loc.count()) > 0) {
        await loc.screenshot({ path: absPath, timeout: 5000 });
      } else {
        await page.screenshot({ path: absPath, fullPage: false });
      }
    } else {
      await page.screenshot({ path: absPath, fullPage: false });
    }
    return path.join("screenshots", fname).replace(/\\/g, "/");
  } catch {
    try {
      await page.screenshot({ path: absPath, fullPage: false });
      return path.join("screenshots", fname).replace(/\\/g, "/");
    } catch {
      return null;
    }
  }
}

async function pushViolation(
  page: Page,
  v: Omit<Violation, "screenshotPath" | "fileHint"> & {
    screenshotSelector?: string | null;
  },
) {
  const { screenshotSelector, ...rest } = v;
  const screenshotPath = await captureScreenshot(
    page,
    v.page,
    v.viewport,
    v.rule,
    screenshotSelector ?? null,
  );
  violations.push({
    ...rest,
    screenshotPath,
    fileHint: FILE_HINTS[v.page],
  });
}

// ----- Navigation helpers ---------------------------------------------------

async function navigateToRoute(
  page: Page,
  route: (typeof ROUTES)[number],
): Promise<void> {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");
  // Click sidebar label
  const nav = page.getByText(route.label, { exact: true }).first();
  await nav.click();
  // Wait for outer marker or a short timeout
  await page
    .locator('[data-testid="settings-outer"]')
    .first()
    .waitFor({ state: "attached", timeout: 5000 })
    .catch(() => undefined);

  if ("captionTab" in route && route.captionTab) {
    const tab = page.locator(`button[role="tab"]`, { hasText: route.captionTab });
    if ((await tab.count()) > 0) {
      await tab.first().click().catch(() => undefined);
      await page.waitForTimeout(150);
    }
  }
}

// ----- Rule checks (R-002..R-005) -------------------------------------------

async function checkR002OuterPadding(
  page: Page,
  routeId: string,
  viewportId: string,
  baseline: { paddingInline: number; maxWidth: number } | null,
): Promise<{ paddingInline: number; maxWidth: number } | null> {
  const outer = page.locator('[data-testid="settings-outer"]').first();
  if ((await outer.count()) === 0) {
    await pushViolation(page, {
      page: routeId,
      viewport: viewportId,
      rule: "R-002-outer-missing",
      severity: "critical",
      selector: '[data-testid="settings-outer"]',
      expected: { present: true },
      actual: { present: false },
      screenshotSelector: null,
    });
    return null;
  }

  const metrics = await outer.evaluate((el) => {
    const cs = getComputedStyle(el);
    return {
      paddingInline:
        parseFloat(cs.paddingLeft || "0") + parseFloat(cs.paddingRight || "0"),
      maxWidth:
        cs.maxWidth && cs.maxWidth !== "none"
          ? parseFloat(cs.maxWidth)
          : el.getBoundingClientRect().width,
    };
  });

  if (baseline) {
    const dPad = Math.abs(metrics.paddingInline - baseline.paddingInline);
    const dMax = Math.abs(metrics.maxWidth - baseline.maxWidth);
    if (dPad > 8 || dMax > 8) {
      await pushViolation(page, {
        page: routeId,
        viewport: viewportId,
        rule: "R-002-outer-padding-drift",
        severity: "major",
        selector: '[data-testid="settings-outer"]',
        expected: {
          paddingInline: baseline.paddingInline,
          maxWidth: baseline.maxWidth,
          tolerancePx: 8,
        },
        actual: metrics,
        screenshotSelector: '[data-testid="settings-outer"]',
      });
    }
  }
  return metrics;
}

async function checkR003TwoColumn(
  page: Page,
  routeId: string,
  viewportId: string,
  isDesktop: boolean,
): Promise<void> {
  const rows = page.locator('[data-testid="setting-row"]');
  const count = await rows.count();
  for (let i = 0; i < count; i++) {
    const row = rows.nth(i);
    if (!(await row.isVisible().catch(() => false))) continue;

    const shape = await row.evaluate((el) => {
      const label = el.querySelector('[data-setting-role="label"]');
      const control = el.querySelector('[data-setting-role="control"]');
      const cs = getComputedStyle(el as Element);
      const layoutHint = (el as HTMLElement).dataset.settingLayout ?? null;
      let order: "label-first" | "control-first" | "same" | "missing" =
        "missing";
      if (label && control) {
        const rel = label.compareDocumentPosition(control);
        if (rel & Node.DOCUMENT_POSITION_FOLLOWING) order = "label-first";
        else if (rel & Node.DOCUMENT_POSITION_PRECEDING) order = "control-first";
        else order = "same";
      }
      return {
        hasLabel: !!label,
        hasControl: !!control,
        order,
        display: cs.display,
        flexDirection: cs.flexDirection,
        gridTemplateColumns: cs.gridTemplateColumns,
        layoutHint,
      };
    });

    if (!shape.hasLabel || !shape.hasControl) {
      await pushViolation(page, {
        page: routeId,
        viewport: viewportId,
        rule: "R-003-missing-role",
        severity: "critical",
        selector: `[data-testid="setting-row"]:nth-of-type(${i + 1})`,
        expected: { hasLabel: true, hasControl: true },
        actual: { hasLabel: shape.hasLabel, hasControl: shape.hasControl },
        screenshotSelector: '[data-testid="setting-row"]',
      });
      continue;
    }

    if (shape.order !== "label-first") {
      await pushViolation(page, {
        page: routeId,
        viewport: viewportId,
        rule: "R-003-order",
        severity: "major",
        selector: `[data-testid="setting-row"]:nth-of-type(${i + 1})`,
        expected: { order: "label-first" },
        actual: { order: shape.order },
        screenshotSelector: '[data-testid="setting-row"]',
      });
    }

    if (isDesktop && shape.layoutHint !== "stacked") {
      let ok = false;
      if (shape.display === "flex" && shape.flexDirection === "row") ok = true;
      else if (shape.display === "grid") {
        const tracks = (shape.gridTemplateColumns || "").split(/\s+/).filter(
          (s) => s && s !== "none",
        ).length;
        if (tracks >= 2) ok = true;
      }
      if (!ok) {
        await pushViolation(page, {
          page: routeId,
          viewport: viewportId,
          rule:
            routeId === "advanced"
              ? "R-003-export-two-column"
              : "R-003-layout",
          severity: "major",
          selector: `[data-testid="setting-row"]:nth-of-type(${i + 1})`,
          expected: {
            display: "flex|grid",
            flexDirection: "row (if flex)",
            gridTracks: ">=2 (if grid)",
          },
          actual: {
            display: shape.display,
            flexDirection: shape.flexDirection,
            gridTemplateColumns: shape.gridTemplateColumns,
          },
          screenshotSelector: '[data-testid="setting-row"]',
        });
      }
    }
  }
}

async function checkR004PreviewClamp(
  page: Page,
  viewport: (typeof VIEWPORTS)[number],
): Promise<void> {
  const pane = page.locator('[data-testid="caption-preview-pane"]').first();
  if ((await pane.count()) === 0) {
    await pushViolation(page, {
      page: "captions",
      viewport: viewport.id,
      rule: "R-004-preview-missing",
      severity: "critical",
      selector: '[data-testid="caption-preview-pane"]',
      expected: { present: true },
      actual: { present: false },
      screenshotSelector: null,
    });
    return;
  }

  const box = await pane.boundingBox();
  if (!box) return;

  if (viewport.id === "desktop-1280x800") {
    const ratio = box.width / viewport.width;
    if (ratio > 0.5) {
      await pushViolation(page, {
        page: "captions",
        viewport: viewport.id,
        rule: "R-004-desktop-width",
        severity: "critical",
        selector: '[data-testid="caption-preview-pane"]',
        expected: { maxWidthRatio: 0.5 },
        actual: { widthRatio: ratio, widthPx: box.width },
        screenshotSelector: '[data-testid="caption-preview-pane"]',
      });
    }
  } else if (viewport.id === "mobile-portrait-390x844") {
    const ratio = box.height / viewport.height;
    if (ratio > 0.4) {
      await pushViolation(page, {
        page: "captions",
        viewport: viewport.id,
        rule: "R-004-mobile-height",
        severity: "critical",
        selector: '[data-testid="caption-preview-pane"]',
        expected: { maxHeightRatio: 0.4 },
        actual: { heightRatio: ratio, heightPx: box.height },
        screenshotSelector: '[data-testid="caption-preview-pane"]',
      });
    }
  }
}

async function checkR004NarrowViewport(page: Page): Promise<void> {
  await page.setViewportSize({ width: 320, height: 568 });
  await page.waitForTimeout(200);
  const scrollWidth = await page.evaluate(
    () => document.documentElement.scrollWidth,
  );
  if (scrollWidth > 321) {
    await pushViolation(page, {
      page: "captions",
      viewport: "narrow-320x568",
      rule: "R-004-narrow-overflow",
      severity: "critical",
      selector: "html",
      expected: { maxScrollWidth: 321 },
      actual: { scrollWidth },
      screenshotSelector: "html",
    });
  }
}

async function checkR005Contract(
  page: Page,
  routeId: string,
  viewportId: string,
): Promise<void> {
  const rows = page.locator('[data-testid="setting-row"]');
  const rowCount = await rows.count();
  for (let i = 0; i < rowCount; i++) {
    const row = rows.nth(i);
    if (!(await row.isVisible().catch(() => false))) continue;

    const info = await row.evaluate((el) => {
      const label = el.querySelector('[data-setting-role="label"]');
      const desc = el.querySelector('[data-setting-role="description"]');
      const ranges = el.querySelectorAll('input[type="range"]');
      const numberInputs = el.querySelectorAll('input[type="number"]');
      const editables = el.querySelectorAll('[contenteditable="true"]');
      return {
        labelText: (label?.textContent || "").trim(),
        descText: desc ? (desc.textContent || "").trim() : null,
        descPresent: !!desc,
        rangeCount: ranges.length,
        numberCount: numberInputs.length,
        editableCount: editables.length,
      };
    });

    // AC-005-a label text
    if (info.labelText) {
      const screaming = /^[A-Z][A-Z0-9_-]+$/.test(info.labelText);
      const camel = /^[a-z][a-zA-Z0-9]*$/.test(info.labelText);
      if (screaming || camel) {
        await pushViolation(page, {
          page: routeId,
          viewport: viewportId,
          rule: "R-005-label-text",
          severity: "major",
          selector: `[data-testid="setting-row"]:nth-of-type(${i + 1}) [data-setting-role="label"]`,
          expected: { humanReadable: true },
          actual: { labelText: info.labelText },
          screenshotSelector: '[data-testid="setting-row"]',
        });
      }
    }

    // AC-005-b missing description
    if (!info.descPresent || !info.descText) {
      await pushViolation(page, {
        page: routeId,
        viewport: viewportId,
        rule: "R-005-missing-description",
        severity: "major",
        selector: `[data-testid="setting-row"]:nth-of-type(${i + 1})`,
        expected: { descriptionPresent: true, descriptionNonEmpty: true },
        actual: {
          descriptionPresent: info.descPresent,
          descriptionText: info.descText,
        },
        screenshotSelector: '[data-testid="setting-row"]',
      });
    }

    // AC-005-c range+editable
    if (info.rangeCount > 0 && info.numberCount === 0 && info.editableCount === 0) {
      await pushViolation(page, {
        page: routeId,
        viewport: viewportId,
        rule: "R-005-range-editable",
        severity: "major",
        selector: `[data-testid="setting-row"]:nth-of-type(${i + 1}) input[type="range"]`,
        expected: { hasNumberInputOrContenteditable: true },
        actual: {
          numberInputs: info.numberCount,
          contenteditable: info.editableCount,
        },
        screenshotSelector: '[data-testid="setting-row"]',
      });
    }
  }

  // AC-005-d color audit — walk text nodes under settings-outer
  const colorHits = await page.evaluate(() => {
    function parseRgb(s: string): [number, number, number, number] | null {
      const m = s.match(
        /^rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*(?:,\s*([\d.]+)\s*)?\)$/,
      );
      if (!m) return null;
      return [
        parseFloat(m[1]),
        parseFloat(m[2]),
        parseFloat(m[3]),
        m[4] !== undefined ? parseFloat(m[4]) : 1,
      ];
    }
    function effBg(el: Element | null): [number, number, number] | null {
      let cur: Element | null = el;
      while (cur) {
        const rgba = parseRgb(getComputedStyle(cur).backgroundColor || "");
        if (rgba && rgba[3] > 0.05) return [rgba[0], rgba[1], rgba[2]];
        cur = cur.parentElement;
      }
      return null;
    }
    function toHsl(r: number, g: number, b: number): [number, number, number] {
      r /= 255;
      g /= 255;
      b /= 255;
      const max = Math.max(r, g, b);
      const min = Math.min(r, g, b);
      let h = 0;
      let s = 0;
      const l = (max + min) / 2;
      if (max !== min) {
        const d = max - min;
        s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
        if (max === r) h = (g - b) / d + (g < b ? 6 : 0);
        else if (max === g) h = (b - r) / d + 2;
        else h = (r - g) / d + 4;
        h *= 60;
      }
      return [h, s, l];
    }

    const root = document.querySelector('[data-testid="settings-outer"]');
    if (!root) return [];
    const hits: Array<{
      kind: "red-on-dark" | "light-grey-on-white";
      text: string;
      colorHSL: [number, number, number];
      bgHSL: [number, number, number];
      tag: string;
    }> = [];
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT);
    let node: Node | null = walker.currentNode;
    let seen = 0;
    while (node && seen < 1500) {
      seen++;
      const el = node as Element;
      const text = (el.textContent || "").trim();
      // Only direct-text elements (no child elements) to avoid bg double-count.
      const hasChildElement = Array.from(el.children).length > 0;
      if (text && !hasChildElement) {
        const cs = getComputedStyle(el);
        const c = parseRgb(cs.color || "");
        const bg = effBg(el);
        if (c && bg) {
          const [ch, cs2, cl] = toHsl(c[0], c[1], c[2]);
          const [, , bl] = toHsl(bg[0], bg[1], bg[2]);
          const redHue = (ch >= 350 && ch <= 360) || (ch >= 0 && ch <= 20);
          if (redHue && cs2 >= 0.5 && cl <= 0.55 && bl <= 0.25) {
            hits.push({
              kind: "red-on-dark",
              text: text.slice(0, 60),
              colorHSL: [ch, cs2, cl],
              bgHSL: [0, 0, bl],
              tag: el.tagName.toLowerCase(),
            });
          } else if (cl >= 0.8 && bl >= 0.9) {
            hits.push({
              kind: "light-grey-on-white",
              text: text.slice(0, 60),
              colorHSL: [ch, cs2, cl],
              bgHSL: [0, 0, bl],
              tag: el.tagName.toLowerCase(),
            });
          }
        }
      }
      node = walker.nextNode();
    }
    return hits;
  });

  for (const hit of colorHits) {
    await pushViolation(page, {
      page: routeId,
      viewport: viewportId,
      rule: `R-005-color-${hit.kind}`,
      severity: "major",
      selector: `[data-testid="settings-outer"] ${hit.tag}`,
      expected: { readableContrast: true },
      actual: {
        text: hit.text,
        colorHSL: hit.colorHSL,
        bgHSL: hit.bgHSL,
      },
      screenshotSelector: '[data-testid="settings-outer"]',
    });
  }
}

// ----- Spec -----------------------------------------------------------------

test.describe.serial("Settings UI consistency audit", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page);
  });

  for (const viewport of VIEWPORTS) {
    test(`audit @ ${viewport.id}`, async ({ page }) => {
      test.setTimeout(60_000);
      await page.setViewportSize({
        width: viewport.width,
        height: viewport.height,
      });

      let baseline: { paddingInline: number; maxWidth: number } | null = null;

      for (const route of ROUTES) {
        await navigateToRoute(page, route);

        // R-002 outer padding — About first call seeds the baseline.
        const metrics = await checkR002OuterPadding(
          page,
          route.id,
          viewport.id,
          route.id === "about" ? null : baseline,
        );
        if (route.id === "about" && metrics) baseline = metrics;

        // R-003 two-column
        await checkR003TwoColumn(
          page,
          route.id,
          viewport.id,
          viewport.id.startsWith("desktop"),
        );

        // R-004 preview clamp — captions tab only
        if (route.id === "captions") {
          await checkR004PreviewClamp(page, viewport);
        }

        // R-005 contract
        await checkR005Contract(page, route.id, viewport.id);
      }

      // R-004 narrow-viewport overflow — only run during desktop test case
      // (mutates viewport; restore after)
      if (viewport.id === "desktop-1280x800") {
        const captionsRoute = ROUTES.find((r) => r.id === "captions")!;
        await page.setViewportSize({ width: 320, height: 568 });
        await navigateToRoute(page, captionsRoute);
        await checkR004NarrowViewport(page);
      }
    });
  }

  test("emit report and assert zero criticals", async () => {
    // Sort deterministically: page, severity, rule, selector
    const severityOrder: Record<Severity, number> = {
      critical: 0,
      major: 1,
      minor: 2,
    };
    violations.sort((a, b) => {
      if (a.page !== b.page) return a.page.localeCompare(b.page);
      if (a.severity !== b.severity)
        return severityOrder[a.severity] - severityOrder[b.severity];
      if (a.rule !== b.rule) return a.rule.localeCompare(b.rule);
      return a.selector.localeCompare(b.selector);
    });

    ensureDir(OUTPUT_DIR);
    const raw = {
      schemaVersion: 1,
      generatedAt: new Date().toISOString(),
      violations,
    };
    fs.writeFileSync(
      path.join(OUTPUT_DIR, "raw.json"),
      JSON.stringify(raw, null, 2),
      "utf8",
    );

    const criticalCount = violations.filter((v) => v.severity === "critical")
      .length;
    expect(criticalCount).toBe(0);
  });
});
