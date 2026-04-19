# Area categorization: time stretch segments

Check all areas this feature touches:

- [x] Frontend (React/TS) — segment context-menu control, player
      `editTimeToSourceTime` routing, `<video>` playback-rate sync.
- [x] Backend (Rust managers) — persisted `SegmentStretch`,
      `canonical_keep_segments_for_media`, time-map helpers,
      captions layout.
- [x] Audio path — `atempo` injection in
      `build_audio_segment_filter`; preview renderer cache key.
- [ ] Transcription adapter — unaffected.
- [x] Export pipeline — audio + video filter_complex (`setpts`
      stretching for video stream).
- [x] Captions / UI strings (i18n) — new strings for context-menu
      control; all 20 locales mirrored.
- [ ] Settings UI — no global settings change; per-segment only.
- [x] Evals — `transcript-precision-eval` and `audio-boundary-eval`
      fixtures extended with stretched segments.
