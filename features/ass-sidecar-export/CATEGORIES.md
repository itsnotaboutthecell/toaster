# Area categorization: ass sidecar export

Check all areas this feature touches:

- [x] Frontend (React/TS)
- [x] Backend (Rust managers)
- [ ] Audio path
- [ ] Transcription adapter
- [x] Export pipeline
- [x] Captions / UI strings (i18n)
- [x] Settings UI
- [ ] Evals

Notes:

- Audio path untouched — sidecar is a pure `std::fs::write` of an
  already-generated string; no FFmpeg/ffprobe invocation added.
- Transcription adapter untouched — consumes existing `CaptionBlock`s.
- Evals: `features/caption-parity-eval/` is adjacent (it compares
  preview vs export ASS); the sidecar does not extend that harness in
  this bundle, though a follow-up could diff on-disk `.ass` files.
- Settings UI: the new boolean is read/written through the same typed
  handler path as `normalize_audio_on_export`
  (`src-tauri/src/commands/app_settings.rs:487-492`). If a dedicated
  Settings panel toggle is desired beyond the toolbar checkbox, that is
  deferred — the toolbar checkbox is the canonical entry point.
