# Area categorization: export format mov

Check all areas this feature touches:

- [x] Frontend (React/TS) — `ExportGroup.tsx` dropdown option
- [x] Backend (Rust managers) — `AudioExportFormat` enum + argv builder
- [ ] Audio path — untouched (same AAC codec; same filter chain)
- [ ] Transcription adapter
- [x] Export pipeline — new muxer flag and extension
- [x] Captions / UI strings (i18n) — new label + description key x20
- [x] Settings UI — `Video (mov)` option in Advanced > Export
- [ ] Evals — no new fixture; AC-001-b is a manual ffprobe check
