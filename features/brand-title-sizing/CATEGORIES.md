# Area categorization: brand title sizing

Check all areas this feature touches:

- [x] Frontend (React/TS) -- Sidebar, App, EditorView, settings panes.
- [ ] Backend (Rust managers)
- [ ] Audio path
- [ ] Transcription adapter
- [ ] Export pipeline
- [ ] Captions / UI strings (i18n) -- no new keys; SVG `alt="Toaster"`
      stays as-is.
- [x] Settings UI -- only the `max-w-*` cap on the root container of
      About / Advanced / History / Models panes; no controls touched.
- [ ] Evals

## Justification

Pure presentational layout change. No new color tokens (AGENTS.md
"Settings UI contract"), no new i18n strings, no behavior change in
audio / transcription / export / captions. Touched files stay well
under the 800-line cap.
