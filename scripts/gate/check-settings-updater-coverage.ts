#!/usr/bin/env bun
/**
 * Design-system gate — live-preview fan-out coverage.
 *
 * Invariant: every `Settings[K]` key in the live-preview domain
 * (`caption_*`, `export_*`, `loudness_*`, `normalize_audio_*`) that a
 * component writes through `updateSetting("<key>", …)` must have a
 * matching entry in the `settingUpdaters` map in
 * `src/stores/settingsStore.ts`.
 *
 * A missing entry produces the silent-no-op class of bug — the store's
 * optimistic update flips the local Zustand slice, but no backend
 * command fires, so persistence and the real preview surface stay
 * stale. Round-3 QC caught this for `caption_profiles`; scanning the
 * repo shows `export_format` has the same shape (tracked separately
 * and allowlisted below). This gate catches future regressions at
 * lint time.
 *
 * Scope: ONLY the four prefixes above. Other `AppSettings` fields
 * (`selected_model`, `bindings`, `ui_expert_mode_enabled`,
 * `experimental_enabled`, `model_unload_timeout`, …) are intentionally
 * out of scope — they follow different persistence pathways or are
 * deliberately client-only. See the "silent keys" exclusions in
 * `settingsStore.ts > updateSetting` (line ~278).
 *
 * Invocation:
 *   bun scripts/gate/check-settings-updater-coverage.ts          # report
 *   bun scripts/gate/check-settings-updater-coverage.ts --strict # CI, exit 1
 *
 * Exit codes: 0 clean, 1 drift, 2 internal error.
 */

import { readdir, readFile } from "node:fs/promises";
import { join, relative } from "node:path";

const ROOT = process.cwd();
const SRC = join(ROOT, "src");
const STORE = join(SRC, "stores", "settingsStore.ts");
const STRICT = process.argv.includes("--strict");

// Prefixes this gate cares about. See docs/design-system.md §8.
const LIVE_PREVIEW_PREFIXES = [
  "caption_",
  "export_",
  "loudness_",
  "normalize_audio_",
];

// Keys intentionally written through updateSetting() without an updater
// entry. Each entry must carry a reason; prefer fixing by adding a
// Tauri command + settingUpdaters entry over growing this list.
const ALLOWLIST: Record<string, string> = {
  export_format:
    "Tracked in features/edit-export-format-override/; needs backend command.",
};

type CallSite = { file: string; line: number; key: string };

async function* walk(dir: string): AsyncGenerator<string> {
  const entries = await readdir(dir, { withFileTypes: true });
  for (const e of entries) {
    const p = join(dir, e.name);
    if (e.isDirectory()) {
      if (e.name === "node_modules" || e.name.startsWith(".")) continue;
      yield* walk(p);
    } else if (/\.tsx?$/.test(e.name)) {
      yield p;
    }
  }
}

async function parseUpdaterKeys(): Promise<Set<string>> {
  const src = await readFile(STORE, "utf8");
  const mapStart = src.indexOf("const settingUpdaters");
  if (mapStart < 0) throw new Error("settingUpdaters map not found in settingsStore.ts");
  // Skip the type annotation `: { ... }` and anchor on the `= {` that
  // opens the actual object literal.
  const eqMatch = src.slice(mapStart).match(/=\s*\{/);
  if (!eqMatch || eqMatch.index === undefined) {
    throw new Error("Could not locate `= {` after settingUpdaters declaration");
  }
  const objOpen = mapStart + eqMatch.index + eqMatch[0].lastIndexOf("{");
  let depth = 0;
  let end = -1;
  for (let i = objOpen; i < src.length; i++) {
    const c = src[i];
    if (c === "{") depth++;
    else if (c === "}") {
      depth--;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  if (end < 0) throw new Error("Could not locate end of settingUpdaters map");
  const body = src.slice(objOpen + 1, end);
  const keys = new Set<string>();
  // Capture top-level keys (two-space indent). Nested object braces
  // deepen depth so we track it and only accept at depth 0.
  let d = 0;
  for (const line of body.split("\n")) {
    const opens = (line.match(/\{/g) ?? []).length;
    const closes = (line.match(/\}/g) ?? []).length;
    if (d === 0) {
      const m = /^\s{2}([a-z_][a-z0-9_]*)\s*:/.exec(line);
      if (m) keys.add(m[1]);
    }
    d += opens - closes;
  }
  return keys;
}

async function findCallSites(): Promise<CallSite[]> {
  const hits: CallSite[] = [];
  // Matches updateSetting("key", …)   and   updateSetting('key', …)
  const re = /\bupdateSetting\s*\(\s*["']([a-z_][a-z0-9_]*)["']/g;
  for await (const file of walk(SRC)) {
    if (file === STORE) continue; // the map itself references keys as identifiers
    const text = await readFile(file, "utf8");
    const lines = text.split("\n");
    lines.forEach((line, idx) => {
      let m: RegExpExecArray | null;
      re.lastIndex = 0;
      while ((m = re.exec(line)) !== null) {
        hits.push({
          file: relative(ROOT, file).replace(/\\/g, "/"),
          line: idx + 1,
          key: m[1],
        });
      }
    });
  }
  return hits;
}

async function main() {
  try {
    const [keys, sites] = await Promise.all([parseUpdaterKeys(), findCallSites()]);
    if (keys.size === 0) {
      console.error("[settings-updater-coverage] parsed zero keys from settingUpdaters — parser bug?");
      process.exit(2);
    }

    const missing = new Map<string, CallSite[]>();
    for (const s of sites) {
      // Only enforce on the live-preview prefixes; everything else is
      // out of scope for this gate (see the scope note in the header).
      if (!LIVE_PREVIEW_PREFIXES.some((p) => s.key.startsWith(p))) continue;
      if (keys.has(s.key)) continue;
      if (ALLOWLIST[s.key]) continue;
      const arr = missing.get(s.key) ?? [];
      arr.push(s);
      missing.set(s.key, arr);
    }

    if (missing.size === 0) {
      console.log(
        `[settings-updater-coverage] OK — ${keys.size} updaters cover ${new Set(sites.map((s) => s.key)).size} distinct updateSetting() keys (${sites.length} call sites). Allowlisted: ${Object.keys(ALLOWLIST).length}.`,
      );
      process.exit(0);
    }

    console.error(
      "[settings-updater-coverage] FAIL — updateSetting() call sites for keys NOT in settingUpdaters:",
    );
    for (const [key, callers] of missing.entries()) {
      console.error(`  ${key}`);
      for (const c of callers) console.error(`    ${c.file}:${c.line}`);
    }
    console.error(
      "\nFix: add an entry to settingUpdaters in src/stores/settingsStore.ts that calls the appropriate backend command. See docs/design-system.md §8 (live-preview fan-out contract).",
    );
    process.exit(STRICT ? 1 : 0);
  } catch (err) {
    console.error("[settings-updater-coverage] internal error:", err);
    process.exit(2);
  }
}

main();
