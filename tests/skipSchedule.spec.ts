import { test, expect } from "@playwright/test";

/**
 * Unit-style tests for the pure `computeNextDeletedSkip` helper exported from
 * `src/components/player/MediaPlayer.tsx`.
 *
 * The helper is imported via Vite's dev server (which resolves the `@/`
 * aliases and compiles TSX), then invoked in the browser page context. This
 * exercises the exact module the production component consumes — no
 * re-implementation or stub.
 *
 * Covers p0-skip-mode-bleed scheduling semantics:
 *  - Returns null when no future deletions remain.
 *  - Returns delay=0 when currentTime is inside a range (skip immediately).
 *  - Delay scales with playbackRate.
 *  - Unsorted input still selects the earliest-starting future range.
 *  - Back-to-back short deletions each produce their own scheduled skip
 *    (no 35 ms debounce squashing them).
 */

type Range = { start: number; end: number };
type SkipResult = { range: Range; delayMs: number } | null;

async function loadHelper(page: import("@playwright/test").Page) {
  await page.goto("/");
  return async (
    currentTime: number,
    ranges: Range[],
    playbackRate: number,
  ): Promise<SkipResult> => {
    return page.evaluate(
      async ({ currentTime, ranges, playbackRate }) => {
        const mod = await import("@/components/player/MediaPlayer");
        const result = mod.computeNextDeletedSkip(currentTime, ranges, playbackRate);
        return result as SkipResult;
      },
      { currentTime, ranges, playbackRate },
    );
  };
}

test.describe("computeNextDeletedSkip", () => {
  test("returns null when all ranges are behind currentTime", async ({ page }) => {
    const compute = await loadHelper(page);
    const result = await compute(10, [{ start: 1, end: 2 }, { start: 3, end: 4 }], 1);
    expect(result).toBeNull();
  });

  test("returns delay=0 when currentTime is inside a range", async ({ page }) => {
    const compute = await loadHelper(page);
    const result = await compute(1.5, [{ start: 1, end: 2 }], 1);
    expect(result).not.toBeNull();
    expect(result!.range).toEqual({ start: 1, end: 2 });
    expect(result!.delayMs).toBe(0);
  });

  test("delay reflects distance to next range start at 1x rate", async ({ page }) => {
    const compute = await loadHelper(page);
    const result = await compute(1.0, [{ start: 2.0, end: 2.5 }], 1);
    expect(result).not.toBeNull();
    expect(result!.delayMs).toBeCloseTo(1000, 1);
  });

  test("delay halves at 2x playbackRate", async ({ page }) => {
    const compute = await loadHelper(page);
    const result = await compute(1.0, [{ start: 2.0, end: 2.5 }], 2);
    expect(result).not.toBeNull();
    expect(result!.delayMs).toBeCloseTo(500, 1);
  });

  test("picks earliest-starting future range even if input is unsorted", async ({ page }) => {
    const compute = await loadHelper(page);
    const result = await compute(
      0,
      [
        { start: 5, end: 6 },
        { start: 1, end: 2 },
        { start: 3, end: 4 },
      ],
      1,
    );
    expect(result).not.toBeNull();
    expect(result!.range.start).toBe(1);
    expect(result!.delayMs).toBeCloseTo(1000, 1);
  });

  test("returns null for non-positive playbackRate", async ({ page }) => {
    const compute = await loadHelper(page);
    expect(await compute(0, [{ start: 1, end: 2 }], 0)).toBeNull();
    expect(await compute(0, [{ start: 1, end: 2 }], -1)).toBeNull();
  });

  test("back-to-back short deletions produce independent scheduled skips", async ({
    page,
  }) => {
    // Three ~10 ms deleted words, 5 ms apart. Simulate scheduling loop:
    //   start -> first range -> seek past -> recompute -> next range -> ...
    // Previously the 35 ms RAF debounce could squash these together. The new
    // scheduler must fire all three.
    const compute = await loadHelper(page);
    const epsilon = 1 / 48000;
    const ranges: Range[] = [
      { start: 1.000, end: 1.010 },
      { start: 1.015, end: 1.025 },
      { start: 1.030, end: 1.040 },
    ];
    let t = 0.95;
    const fires: number[] = [];
    for (let i = 0; i < 5 && fires.length < ranges.length; i++) {
      const next = await compute(t, ranges, 1);
      if (!next) break;
      fires.push(next.range.start);
      t = next.range.end + epsilon; // simulate the seek past the inclusive end
    }
    expect(fires).toEqual([1.000, 1.015, 1.030]);
  });
});
