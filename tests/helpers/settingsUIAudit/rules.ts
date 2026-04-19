/**
 * Rule checks (R-002..R-005) for the settings UI audit spec. Extracted
 * from tests/settingsUIAudit.spec.ts to keep the spec under the 800-line
 * file cap and to leave headroom for R-006 (double-label) and R-008
 * (duplicate-description) rules being added in a follow-up.
 */
import type { Page } from "@playwright/test";
import { pushViolation } from "./reporter";

export type Route = {
  id: string;
  label: string;
  expectMaxW5xl: boolean;
  captionTab?: string;
};

export type Viewport = {
  id: string;
  width: number;
  height: number;
};

export const ROUTES: readonly Route[] = [
  { id: "about", label: "About", expectMaxW5xl: true },
  { id: "models", label: "Models", expectMaxW5xl: true },
  { id: "post-process", label: "Post Process", expectMaxW5xl: true },
  { id: "advanced", label: "Advanced", expectMaxW5xl: true },
  {
    id: "captions",
    label: "Advanced",
    expectMaxW5xl: true,
    captionTab: "Desktop",
  },
];

export const VIEWPORTS: readonly Viewport[] = [
  { id: "desktop-1280x800", width: 1280, height: 800 },
  { id: "mobile-portrait-390x844", width: 390, height: 844 },
];

export async function navigateToRoute(
  page: Page,
  route: Route,
): Promise<void> {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");
  const nav = page.getByText(route.label, { exact: true }).first();
  await nav.click();
  await page
    .locator('[data-testid="settings-outer"]')
    .first()
    .waitFor({ state: "attached", timeout: 5000 })
    .catch(() => undefined);

  if (route.captionTab) {
    const tab = page.locator(`button[role="tab"]`, { hasText: route.captionTab });
    if ((await tab.count()) > 0) {
      await tab.first().click().catch(() => undefined);
      await page.waitForTimeout(150);
    }
  }
}

export async function checkR002OuterPadding(
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

export async function checkR003TwoColumn(
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

export async function checkR004PreviewClamp(
  page: Page,
  viewport: Viewport,
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

export async function checkR004NarrowViewport(page: Page): Promise<void> {
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

export async function checkR005Contract(
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

/**
 * R-006 double-label: a SettingContainer must not repeat (case-insensitive)
 * the title of its enclosing SettingsGroup. Catches drift like the
 * "Post Processing" group containing a "Post Processing" container
 * (resolved in this sprint's Post-Processing phase).
 */
export async function checkR006DoubleLabel(
  page: Page,
  routeId: string,
  viewportId: string,
): Promise<void> {
  const hits = await page.evaluate(() => {
    const out: Array<{ groupTitle: string; containerTitle: string; groupIndex: number }> = [];
    const groups = document.querySelectorAll('[data-testid="settings-group"]');
    groups.forEach((group, gi) => {
      const groupTitleEl = group.querySelector('[data-setting-role="group-title"]');
      const groupTitle = (groupTitleEl?.textContent || "").trim().toLowerCase();
      if (!groupTitle) return;
      const containers = group.querySelectorAll('[data-setting-role="label"]');
      containers.forEach((labelEl) => {
        const labelText = (labelEl.textContent || "").trim().toLowerCase();
        if (labelText && labelText === groupTitle) {
          out.push({
            groupTitle: groupTitle,
            containerTitle: labelText,
            groupIndex: gi,
          });
        }
      });
    });
    return out;
  });

  for (const hit of hits) {
    await pushViolation(page, {
      page: routeId,
      viewport: viewportId,
      rule: "R-006-double-label",
      severity: "major",
      selector: `[data-testid="settings-group"]:nth-of-type(${hit.groupIndex + 1})`,
      expected: { containerTitleDistinctFromGroupTitle: true },
      actual: { groupTitle: hit.groupTitle, containerTitle: hit.containerTitle },
      screenshotSelector: '[data-testid="settings-group"]',
    });
  }
}

/**
 * R-008 duplicate-description: two SettingContainers under the same
 * SettingsGroup must not share identical description text. Catches
 * drift like the "experimental_simplify_mode" pair that showed the
 * same description twice.
 */
export async function checkR008DuplicateDescription(
  page: Page,
  routeId: string,
  viewportId: string,
): Promise<void> {
  const hits = await page.evaluate(() => {
    const out: Array<{ description: string; groupIndex: number; count: number }> = [];
    const groups = document.querySelectorAll('[data-testid="settings-group"]');
    groups.forEach((group, gi) => {
      const seen = new Map<string, number>();
      const descs = group.querySelectorAll('[data-setting-role="description"]');
      descs.forEach((el) => {
        const text = (el.textContent || "").trim();
        if (!text) return;
        seen.set(text, (seen.get(text) ?? 0) + 1);
      });
      for (const [text, count] of seen.entries()) {
        if (count > 1) {
          out.push({ description: text, groupIndex: gi, count });
        }
      }
    });
    return out;
  });

  for (const hit of hits) {
    await pushViolation(page, {
      page: routeId,
      viewport: viewportId,
      rule: "R-008-duplicate-description",
      severity: "major",
      selector: `[data-testid="settings-group"]:nth-of-type(${hit.groupIndex + 1})`,
      expected: { descriptionsUniqueWithinGroup: true },
      actual: { duplicate: hit.description, occurrences: hit.count },
      screenshotSelector: '[data-testid="settings-group"]',
    });
  }
}
