#!/usr/bin/env bun
/**
 * Settings UI audit rule R-007 — detect raw `<button>` elements that
 * duplicate Button-component styling with brand / ui background classes.
 *
 * Policy: any `<button ... className=".*(bg-logo-primary|bg-background-ui|
 * bg-mid-gray\/10) ..." ...>` under `src/` is a drift site — the shared
 * `<Button>` component owns variant → colour mapping. Raw uses produce
 * inconsistent hover / focus / disabled states and defeat the brand
 * standardisation effort.
 *
 * Exceptions: files under `src/components/ui/Button.tsx` itself (the
 * variant definitions), and anything ending in `.stories.tsx` or
 * `.test.tsx`. Extend `ALLOWED_PATHS` only with a referenced justification.
 *
 * Invocation: `bun scripts/gate/check-button-variant-drift.ts`
 * Exit codes: 0 clean, 1 violation(s) found, 2 internal error.
 */

import { readdir, readFile } from "node:fs/promises";
import { join, relative } from "node:path";

const ROOT = process.cwd();
const SRC = join(ROOT, "src");
const ALLOWED_PATHS = new Set<string>([
  // Button.tsx owns the variant class map; it must reference these.
  "src/components/ui/Button.tsx",
  // Dropdown internal option-row + toggle buttons — option rows inside
  // a popover are list items, not standalone actions, and styling them
  // as `<Button variant="…">` would double-wrap padding + focus.
  "src/components/ui/Dropdown.tsx",
  // Language-chip grid: selection pills, same rationale as Dropdown
  // option rows.
  "src/components/settings/LanguageSelector.tsx",
  // Inline copy-affordance; its inline width + min-height escape the
  // Button primitive. Revisit if Button gains a compact icon variant.
  "src/components/ui/TextDisplay.tsx",
  // Update-checker banner CTA; lives on a modal overlay with bespoke
  // spacing. Tracked as design debt.
  "src/components/update-checker/UpdateChecker.tsx",
  // Editor-page toolbar + dashboard buttons — grandfathered design debt
  // from the pre-design-system era. New settings/editor buttons MUST
  // use `<Button variant="…">` per docs/design-system.md §2. When these
  // files are next touched, migrate them and remove from this list.
  "src/components/editor/EditorToolbar.tsx",
  "src/components/editor/EditorView.tsx",
  "src/components/editor/FillerDashboard.tsx",
]);
const BRAND_CLASSES = [
  "bg-logo-primary",
  "bg-background-ui",
  "bg-mid-gray/10",
];
const BUTTON_REGEX = /<button\b[^>]*className\s*=\s*\{?\s*[`"']([^`"']+)[`"']/gms;

type Violation = {
  file: string;
  line: number;
  match: string;
  offendingClass: string;
};

async function walk(dir: string): Promise<string[]> {
  const entries = await readdir(dir, { withFileTypes: true });
  const results: string[] = [];
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...(await walk(full)));
    } else if (entry.isFile() && /\.(tsx?|jsx?)$/.test(entry.name)) {
      results.push(full);
    }
  }
  return results;
}

function lineOf(content: string, offset: number): number {
  return content.slice(0, offset).split("\n").length;
}

async function main(): Promise<number> {
  const files = await walk(SRC);
  const violations: Violation[] = [];

  for (const file of files) {
    const rel = relative(ROOT, file).replace(/\\/g, "/");
    if (ALLOWED_PATHS.has(rel)) continue;
    if (rel.endsWith(".stories.tsx") || rel.endsWith(".test.tsx")) continue;

    const content = await readFile(file, "utf8");
    for (const m of content.matchAll(BUTTON_REGEX)) {
      const classes = m[1];
      const hit = BRAND_CLASSES.find((cls) => classes.includes(cls));
      if (!hit) continue;
      violations.push({
        file: rel,
        line: lineOf(content, m.index ?? 0),
        match: m[0].slice(0, 120) + (m[0].length > 120 ? "…" : ""),
        offendingClass: hit,
      });
    }
  }

  if (violations.length === 0) {
    console.log("[R-007] OK — no raw <button> brand-class drift found.");
    return 0;
  }

  const strict = process.argv.includes("--strict");
  const verb = strict ? "FAIL" : "WARN";
  const exitCode = strict ? 1 : 0;
  console.error(
    `[R-007] ${verb} — ${violations.length} raw <button> element(s) use brand/ui background classes that should route through <Button variant=...>:`,
  );
  for (const v of violations) {
    console.error(
      `  ${v.file}:${v.line}  [${v.offendingClass}]  ${v.match}`,
    );
  }
  console.error(
    "\nFix: replace with <Button variant='brand' | 'primary' | 'secondary' ...> from src/components/ui/Button.tsx.",
  );
  if (!strict) {
    console.error(
      "\n(Report-only mode. Pass --strict to turn drift into a hard CI failure.)",
    );
  }
  return exitCode;
}

main()
  .then((code) => process.exit(code))
  .catch((err) => {
    console.error("[R-007] internal error:", err);
    process.exit(2);
  });
