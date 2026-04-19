# Area categorization: poster-frame-export

Check all areas this feature touches:

- [x] Frontend (React/TS) -- one new context-menu entry in
      `src/components/editor/TranscriptEditor.tsx` and a Tauri
      command invocation to persist the chosen timestamp.
- [x] Backend (Rust managers) -- new `poster_frame_ms` field on
      `ProjectSettings` in `src-tauri/src/managers/project.rs`;
      project version bump to 1.2.0 with backward-compat.
- [ ] Audio path
- [ ] Transcription adapter
- [x] Export pipeline -- poster-frame extraction + attachment
      injected into `build_export_args` in
      `src-tauri/src/commands/waveform/mod.rs`; temp PNG lifecycle
      in `src-tauri/src/commands/waveform/commands.rs`.
- [x] Captions / UI strings (i18n) -- one new i18next key for the
      context-menu label, mirrored across all 20 locales.
- [ ] Settings UI
- [x] Evals -- new ffprobe-based assertion that mp4/mov exports
      with a selected poster frame expose an attached-picture
      stream and exports without one do not.
