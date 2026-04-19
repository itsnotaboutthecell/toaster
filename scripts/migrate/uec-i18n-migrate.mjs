#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const localesDir = path.resolve("src/i18n/locales");
const locales = fs
  .readdirSync(localesDir, { withFileTypes: true })
  .filter((d) => d.isDirectory())
  .map((d) => d.name);

const NEW_EXPERIMENTAL = {
  title: "Experimental",
  banner:
    "These features are under active development and may change or be removed.",
};
const NEW_EXPERIMENTS = {
  simplifyMode: {
    label: "Simplify Mode",
    description:
      "Reduce visual density and hide secondary controls in the editor.",
  },
  feedbackLink: "Send feedback",
};
const NEW_SIDEBAR_EXPERIMENTAL = "Experimental";

function load(locale) {
  const p = path.join(localesDir, locale, "translation.json");
  return { p, json: JSON.parse(fs.readFileSync(p, "utf8")) };
}

function save(p, json) {
  fs.writeFileSync(p, JSON.stringify(json, null, 2) + "\n", "utf8");
}

for (const locale of locales) {
  const { p, json } = load(locale);

  // 5a: experimental sidebar entry + settings block + experiments block
  if (json.sidebar && !json.sidebar.experimental) {
    json.sidebar.experimental = NEW_SIDEBAR_EXPERIMENTAL;
  }
  json.settings = json.settings || {};
  json.settings.experimental = {
    title: json.settings.experimental?.title ?? NEW_EXPERIMENTAL.title,
    banner: json.settings.experimental?.banner ?? NEW_EXPERIMENTAL.banner,
  };
  json.experiments = {
    simplifyMode: {
      label:
        json.experiments?.simplifyMode?.label ??
        NEW_EXPERIMENTS.simplifyMode.label,
      description:
        json.experiments?.simplifyMode?.description ??
        NEW_EXPERIMENTS.simplifyMode.description,
    },
    feedbackLink:
      json.experiments?.feedbackLink ?? NEW_EXPERIMENTS.feedbackLink,
  };

  // 5c: remove appleIntelligence keys
  const api = json.settings?.postProcessing?.api;
  if (api) {
    delete api.appleIntelligence;
    if (api.model) {
      delete api.model.descriptionApple;
      delete api.model.placeholderApple;
    }
  }

  // Final orphan purge
  if (json.sidebar) delete json.sidebar.general;
  delete json.overlay;
  if (json.settings?.debug) delete json.settings.debug.clamshellMicrophone;

  save(p, json);
  console.log(`updated ${locale}`);
}
