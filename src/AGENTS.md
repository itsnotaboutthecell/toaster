# src — Frontend (React + TypeScript) conventions

Path-scoped conventions for `src/`. Authoritative per the
[AGENTS.md nearest-file rule](https://agents.md/). The root
[`../AGENTS.md`](../AGENTS.md) covers cross-cutting rules; this file covers
frontend-specific ones.

## Quick rules

- NEVER use `any`. Strict TypeScript; prefer `unknown` + type guards if inference cannot narrow.
- ALWAYS use functional components with hooks; no class components.
- ALWAYS use `@/` as the import alias for `src/`.
- ALWAYS use Tailwind utility classes; no inline `style` except for dynamic numeric values that can't be expressed in Tailwind.
- ALWAYS route user-visible strings through i18next (`t('namespace.key')`). Hardcoded English is a bug — gated by `bun scripts/check-translations.ts` across all 20 locales.
- NEVER re-derive keep-segment / time-mapping / caption layout in frontend code. The backend is the single source of truth; call the relevant Tauri command.
- NEVER swap the `<video>` element source to an audio preview file. Sync preview audio independently.
- FORBIDDEN: hand-editing `src/bindings.ts` except for temporary single-line union additions — it is specta-generated (see below).
- FORBIDDEN: files over **800 lines** in `src/` (excluding generated `bindings.ts`). Enforced by `bun run check:file-sizes`. Grandfathered offenders in `scripts/file-size-allowlist.txt`.
- PREFER Zustand stores in `src/stores/` for cross-component state; local `useState` for component-local.

## Settings UI contract

Applies to `src/components/settings/**`. Full rules, anatomy diagrams,
and CI gates live in [`../docs/design-system.md`](../docs/design-system.md);
the nested [`components/settings/AGENTS.md`](components/settings/AGENTS.md)
is the path-scoped shortcut for agents editing those files.

- Every top-level settings page opens with a **hero** (`<h1 text-xl>` + `<p text-sm text-text/60>`); one `<h1>` per page, SettingsGroup titles are `<h2>`.
- Every user-exposed setting renders a **human-readable label and one-line description** (never raw flag names).
- Numeric preview-affecting controls must **update during drag** (`SliderWithInput` `onChange`, not `onCommit`) — gate: live QC of a caption slider.
- Every `caption_*` / `export_*` / `loudness_*` / `normalize_audio_*` `Settings` key written via `updateSetting("…", …)` MUST have an entry in `settingUpdaters` (`src/stores/settingsStore.ts`) — gate: `bun run check:settings-updater-coverage`.
- Use existing color tokens; the brand yellow is `bg-logo-primary`, neutral chrome is `bg-background-ui`. Never red-on-dark, never light-grey-on-white.

## Design tokens — single source of truth

All brand/theme colors live in `src/App.css` inside the `@theme` block — it's the only place hex literals may be declared. See [`../docs/design-system.md`](../docs/design-system.md) for the full token table, primitive contract, page anatomy, live-update and live-preview fan-out contracts, and CI gates.

- NEVER write hex color literals (`#RRGGBB`) in `.ts` / `.tsx` / component `.css` files. Reference tokens via Tailwind utilities (`bg-logo-primary`) or `var(--color-*)` in inline styles.
- NEVER use `bg-background-ui` / `peer-checked:bg-background-ui` / `bg-accent` to express the brand color — use `logo-primary`. `accent` is a legacy alias; new code uses `logo-primary`.
- Brand-carrying primitives (`Button variant="brand"`, `Slider`, `ToggleSwitch` checked state, `Badge variant="primary"`, `ProgressBar` fill) all share the **same** token `--color-logo-primary`. A primitive reaching for a different token is a drift bug.

## `src/bindings.ts` — specta-generated

`src/bindings.ts` is regenerated on every debug `cargo tauri dev` startup (see
`src-tauri/src/lib.rs` around the specta builder). Post-processing rewrites
`e as any` → `e as string` and appends a void-trailer.

- **Do not hand-edit this file** except for forward-compatible single-line patches (e.g. adding a new variant to a union).
- Any hand-patch must be verified by a subsequent successful `cargo tauri dev` launch before claiming the feature complete — otherwise struct field additions silently drift between Rust and TS.
- `knip` lists many `src/bindings.ts` types as unused. This is expected for a generated file and must not be silenced by hand-editing.

## i18n hygiene

- When deleting, renaming, or adding a user-visible key, invoke the `i18n-pruning` skill.
- All 20 locale files in `src/i18n/locales/*/translation.json` must stay in sync — CI-enforced by `bun scripts/check-translations.ts`.
- Do not add a key to `en/translation.json` alone.

## Example

```typescript
// ✅ Good — typed props, i18next key, no `any`
const DeleteButton: React.FC<{ wordIds: string[] }> = ({ wordIds }) => {
  const { t } = useTranslation();
  return <button onClick={() => deleteWords(wordIds)}>{t('editor.delete')}</button>;
};

// ❌ Bad — any type, hardcoded string, class component
class DeleteButton extends React.Component<any> {
  render() { return <button onClick={() => deleteWords(this.props.ids)}>Delete</button>; }
}
```

## Testing & verification

- No frontend unit-test framework. Gates are `npm run lint`, `npm run build`, and live-app inspection via `scripts\launch-toaster-monitored.ps1`.
- For user-visible flow changes, add a Playwright spec under `tests/` and run `npx playwright test`.
- For UI or playback-path changes, a live-app check is required per the "Verified means the live app, not `cargo check`" rule in [`../AGENTS.md`](../AGENTS.md).

## Related

- [`../src-tauri/AGENTS.md`](../src-tauri/AGENTS.md) — backend / Rust conventions.
- Root [AGENTS.md](../AGENTS.md) "Skills and agents" — `i18n-pruning`, `canonical-instructions`, `dep-hygiene`.
- [`../.github/instructions/code-review.instructions.md`](../.github/instructions/code-review.instructions.md) — Toaster-specific review gates.
