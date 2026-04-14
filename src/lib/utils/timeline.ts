/**
 * Dual-track timeline mapping utilities.
 *
 * Pure functions extracted from MediaPlayer.tsx so they can be exercised by
 * unit tests independently of React rendering.
 *
 * ── Surrogate coverage note ──────────────────────────────────────────────────
 * The Toaster frontend does not yet have a vitest/jest harness.  The logic
 * here is therefore validated via two complementary strategies:
 *
 *  1. **Rust mirror tests** — `src-tauri/src/managers/editor.rs` contains a
 *     `dual_track_regression` test module that exercises the identical
 *     algorithms (keep-segment computation, edit→source mapping, boundary
 *     clamping, monotonicity) in the backend where `cargo test` runs in CI.
 *
 *  2. **TypeScript type-checking** — `npx tsc --noEmit` catches every
 *     incorrect call-site in MediaPlayer.tsx via the exported types below.
 *
 * When a frontend unit-test runner (vitest) is added, add tests directly
 * against these exports:
 *
 *   import { editTimeToSourceTime, sourceTimeToEditTime, … } from
 *     "@/lib/utils/timeline";
 *
 *   describe("editTimeToSourceTime", () => { … });
 * ─────────────────────────────────────────────────────────────────────────────
 */

import type { KeepSegment } from "@/bindings";
import type { Word } from "@/stores/editorStore";

// ── Constants ────────────────────────────────────────────────────────────────

/** Minimum A/V drift (seconds) before the video element is resynchronised. */
export const DUAL_TRACK_DRIFT_THRESHOLD = 0.08; // 80 ms

/** Minimum real-clock interval (ms) between consecutive drift corrections. */
export const DUAL_TRACK_SYNC_COOLDOWN_MS = 250;

// ── Keep-segment helpers ──────────────────────────────────────────────────────

/** A half-open time interval in seconds [start, end). */
export interface TimeSegment {
  start: number;
  end: number;
}

/**
 * Build sorted list of deleted time ranges from a word array, with a small
 * crossfade pad to prevent clicks/pops at edit boundaries.
 *
 * Regression guard: adjacent deleted words closer than 50 ms are merged into
 * a single range so the skip loop never oscillates between two ranges.
 */
export function getDeletedRanges(words: Word[], duration: number): TimeSegment[] {
  const CROSSFADE_PAD = 0.01; // 10 ms
  const MIN_RANGE_DURATION = 0.001; // 1 ms
  const maxDuration =
    Number.isFinite(duration) && duration > 0 ? duration : Number.POSITIVE_INFINITY;

  const ranges: TimeSegment[] = [];
  let rangeStart: number | null = null;
  let rangeEnd = 0;

  const pushRange = (start: number, end: number) => {
    const clampedStart = Math.min(maxDuration, Math.max(0, start));
    const clampedEnd = Math.min(maxDuration, Math.max(0, end));
    if (clampedEnd - clampedStart >= MIN_RANGE_DURATION) {
      ranges.push({ start: clampedStart, end: clampedEnd });
    }
  };

  for (const w of words) {
    if (w.deleted) {
      const startSec = w.start_us / 1_000_000;
      const endSec = w.end_us / 1_000_000;
      if (rangeStart === null) {
        rangeStart = startSec;
        rangeEnd = endSec;
      } else if (startSec <= rangeEnd + 0.05) {
        // Merge: gap is small enough to bridge
        rangeEnd = Math.max(rangeEnd, endSec);
      } else {
        pushRange(rangeStart - CROSSFADE_PAD, rangeEnd + CROSSFADE_PAD);
        rangeStart = startSec;
        rangeEnd = endSec;
      }
    } else {
      if (rangeStart !== null) {
        pushRange(rangeStart - CROSSFADE_PAD, rangeEnd + CROSSFADE_PAD);
        rangeStart = null;
      }
    }
  }
  if (rangeStart !== null) {
    pushRange(rangeStart - CROSSFADE_PAD, rangeEnd + CROSSFADE_PAD);
  }
  return ranges;
}

