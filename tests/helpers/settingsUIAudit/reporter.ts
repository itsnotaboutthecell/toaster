/**
 * Violation accumulator + screenshot/report writer for the settings UI
 * audit spec. Extracted from tests/settingsUIAudit.spec.ts so the spec
 * stays under the 800-line file cap and so additional rule specs can
 * emit into the same report.
 */
import * as fs from "node:fs";
import * as path from "node:path";
import type { Page } from "@playwright/test";

export type Severity = "critical" | "major" | "minor";

export type Violation = {
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

/**
 * File-hint lookup so PR reviewers can jump from a violation directly
 * to the component to change. Keyed by route id (matches ROUTES.id).
 */
export const FILE_HINTS: Record<string, string> = {
  about: "src/components/settings/about/AboutSettings.tsx",
  models: "src/components/settings/models/ModelsSettings.tsx",
  "post-process":
    "src/components/settings/post-processing/PostProcessingSettings.tsx",
  advanced: "src/components/settings/advanced/AdvancedSettings.tsx",
  captions: "src/components/settings/advanced/AdvancedSettings.tsx",
};

export const OUTPUT_DIR =
  process.env.AUDIT_OUTPUT_DIR ||
  path.join(process.cwd(), "test-results", "settings-ui-audit");
export const SCREENSHOT_DIR = path.join(OUTPUT_DIR, "screenshots");

export function ensureDir(p: string): void {
  fs.mkdirSync(p, { recursive: true });
}

const violations: Violation[] = [];
let screenshotCounter = 0;

export function getViolations(): Violation[] {
  return violations;
}

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

export async function pushViolation(
  page: Page,
  v: Omit<Violation, "screenshotPath" | "fileHint"> & {
    screenshotSelector?: string | null;
  },
): Promise<void> {
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

export function writeRawReport(): void {
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
}
