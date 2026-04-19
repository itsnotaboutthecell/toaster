# Feature request: export format mov

## 1. Problem & Goals

Toaster's export pipeline can only emit `.mp4` (H.264 + AAC). Many
professional editing workflows (DaVinci Resolve, Final Cut Pro X,
Adobe Premiere) prefer a `.mov` container for ingest — particularly on
macOS — even when the underlying codec is still H.264. This bundle
adds `.mov` as an opt-in container alongside the existing `Mp4`
variant of `AudioExportFormat`. No codec change, no muxer policy
change, no splice/keep-segment/time-mapping change. Only the muxer
flag and file-extension differ.

Roadmap anchor: Milestone 2 item 2.1 of
`features/product-map-v1/PRD.md:471` — "Add mov container option
(H.264 inside mov); same codec, different muxer flag."

## 2. Desired Outcome & Acceptance Criteria

- Backend enum gains a `Mov` variant with extension `.mov` and
  FFmpeg muxer `mov`; code path reuses the existing libx264 + AAC
  composition (`src-tauri/src/commands/waveform/mod.rs:571-729`).
- Export dialog exposes `mov` as a selectable format; the setting
  round-trips through `settings.json`.
- i18n key `settings.export.format.options.mov.{label,description}`
  added to all 20 locale files under `src/i18n/locales/*/translation.json`.
- When the `hardware-encoder-fallback` bundle lands, enabling the
  hardware encoder with `.mov` selected uses the same codec argv it
  would use for `.mp4`; only the container differs.
- No regression on any existing AC in
  `features/export-audio-only/PRD.md` or
  `features/export-hardware-encoder/PRD.md`.

## 3. Scope Boundaries

### In scope

- New `AudioExportFormat::Mov` variant in
  `src-tauri/src/commands/waveform/export_format.rs`.
- Explicit `-f mov` muxer flag and `-pix_fmt yuv420p` where not
  already inherited from the libx264 encoder profile, added in
  `build_export_args` (`src-tauri/src/commands/waveform/mod.rs:571`).
- Frontend dropdown addition in
  `src/components/settings/advanced/ExportGroup.tsx:18` (the
  `EXPORT_FORMATS` literal) plus the matching i18n key.
- 20-locale translation parity for the new key.
- Extension-swap defensive logic at
  `src-tauri/src/commands/waveform/commands.rs:464-473` automatically
  rewrites the save-dialog extension if the user picked mov from the
  settings but the filename still ends in `.mp4`.

### Out of scope (explicit)

- ProRes, yuv422p10le, HEVC, AV1, VP9, or any non-H.264 codec.
- Audio-only inside `.mov` (Mov is a **video** container variant in
  this bundle; if the user selects an audio-only format the mov
  option is disabled).
- Hardware-encoder detection / selection / fallback — owned by
  `features/export-hardware-encoder/`.
- Export-dialog layout refactor (kept as inline dropdown).
- File-association / MIME registration on Windows / macOS / Linux.
- webm, mkv, flac, or any other container enumerated in
  `features/product-map-v1/PRD.md:406-408`.

## 4. References to Existing Code

- `src-tauri/src/commands/waveform/export_format.rs:25-58` —
  `AudioExportFormat` enum, `is_audio_only`, `extension` impl.
  The `Mov` variant is added here.
- `src-tauri/src/commands/waveform/export_format.rs:89-113` —
  `export_format_codec_map`. Mov returns `None` (video pipeline owns
  codec selection, same as `Mp4`).
- `src-tauri/src/commands/waveform/mod.rs:571-729` —
  `build_export_args`; where the muxer `-f mov` and pixel format flag
  are added when `format == AudioExportFormat::Mov`.
- `src-tauri/src/commands/waveform/commands.rs:455-493` —
  `export_edited_media` reads `settings.export_format` and calls
  `build_export_args`. Extension-swap logic at lines 464-473.
