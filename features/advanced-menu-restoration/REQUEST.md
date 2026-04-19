# Feature request: Advanced menu restoration

## 1. Problem & Goals

The Allow / Discard word lists were relocated from a previous Advanced settings panel into the inline editor (`src/components/editor/EditorView.tsx:574-578` — note the comment "relocated from the deleted Advanced panel"). The user reports this as a UX regression: word lists are configured once per project (or once globally) and almost never edited mid-edit, so surfacing them inline next to per-clip controls is overkill and steals attention from per-clip workflow.

The user has explicitly asked for a broader audit: **"what other options need to be moved back into the Advanced menu?"** — so this feature is not just "undo the relocation" but also "identify other set-once controls that should not live in the primary editor surface."

**Goal:** restore an Advanced panel (or equivalent collapsed surface) under Settings, move the Allow / Discard word lists back into it, and document a frequency-of-use heuristic plus the audit results so future settings placement decisions are not ad hoc.

## 2. Desired Outcome & Acceptance Criteria

- The inline `<DiscardWords>` and `<AllowWords>` blocks are removed from `EditorView.tsx`. The editor only shows per-clip controls.
- Settings has an Advanced section, collapsed by default, that contains the Allow / Discard word lists with identical functionality (add, remove, conflict detection, sanitization, persistence under `custom_words` / `custom_filler_words`).
- An audit document lists every current setting against a "configured once" vs "tweaked per clip" heuristic, with at least the word lists relocated and a written recommendation for any other set-once controls (no extra forced moves — the user asked us to *identify* them; moving each is its own decision).
- All 22 i18n locales stay in sync (`scripts/check-translations.ts` exit 0).

(See `PRD.md` for the formalized AC list.)

## 3. Scope Boundaries

### In scope

- Remove the inline word-list block from `src/components/editor/EditorView.tsx`.
- Add an Advanced section to Settings (collapsed by default) containing `<DiscardWords>` and `<AllowWords>`.
- Keep all underlying setting keys (`custom_words`, `custom_filler_words`) and component logic unchanged — pure relocation.
- Audit current settings against a documented frequency-of-use heuristic; record results in `features/advanced-menu-restoration/journal.md`.
- Add new i18n keys (`settings.advanced.title`, `settings.advanced.description`) across all 22 locales.

### Out of scope (explicit)

- Changing the schema of `custom_words` / `custom_filler_words` or any other setting.
- Adding new word-list features (search, import / export, bulk edit).
- Forcibly relocating any other setting in this PR — the audit produces *recommendations*, each subsequent move is a separate feature.
- Mobile / responsive treatment of the Advanced panel beyond what the existing settings surface already does.

## 4. References to Existing Code

- `src/components/editor/EditorView.tsx:574-578` — the inline relocation that this feature reverses (note the `relocated from the deleted Advanced panel` comment).
- `src/components/settings/AllowWords.tsx:14-129` — component to keep verbatim, only its render site moves.
- `src/components/settings/DiscardWords.tsx:14-129` — same.
- `src/components/settings/post-processing/PostProcessingSettings.tsx:43-141` — canonical example of a `<SettingsGroup>` + `<SettingContainer>` page; the Advanced panel mirrors this pattern.
- `src/components/settings/experimental/ExperimentalSettings.tsx:12-65` — the closest precedent for a "this lives off the main settings page" surface; review for layout pattern reuse.
- `src/i18n/locales/*/translation.json` — must add `settings.advanced.*` keys to all 22 locales (see `scripts/check-translations.ts` for the gate).
- `AGENTS.md` Settings UI contract — every control needs human-readable label + one-line description; this feature must not regress that.

## 5. Edge Cases & Constraints

- The Allow / Discard word lists currently render with `grouped` wrapped in a `<SettingsGroup>` titled "editor.sections.words" inside the editor. The new Advanced page must use the same `grouped` semantics (so the visual shells stay identical) but inside a Settings page, not the editor view.
- Conflict detection between Allow and Discard lists already lives in the components themselves and must keep working when rendered in the new location (no test regression).
- The Advanced panel collapsed-state preference: prefer **no new persisted setting** unless audit shows users will repeatedly toggle it. Default = collapsed; remembering across sessions can land in a follow-up.
- ASCII only in journal / docs / planning artifacts (per AGENTS.md output discipline).
- The `i18n-pruning` skill is mandatory for the new keys.

## 6. Data Model (optional)

No data-model change. Pure UI relocation. The audit may later recommend a `settings_advanced_expanded: boolean` setting; if so, that lands in a separate feature.

## Q&A

Resolved 2026-04-18:

- **Nav placement:** **Sidebar item** (restores the previous design). Advanced is a top-level Settings sidebar entry, matching how it existed before the regression. Implementers: do *not* introduce a new route, modal, or collapsible section - add it back to the Settings sidebar where it used to live.
- **Audit output:** Kept in `journal.md` + a recommendation table in `BLUEPRINT.md`. No auto-relocation of non-AllowWords/DiscardWords controls in this PR; audit findings spawn follow-up features if needed.
