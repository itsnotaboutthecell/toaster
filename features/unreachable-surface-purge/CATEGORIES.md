# Area categorization: Unreachable Surface Purge

Check all areas this feature touches:

- [x] Frontend (React/TS) - Sidebar.tsx, post-processing panel,
  debug panel wrapper.
- [x] Backend (Rust managers) - settings/types.rs,
  commands/waveform/mod.rs (delete helper + dead branch).
- [ ] Audio path
- [ ] Transcription adapter
- [ ] Export pipeline
- [x] Captions / UI strings (i18n) - sidebar.general + overlay
  namespace purge across 20 locale files.
- [x] Settings UI - re-mount debug panel; confirm post-processing
  panel loopback label.
- [ ] Evals