- `src-tauri/src/settings/types.rs:289-294` — `export_format` field
  on `AppSettings`; serde-derived so round-trip is automatic once the
  enum variant is added.
- `src-tauri/src/settings/defaults.rs:546` — default is `Mp4`; mov
  is opt-in, default unchanged.
- `src/components/settings/advanced/ExportGroup.tsx:18` —
  `EXPORT_FORMATS` TS literal. The single frontend duplication point
  of the backend enum; must be extended in lock-step.
- `src/i18n/locales/en/translation.json:455-472` — existing
  `settings.export.format.options.{mp4,mp3,...}.{label,description}`
  key pattern to mirror.
- `src-tauri/tests/export_format_codec_map.rs` — existing backend
  codec-map test; add a parallel round-trip / parity test for Mov.
- `scripts/check-translations.ts` — the 20-locale parity gate.

## 5. Edge Cases & Constraints

- **FFmpeg container inference.** Today the code does not pass `-f
  <mux>` explicitly; FFmpeg infers the muxer from the output
  extension. This works for `.mov` out of the box, but adding an
  explicit `-f mov` is a defensive improvement so mis-extension cases
  (rare, guarded by the swap at `commands.rs:464`) still produce a
  valid mov.
- **Pixel format.** `libx264` defaults to `yuv420p` for mov outputs
  when the source is `yuv420p`, which matches the existing mp4
  profile. ProRes-style `yuv422p10le` is explicitly out of scope.
- **Hardware-encoder interop.** The
  `features/export-hardware-encoder/` bundle will switch `-c:v
  libx264` (`commands/waveform/mod.rs:510,522`) to a hardware encoder
  variant. This bundle's contract: the container choice (`-f mov` vs
  `-f mp4`) must be orthogonal to the codec choice. A regression
  test asserts the codec-portion of the argv is identical for Mov
  and Mp4 given the same input.
- **Audio-only × Mov.** Not a combination we support; selecting Mov
  implicitly means "video export with mov muxer". If a user has an
  audio-only source, `effective_has_video` is `false`
  (`commands/waveform/commands.rs:457`) regardless of container
  choice, but a `.mov` with only an audio track is still valid per
  QuickTime spec. We allow it but do not advertise it.
- **Settings file backward compatibility.** Old `settings.json`
  files containing `"export_format": "mov"` currently fail to
  deserialize; after this change they deserialize cleanly. Forward
  compatibility (new settings files on old builds) is not a goal.
- **800-line cap.** `commands/waveform/mod.rs` is 704 lines today;
  this bundle adds < 20 lines. `export_format.rs` is 189 lines.
  Neither approaches the cap.

## 6. Data Model

Extend the existing enum; no new struct.

```rust
// src-tauri/src/commands/waveform/export_format.rs
#[serde(rename_all = "lowercase")]
pub enum AudioExportFormat {
    #[default]
    Mp4,
    Mov,      // NEW
    Mp3,
    Wav,
    M4a,
    Opus,
}
```

Serialized form: `"mov"` (lowercase, matches existing variants).
Extension: `".mov"`. Codec map: `None` (video pipeline).

## Q&A

> The parent session pre-answered all open questions. The entries
> below are recorded verbatim from the REQUEST seed so
> coverage/audit trails survive re-read.

**Q1. Pixel format for H.264-in-mov?**
A: `yuv420p`. Widest editor compatibility; matches the existing mp4
profile. `yuv422p10le` / ProRes explicitly out of scope.

**Q2. Codec inside the mov container?**
A: H.264 only. HEVC and ProRes are out of scope.

**Q3. Audio track codec?**
A: AAC, matching the current mp4 profile. No change to audio
encoder selection.

**Q4. Default container?**
A: Unchanged (`Mp4`). Mov is opt-in via the export dropdown.

**Q5. Interaction with `hardware-encoder-fallback`?**
A: When the HW encoder is enabled, `.mov` must use the same HW
codec argv as `.mp4`; container selection must not bypass the
encoder path. Verified via codec-parity cargo test (AC-003-a).

