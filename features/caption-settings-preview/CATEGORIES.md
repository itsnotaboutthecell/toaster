# Area categorization: Caption Settings Preview

Check all areas this feature touches:

- [x] Frontend (React/TS) - new preview pane component, reuse of
      `CaptionOverlay`.
- [ ] Backend (Rust managers) - explicitly untouched.
- [ ] Audio path - n/a.
- [ ] Transcription adapter - n/a.
- [ ] Export pipeline - explicitly untouched
      (`src-tauri/src/managers/captions/ass.rs` is read-only here).
- [x] Captions / UI strings (i18n) - three sample strings + one
      preview-pane heading and one dropdown legend across all 20
      `src/i18n/locales/*/translation.json` files.
- [x] Settings UI - `src/components/settings/CaptionSettings.tsx`
      gains a preview pane above the controls.
- [ ] Evals - no new eval; manual live-app verification only.
