# Toaster design system — single source of truth

Canonical aesthetic contract for every Toaster UI surface. Linked from
`AGENTS.md`, `src/AGENTS.md`, and `src/components/settings/AGENTS.md`. If
you're about to build or edit a settings screen, editor affordance, or
control primitive, this page is the rule you follow — and the gates
below are what CI enforces.

## Table of contents

1. [Color tokens](#1-color-tokens)
2. [Primitive → token contract](#2-primitive--token-contract)
3. [Page anatomy (hero pattern)](#3-page-anatomy-hero-pattern)
4. [SettingsGroup anatomy](#4-settingsgroup-anatomy)
5. [SettingContainer anatomy](#5-settingcontainer-anatomy)
6. [Control primitives catalog](#6-control-primitives-catalog)
7. [Live-update contract](#7-live-update-contract)
8. [Live-preview fan-out contract](#8-live-preview-fan-out-contract)
9. [Dark-theme rules](#9-dark-theme-rules)
10. [CI gates](#10-ci-gates)

## 1. Color tokens

All brand/theme colors live in `src/App.css` inside the `@theme` block.
That block is the only place hex colors may be declared. Component code
references tokens via Tailwind utilities (`bg-logo-primary`, `text-text`)
or `var(--color-*)` in inline styles.

| CSS var | Tailwind prefix | Hex (light) | Purpose |
|---------|-----------------|-------------|---------|
| `--color-text` | `text-text`, `bg-text` | `#0f0f0f` / `#fbfbfb` dark | Body text + neutral foreground. |
| `--color-background` | `bg-background` | `#fbfbfb` / `#1E1E1E` dark | Page background. |
| `--color-background-ui` | `bg-background-ui` | `#D9D8D8` | Neutral chrome. `Button variant="primary"`. **Not brand.** |
| `--color-logo-primary` | `bg-logo-primary`, `text-logo-primary`, `ring-logo-primary` | `#E8A838` | **Brand accent.** `Button variant="brand"`, `Slider` fill, `ToggleSwitch` checked, `Badge primary`, progress bars, hover/focus rings. |
| `--color-logo-stroke` | `logo-stroke` | `#3D2B1F` / `#E8A838` dark | SVG logo stroke (flips in dark theme). |
| `--color-text-stroke` | `text-stroke` | `#f6f6f6` | `.text-stroke` utility only. |
| `--color-mid-gray` | `text-mid-gray`, `bg-mid-gray`, `border-mid-gray` | `#808080` | Muted/disabled text, 10–20 % tints. |
| `--color-accent` | `bg-accent`, `text-accent` | `#E8A838` | Legacy alias of `--color-logo-primary`. New code uses `logo-primary`. |

### Rules
1. **No hex literals** in `.ts`/`.tsx`/component `.css`. Only `src/App.css @theme`.
2. **Brand yellow has exactly one name.** Prefer `logo-primary`. `accent` is migration-legacy.
3. **Primary ≠ brand.** `Button variant="primary"` uses `background-ui`; `variant="brand"` uses `logo-primary`.
4. **Adding a color** means declaring a `--color-*` token in `App.css @theme`, exposing it in `tailwind.config.js`, and adding a row to the table above.

## 2. Primitive → token contract

| Primitive | Brand-carrying surface | Token |
|-----------|------------------------|-------|
| `Button` variant `brand` | background + border | `logo-primary` |
| `Button` variant `primary` | background + border | `background-ui` (neutral) |
| `Button` variant `secondary` | border, transparent fill | `mid-gray/20` |
| `Slider` / `SliderWithInput` | track fill + thumb | `logo-primary` |
| `ToggleSwitch` | checked background | `logo-primary` |
| `Badge` variant `primary` | background | `logo-primary` |
| `ProgressBar` | value fill | `logo-primary` |
| `Dropdown` (selected row + focus ring) | text + ring | `text-text` on `bg-background`; ring `logo-primary` |
| `Alert` variant `warning` | border + icon | `logo-primary` |

If a design calls for the brand color, use `logo-primary`. If it calls
for neutral chrome, use `background-ui`. If neither, `mid-gray` tints.
There is no fourth choice.

### Raw `<button>` is forbidden
Any raw `<button>` that carries `bg-logo-primary`, `bg-background-ui`, or
`bg-mid-gray/10` is a drift site — the shared `<Button>` component owns
variant → color → hover/focus/disabled mapping. Gate:
`scripts/gate/check-button-variant-drift.ts`.

## 3. Page anatomy (hero pattern)

Every top-level settings page opens with a hero block. Models, Advanced,
and About all follow this — it is the convention, and new pages follow
suit.

```tsx
<div className="max-w-5xl w-full mx-auto space-y-6">
  <div className="mb-4">
    <h1 className="text-xl font-semibold mb-2">
      {t("settings.<page>.title")}
    </h1>
    <p className="text-sm text-text/60">
      {t("settings.<page>.description")}
    </p>
  </div>
  {/* … SettingsGroup children … */}
</div>
```

Rules:
- Both `title` and `description` i18n keys must exist in all 20 locale files.
- Hero `<h1>` is the only `h1` on the page. `SettingsGroup` titles are `<h2>`.
- Do not repeat the page title as the first `SettingsGroup` title.

## 4. SettingsGroup anatomy

`SettingsGroup` is a card wrapper. `title` is optional; default is
title-only (no prose below it). Round-3 QC found per-group descriptions
noisy — reach for `description` **only** when a group needs onboarding
framing (e.g. Experimental's "This is off by default" banner).

```tsx
<SettingsGroup title={t("settings.advanced.groups.captions.title")}>
  <CaptionSettings descriptionMode="tooltip" grouped />
</SettingsGroup>
```

Rules:
- Titles are `<h2 text-xs font-medium text-mid-gray uppercase tracking-wide>`.
- If you render a hero on the page, do not also render a `SettingsGroup` whose title duplicates it.
- Per-group descriptions are opt-in, not default.

## 5. SettingContainer anatomy

`SettingContainer` is the atom for an individual setting row.

```
┌──────────────────────────────────────────────────────────────┐
│ Label  (i)                                           [control]│  ← horizontal (default)
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│ Label  (i)                                                   │  ← stacked
│ [────────── wide control ──────────]                         │
└──────────────────────────────────────────────────────────────┘
```

- **Left:** `<h3 text-sm font-medium>` = title, then an info icon.
- **Info icon:** `w-4 h-4 text-mid-gray cursor-help hover:text-logo-primary`
  SVG circle-i. Click/hover/tap opens a `Tooltip` carrying the description.
- **Right (horizontal)** / **Below (stacked):** the control itself.
- **`descriptionMode="tooltip"`** is the default. Reach for
  `descriptionMode="inline"` only when the control isn't self-explanatory
  without the description visible.
- **`layout="stacked"`** only when the control is wider than ~40 % of the
  row (multi-line textarea, color picker, multi-select chip grid).
- **DOM contract** (Playwright depends on it):
  `data-setting-role="row" | "label" | "description" | "control"`.

## 6. Control primitives catalog

| Primitive | When | Required token | Never reach for |
|-----------|------|----------------|-----------------|
| `Button` | Any user action | `brand` for primary CTA in a flow, `primary` for neutral, `secondary` for cancel/dismiss | Raw `<button>` with `bg-*` |
| `ToggleSwitch` | Boolean setting | `logo-primary` checked | `bg-background-ui` for "on" |
| `Slider` / `SliderWithInput` | Numeric setting | `logo-primary` fill+thumb | Non-live `onChange` (see §7) |
| `Dropdown` | One-of-N with > 3 options | `text-text` on `bg-background`; ring `logo-primary` | Yellow-on-yellow (round-2 contrast bug) |
| `Badge` | State indicator | `logo-primary` (primary), `mid-gray` (neutral) | Inventing new hue |
| `Alert` | Inline warning/info | `logo-primary` border | Red-on-dark (readability) |
| `ProgressBar` | Async work | `logo-primary` fill | Flashing/pulsing beyond 2 Hz |
| `Tooltip` | Info-icon description | `bg-background` + `border-mid-gray/20` | Long prose (>2 lines) — use stacked description |

## 7. Live-update contract

Numeric controls tied to a live-preview surface must update **during
drag**, not only on commit. Learned twice as a bug (round 2). The
`SliderWithInput` primitive fires `onChange` during `input` events and
fires a separate `onCommit` on `change`. Consumers should bind
preview-affecting setters to `onChange`.

```tsx
// ✅ live
<SliderWithInput
  value={caption.bg_alpha}
  onChange={(v) => updateCaptionProfile({ ...caption, bg_alpha: v })}
  min={0} max={1} step={0.01}
/>

// ❌ lag
<SliderWithInput
  value={caption.bg_alpha}
  onCommit={(v) => updateCaptionProfile({ ...caption, bg_alpha: v })}
/>
```

Debounce the downstream IPC if needed (it already is — see §8), not the
upstream UI. Users must feel their drag immediately.

## 8. Live-preview fan-out contract

Every `Settings[K]` field whose value renders through a live preview
(captions, waveform, export presets) **MUST** have a matching entry in
the `settingUpdaters` map in `src/stores/settingsStore.ts`. A missing
entry produces the silent-no-op class of bug — the Zustand store
optimistically updates, the backend never moves, the preview stays
stale (round 3 caption_profiles bug).

```ts
// src/stores/settingsStore.ts
const settingUpdaters: { [K in keyof Settings]?: (value) => Promise<unknown> } = {
  caption_profiles: async (value) => {
    const set = value as CaptionProfileSet;
    await commands.setCaptionProfile("Desktop", set.desktop, "App");
    await commands.setCaptionProfile("Mobile", set.mobile, "App");
  },
  // … every caption_/export_/loudness_/normalize_ key has an entry …
};
```

Rules:
- Any `Settings` field with prefix `caption_`, `export_`, `loudness_`,
  or `normalize_audio_` must appear in `settingUpdaters`.
- Complex values (like `CaptionProfileSet`) fan out to multiple
  backend commands inside one updater. Dedup at the IPC layer, not
  upstream.
- Gate: `scripts/gate/check-settings-updater-coverage.ts`.

## 9. Dark-theme rules

- Only neutral chrome flips (`text`, `background`). Brand yellow stays constant.
- **Never** red text on dark. **Never** light-grey on white.
- Use `text-mid-gray` (not `text-gray-*`) for muted foreground; it respects the theme.

## 10. CI gates

| Gate | Script | What it catches |
|------|--------|-----------------|
| Hex-token drift | `bun run check:design-tokens` | Any `#RRGGBB` outside `src/App.css @theme` |
| Button-variant drift | `bun run check:button-variants` | Raw `<button>` duplicating a `Button` variant |
| Settings updater coverage | `bun run check:settings-updater-coverage` | `caption_*`/`export_*`/`loudness_*`/`normalize_audio_*` keys missing from `settingUpdaters` |
| i18n parity | `bun run check:translations` | Keys present in `en/` but missing elsewhere |
| File-size cap | `bun run check:file-sizes` | Files > 800 lines under `src/` / `src-tauri/src/` |

All five run in `.github/workflows/ci.yml > frontend-quality`. Don't
`--no-verify` past them; fix the drift.
