#!/usr/bin/env bun
/**
 * File-size guardrail.
 *
 * Enforces an 800-line cap on .rs / .ts / .tsx files under src/ and
 * src-tauri/src/ so monolithic modules cannot silently creep back into
 * the repo. Files explicitly listed in scripts/file-size-allowlist.txt
 * are grandfathered in while the monolith-split plan lands; each phase
 * of that plan removes its entry from the allowlist.
 *
 * Generated files (src/bindings.ts) and vendored directories
 * (node_modules, target, dist) are excluded by construction.
 *
 * Exit codes:
 *   0 — no violations, and no stale allowlist entries
 *   1 — at least one file exceeds the cap without being allowlisted,
 *       OR an allowlisted file is now under the cap (stale entry).
 */
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "..");

const MAX_LINES = 800;
const SEARCH_ROOTS = ["src", path.join("src-tauri", "src")];
const EXTENSIONS = new Set([".rs", ".ts", ".tsx"]);
const EXCLUDE_DIRS = new Set([
  "node_modules",
  "target",
  "dist",
  ".git",
  ".nix",
]);
const EXCLUDE_FILES = new Set([
  path.join("src", "bindings.ts").replaceAll("\\", "/"),
]);
const ALLOWLIST_PATH = path.join(__dirname, "file-size-allowlist.txt");

const colors = {
  reset: "\x1b[0m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  cyan: "\x1b[36m",
  bold: "\x1b[1m",
};

function toPosix(p: string): string {
  return p.replaceAll("\\", "/");
}

function walk(dir: string, out: string[]): void {
  let entries: fs.Dirent[];
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    if (EXCLUDE_DIRS.has(entry.name)) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(full, out);
    } else if (entry.isFile()) {
      const ext = path.extname(entry.name);
      if (EXTENSIONS.has(ext)) out.push(full);
    }
  }
}

function countLines(absPath: string): number {
  const buf = fs.readFileSync(absPath);
  if (buf.length === 0) return 0;
  let count = 0;
  for (let i = 0; i < buf.length; i++) {
    if (buf[i] === 0x0a) count++;
  }
  // Trailing content without a newline still counts as a line.
  if (buf[buf.length - 1] !== 0x0a) count++;
  return count;
}

function loadAllowlist(): Set<string> {
  if (!fs.existsSync(ALLOWLIST_PATH)) return new Set();
  const raw = fs.readFileSync(ALLOWLIST_PATH, "utf8");
  return new Set(
    raw
      .split(/\r?\n/)
      .map((line) => line.replace(/#.*$/, "").trim())
      .filter((line) => line.length > 0)
      .map(toPosix),
  );
}

function main(): void {
  const files: string[] = [];
  for (const root of SEARCH_ROOTS) {
    walk(path.join(REPO_ROOT, root), files);
  }

  const allowlist = loadAllowlist();
  const violations: Array<{ rel: string; lines: number }> = [];
  const allowlistHits = new Set<string>();

  for (const abs of files) {
    const rel = toPosix(path.relative(REPO_ROOT, abs));
    if (EXCLUDE_FILES.has(rel)) continue;
    const lines = countLines(abs);
    if (lines <= MAX_LINES) continue;
    if (allowlist.has(rel)) {
      allowlistHits.add(rel);
      continue;
    }
    violations.push({ rel, lines });
  }

  const staleAllowlist = [...allowlist].filter(
    (entry) => !allowlistHits.has(entry),
  );

  violations.sort((a, b) => b.lines - a.lines);

  if (violations.length === 0 && staleAllowlist.length === 0) {
    console.log(
      `${colors.green}✓ File-size check passed${colors.reset} ` +
        `(cap ${MAX_LINES} lines, ${files.length} files scanned, ` +
        `${allowlist.size} allowlisted).`,
    );
    process.exit(0);
  }

  if (violations.length > 0) {
    console.error(
      `${colors.red}${colors.bold}✗ File-size violations (cap ${MAX_LINES} lines):${colors.reset}`,
    );
    for (const { rel, lines } of violations) {
      console.error(
        `  ${colors.red}${lines.toString().padStart(5)} lines${colors.reset}  ${rel}`,
      );
    }
    console.error(
      `\n${colors.yellow}Split the file into cohesive submodules (see plan.md / AGENTS.md).` +
        `\nIf the file is genuinely indivisible, add it to scripts/file-size-allowlist.txt` +
        `\nwith a comment linking the tracking issue.${colors.reset}`,
    );
  }

  if (staleAllowlist.length > 0) {
    console.error(
      `${colors.yellow}${colors.bold}⚠ Stale allowlist entries (now under ${MAX_LINES} lines — remove them):${colors.reset}`,
    );
    for (const entry of staleAllowlist.sort()) {
      console.error(`  ${entry}`);
    }
  }

  process.exit(1);
}

main();