/**
 * Compute deleted ranges from backend keep-segments (inverse of keep = deleted).
 *
 * Regression guard: the output ranges are always bounded by the transcript
 * span [words[0].start_us, words[-1].end_us] and are non-overlapping.
 */
export function getDeletedRangesFromKeepSegments(
  words: Word[],
  keepSegments: KeepSegment[],
): TimeSegment[] {
  const MIN_RANGE_DURATION = 0.001;
  if (words.length === 0) return [];

  const transcriptStart = words[0].start_us / 1_000_000;
  const transcriptEnd = words[words.length - 1].end_us / 1_000_000;
  if (transcriptEnd - transcriptStart < MIN_RANGE_DURATION) return [];

  const normalized = [...keepSegments]
    .map((seg) => ({
      start: seg.start_us / 1_000_000,
      end: seg.end_us / 1_000_000,
    }))
    .filter((seg) => seg.end - seg.start >= MIN_RANGE_DURATION)
    .sort((a, b) => a.start - b.start);

  const ranges: TimeSegment[] = [];
  let cursor = transcriptStart;

  for (const segment of normalized) {
    const segStart = Math.max(transcriptStart, segment.start);
    const segEnd = Math.min(transcriptEnd, segment.end);
    if (segEnd - segStart < MIN_RANGE_DURATION) continue;
    if (segStart - cursor >= MIN_RANGE_DURATION) {
      ranges.push({ start: cursor, end: segStart });
    }
    cursor = Math.max(cursor, segEnd);
  }

  if (transcriptEnd - cursor >= MIN_RANGE_DURATION) {
    ranges.push({ start: cursor, end: transcriptEnd });
  }

  return ranges;
}

// ── Edit↔Source time mapping ──────────────────────────────────────────────────

/**
 * Map edit-time (preview audio timeline, deletions removed) → source-time
 * (original video position).
 *
 * `keepSegments` must be sorted ascending by `start`.
 *
 * Properties guaranteed (and tested in the Rust mirror suite):
 *  - Identity when `keepSegments` is empty.
 *  - Monotonically non-decreasing: editTimeToSourceTime(t1) ≤
 *    editTimeToSourceTime(t2)  for all t1 ≤ t2.
 *  - Clamps to the end of the last keep-segment when editTime exceeds total
 *    keep duration (no video collapse / invalid seek target).
 */
export function editTimeToSourceTime(
  editTime: number,
  keepSegments: ReadonlyArray<TimeSegment>,
): number {
  if (keepSegments.length === 0) return editTime;
  let accumulated = 0;
  for (const seg of keepSegments) {
    const segDur = seg.end - seg.start;
    if (editTime <= accumulated + segDur) {
      return seg.start + (editTime - accumulated);
    }
    accumulated += segDur;
  }
  // Clamp to end of timeline — prevents seeking past the end of the source.
  return keepSegments[keepSegments.length - 1].end;
}

/**
 * Map source-time (original video) → edit-time (preview audio timeline).
 *
 * When `sourceTime` falls inside a deleted region the function returns the
 * edit-time of the *start* of the next keep-segment boundary (snap-forward).
 *
 * Properties guaranteed (and tested in the Rust mirror suite):
 *  - Identity when `keepSegments` is empty.
 *  - Result is always ≥ 0.
 *  - Result never exceeds the total keep-segment duration.
 */
export function sourceTimeToEditTime(
  sourceTime: number,
  keepSegments: ReadonlyArray<TimeSegment>,
): number {
  if (keepSegments.length === 0) return sourceTime;
  let accumulated = 0;
  for (const seg of keepSegments) {
    if (sourceTime < seg.start) {
      // Inside a deleted region — snap to the start of this keep-segment.
      return accumulated;
    }
    if (sourceTime < seg.end) {
      return accumulated + (sourceTime - seg.start);
    }
    accumulated += seg.end - seg.start;
  }
  return accumulated;
}
