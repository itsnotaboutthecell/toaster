# Area categorization: Edit Export Format Override

Check all areas this feature touches:

- [x] Frontend (React/TS) - Editor Export button flow
- [x] Backend (Rust managers) - `commands/waveform/commands.rs`, `export_format.rs`
- [ ] Audio path - preview untouched; splice untouched
- [ ] Transcription adapter
- [x] Export pipeline - `export_edited_media` command signature + FFmpeg argv assembly
- [x] Captions / UI strings (i18n) - new keys mirrored across all 20 locales
- [ ] Settings UI - Advanced -> Export kept as-is (verified only)
- [ ] Evals - no eval-harness change; precision / boundary evals re-run as regression gate only
