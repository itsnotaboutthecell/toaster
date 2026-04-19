/**
 * Settings UI consistency audit — executes R-001..R-005 against every
 * settings route at desktop + mobile viewports and emits a structured
 * violations report. See features/settings-ui-consistency-audit/PRD.md.
 *
 * Implementation modules live under tests/helpers/settingsUIAudit/:
 *  - tauriMock.ts — IPC mock injected before every page load
 *  - reporter.ts  — violations accumulator + screenshot capture + raw.json emit
 *  - rules.ts     — R-002..R-005 rule checks and route navigation
 *
 * Split out to keep this spec under the 800-line file cap and leave
 * headroom for R-006 (double-label) / R-008 (duplicate-description)
 * runtime rules scheduled as a follow-up.
 */
import { test, expect } from "@playwright/test";
import { setupTauriMocks } from "./helpers/settingsUIAudit/tauriMock";
import {
  getViolations,
  writeRawReport,
} from "./helpers/settingsUIAudit/reporter";
import {
  ROUTES,
  VIEWPORTS,
  navigateToRoute,
  checkR002OuterPadding,
  checkR003TwoColumn,
  checkR004PreviewClamp,
  checkR004NarrowViewport,
  checkR005Contract,
} from "./helpers/settingsUIAudit/rules";

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
    writeRawReport();
    const criticalCount = getViolations().filter(
      (v) => v.severity === "critical",
    ).length;
    expect(criticalCount).toBe(0);
  });
});
