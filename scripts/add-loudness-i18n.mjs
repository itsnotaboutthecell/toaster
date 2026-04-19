// One-shot: inject loudness-export i18n keys into every locale.
// Used to bootstrap features/export-loudness; safe to re-run (idempotent).
import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const LOCALES_DIR = "src/i18n/locales";
const SIDEBAR_KEY = "export";
const SIDEBAR_VALUE = "Export";
const EXPORT_BLOCK = {
  title: "Export",
  description:
    "Settings that apply when exporting your edited media.",
  loudness: {
    title: "Loudness normalization",
    description:
      "Match a target integrated loudness so exported videos sound consistent across players and platforms.",
    options: {
      off: {
        label: "Off",
        description: "No loudness normalization. Exported audio uses the source levels.",
      },
      "podcast_-16": {
        label: "Podcast (-16 LUFS)",
        description: "Broadcast / podcast preset. Targets -16 LUFS integrated.",
      },
      "streaming_-14": {
        label: "Streaming (-14 LUFS)",
        description: "Spotify / YouTube preset. Targets -14 LUFS integrated.",
      },
    },
    preflight: {
      title: "Preflight measurement",
      description:
        "Measures the post-edit audio without exporting so you can preview how far off-target it is.",
      run: "Run preflight",
      running: "Measuring…",
      integrated: "Integrated",
      truePeak: "True peak",
      lra: "Loudness range",
      delta: "Delta to target",
      target: "Target",
      noMedia: "Open a project to run preflight.",
      noTarget: "—",
      warningOffTarget:
        "Measured loudness is more than 12 LU off the target. Consider re-recording or adjusting the source before export.",
      error: "Preflight failed: {{error}}",
    },
  },
};

for (const locale of readdirSync(LOCALES_DIR, { withFileTypes: true })) {
  if (!locale.isDirectory()) continue;
  const path = join(LOCALES_DIR, locale.name, "translation.json");
  const raw = readFileSync(path, "utf8");
  const json = JSON.parse(raw);
  json.sidebar = json.sidebar ?? {};
  json.sidebar[SIDEBAR_KEY] = SIDEBAR_VALUE;
  json.settings = json.settings ?? {};
  json.settings.export = EXPORT_BLOCK;
  writeFileSync(path, JSON.stringify(json, null, 2) + "\n", "utf8");
  console.log(`updated ${locale.name}`);
}
