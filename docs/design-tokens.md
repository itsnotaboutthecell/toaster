# Design tokens — single source of truth

All Toaster brand/theme colors live in `src/App.css` inside the `@theme` block. That block is the only place hex colors may be declared. Component code MUST reference the tokens via Tailwind utilities (e.g. `bg-logo-primary`, `text-text`) or `var(--color-*)` in inline styles.

## Tokens

| CSS var | Tailwind utility prefix | Hex (light) | Purpose |
|---------|-------------------------|-------------|---------|
| `--color-text` | `text-text`, `bg-text`, `border-text` | `#0f0f0f` / `#fbfbfb` dark | Body text + neutral foreground. |
| `--color-background` | `bg-background` | `#fbfbfb` / `#1E1E1E` dark | Page background. |
| `--color-background-ui` | `bg-background-ui`, `border-background-ui` | `#D9D8D8` | Neutral chrome. `Button variant="primary"` uses this. Do NOT use for brand/accent. |
| `--color-logo-primary` | `bg-logo-primary`, `text-logo-primary`, `border-logo-primary`, `ring-logo-primary` | `#E8A838` (brand yellow) | **Brand accent.** `Button variant="brand"`, `Slider` track fill, `ToggleSwitch` checked state, `Badge primary`, progress bars, hover/focus rings. Single token for anything that should read as "Toaster yellow". |
| `--color-logo-stroke` | `logo-stroke` utilities | `#3D2B1F` / `#E8A838` dark | SVG logo stroke (flips in dark theme). |
| `--color-text-stroke` | `text-stroke` | `#f6f6f6` | `.text-stroke` utility only. |
| `--color-mid-gray` | `bg-mid-gray`, `text-mid-gray`, `border-mid-gray` | `#808080` | Muted/disabled text, 10–20 % tints for surfaces and borders. |
| `--color-accent` | `bg-accent`, `text-accent`, `border-accent` | `#E8A838` | **Alias of `--color-logo-primary`** (both resolve to the brand yellow). Retained only for legacy callers (onboarding `ModelCard`, `EditorView` top bar, `CaptionOrientationRadio`). New code should prefer `logo-primary`. |

## Rules

1. **No hex literals in component code.** Any `#RRGGBB` outside `src/App.css @theme` is a drift bug. CI gate: `scripts/gate/check-brand-token-drift.ts` (wired in Commit C of the design-tokens sprint).
2. **Brand yellow has exactly one name.** Prefer `logo-primary`. `accent` is an alias kept for migration, not a second design decision.
3. **Primary ≠ brand.** `Button variant="primary"` uses the neutral `background-ui`; `variant="brand"` uses `logo-primary`. Do not mix.
4. **Dark theme**: only neutral chrome (`--color-text`, `--color-background`) flips. The brand yellow is constant across themes.
5. **Adding a new color** means declaring a new `--color-*` token in `App.css @theme`, exposing it in `tailwind.config.js` if `bg-foo`/`text-foo` utilities are needed, and adding a row to this table.

## Primitive contract

| Primitive | Brand-carrying surface | Token |
|-----------|------------------------|-------|
| `Button` variant `brand` | background + border | `logo-primary` |
| `Button` variant `primary` | background + border | `background-ui` (neutral) |
| `Slider` | track fill + thumb | `logo-primary` (thumb in `App.css` range-thumb rule; fill in `Slider.tsx`) |
| `ToggleSwitch` | checked background | `logo-primary` |
| `Badge` variant `primary` | background | `logo-primary` |
| `ProgressBar` | value fill | `logo-primary` |

If a design calls for the brand color, use `logo-primary`. If it calls for neutral chrome, use `background-ui`. If neither, pick `mid-gray` tints. There is no fourth choice.
