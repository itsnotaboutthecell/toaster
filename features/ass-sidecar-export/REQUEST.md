# Feature request: ass sidecar export

## 1. Problem & Goals

Toaster already generates a full ASS (Advanced SubStation Alpha)
document in `src-tauri/src/managers/captions/ass.rs::blocks_to_ass`
(`src-tauri/src/managers/captions/ass.rs:38`) and hands it to FFmpeg's
`subtitles=` filter for the burn-in path
(`src-tauri/src/commands/waveform/commands.rs:430-443`). That exact ASS
document is never surfaced to the user as a file, even though the
product map explicitly calls this out as a Polish-milestone gap:

- `features/product-map-v1/PRD.md` §6 item F11 — "Subtitle sidecar
  export: SRT / VTT (already shipped), add ASS ... `captions/ass.rs`
  already produces ASS for burn-in; expose a 'Save .ass' button."
- `features/product-map-v1/PRD.md` §10 milestone 2.5 —
  `ass-sidecar-export` is the chosen slug.
- `features/product-map-v1/PRD.md` §6 item SR5 — "ASS is generated for
  burn-in but never offered as an export. Two lines of UI."

Users who caption in Aegisub / Subtitle Edit / ffmpeg-outside-Toaster
currently have no way to get Toaster's authoritative caption document
without either (a) running a burn-in export and discarding the video or
(b) handwriting ASS from scratch. Both defeat the single-source-of-truth
rule that the PRD establishes for caption layout.

Goal: add one opt-in checkbox "Also save .ass subtitle sidecar" to the
export toolbar so the same ASS document the burn-in path feeds to
FFmpeg is also written to `<basename>.ass` next to the exported media.
Zero duplication of layout/geometry logic: the sidecar path and the
burn-in path consume the same `blocks_to_ass` output.

## 2. Desired Outcome & Acceptance Criteria

- A new persisted boolean setting `export_ass_sidecar_enabled`
  (default `false`) round-trips through the typed-handler pattern used
  by `change_normalize_audio_setting`
  (`src-tauri/src/commands/app_settings.rs:487-492`).
- The EditorToolbar ships one new toggle, adjacent to the existing burn
  caption toggle (`src/components/editor/EditorView.tsx:538-548`),
  wired to the new setting and i18n'd across all 20 locales under
  `src/i18n/locales/*/translation.json`.
- When the toggle is on, `export_edited_media`
  (`src-tauri/src/commands/waveform/commands.rs:386`) writes a
  `<output-basename>.ass` file alongside the exported media containing
  the exact byte-for-byte document produced by `blocks_to_ass` for
  this edit.
- The sidecar and the burn-in flag are fully orthogonal: any of the
  four (0,0) / (0,1) / (1,0) / (1,1) combinations behaves as specified
  without duplicating caption layout code.
- A static / test-level assertion guarantees there is exactly one
  caller of the ASS-document builder per export invocation — no
  "build once for burn, build again for sidecar."

Detailed ACs live in `PRD.md`.

## 3. Scope Boundaries

### In scope

- Backend: new setting field, default, typed setter command, sidecar
  write inside `export_edited_media`, a small refactor that extracts
  "build the ASS document for this export" into a single helper the
  burn path and the sidecar path both call.
- Frontend: one new button in the export toolbar, one new i18n key in
  20 locales, one new plumbing line into `commands.exportEditedMedia`.
- Eval/tests: a unit test that pins single-site generation and a
  fixture-based manual check that the two artifacts are byte-identical.

### Out of scope (explicit)

- SRT / VTT / script sidecars (already shipped via
  `managers/export.rs`; this bundle does not restructure them).
- A standalone "export just the ASS, no media" command. The request
  explicitly piggybacks on `export_edited_media`.
- Any change to `blocks_to_ass` itself, to `CaptionBlock`, or to
  caption layout geometry.
- File-conflict dialogs. Overwrite silently, matching existing media
  export behaviour (user-confirmed; see §Q&A).
- Wiring this into a future "export all sidecars" bulk flow — deferred
  to whichever feature ships that bulk flow.

## 4. References to Existing Code

- `src-tauri/src/managers/captions/ass.rs:38` — `blocks_to_ass` entry
  point. Signature `pub fn blocks_to_ass(blocks: &[CaptionBlock]) ->
  String`. This is the single source of truth the sidecar must reuse.
- `src-tauri/src/managers/captions/mod.rs:10` — re-export
  `pub use ass::blocks_to_ass;` (already public at the module root).
- `src-tauri/src/commands/waveform/commands.rs:386-443` —
  `export_edited_media`; note the existing burn-captions conditional
  that builds `blocks`, calls `blocks_to_ass`, and writes a temp
  `.burn_captions.ass` next to the output. The sidecar branch piggybacks
  on the same `blocks` and `doc` computation.
- `src-tauri/src/commands/export.rs::build_caption_blocks_for_export` —
  referenced at `waveform/commands.rs:434` as the canonical
  `[CaptionBlock]` builder for export. Sidecar reuses it.
- `src-tauri/src/settings/types.rs:268-269` — precedent for an
  `export_*: bool` setting (`normalize_audio_on_export`).
- `src-tauri/src/settings/defaults.rs:541` — pattern for defaulting
  the new field to `false`.
- `src-tauri/src/commands/app_settings.rs:487-492` —
  `change_normalize_audio_setting` is the typed-handler template.
- `src-tauri/src/lib.rs:223` — command registration site.
- `src/components/editor/EditorView.tsx:49, 330, 464, 538-548` —
  existing `burnCaptions` local state, the `exportEditedMedia` call
  and the toolbar button wiring to mirror.
- `src/bindings.ts:754-756` — `exportEditedMedia` specta binding
  (regenerated automatically; no manual edit).
- `src/i18n/locales/en/translation.json:654` — `burnCaptions` key
  location; the new key lives adjacent.
- `scripts/check-translations.ts` — translation-parity gate.

### Adjacent feature bundles that matter

- `features/caption-parity-eval/` — the preview/export ASS parity
  harness. Its fixture does not reference `.ass` sidecar output
  (grepped: only `export.ass` intermediate artifact referenced); the
  sidecar path does not conflict with that eval because both consume
  the same `blocks_to_ass` string. Sidecar enabling this eval's future
  diffing of on-disk `.ass` is a bonus, not a dependency.
- `features/product-map-v1/` — F11 / SR5 / M2.5 are the parents of
  this bundle.

## 5. Edge Cases & Constraints

- Audio-only exports (`export_format.is_audio_only() == true`) already
  suppress the burn path at `waveform/commands.rs:430-433`. The sidecar
  has no such restriction conceptually (ASS for audio is
  still well-defined — Aegisub uses it for audio-drama captioning), but
  the feature must decide and document the behaviour. Default answer:
  **sidecar also suppressed for audio-only exports** so the user can
  never receive a `.ass` that references a `PlayResX/Y` frame with no
  matching video. Rationale: `blocks_to_ass` hard-codes
  `PlayResX/PlayResY` from `CaptionBlock.frame_width/height`
  (`ass.rs:39-42`); audio-only exports have no meaningful frame size.
- Input has no video (`has_video == false`): sidecar is suppressed,
  matching burn-path behaviour. Covered by AC-004-a (see PRD).
- Empty edit (no keep-segments): `export_edited_media` already errors
  at `waveform/commands.rs:401-403` before any ASS is built; no change.
- Empty caption stream (no `CaptionBlock`s): `blocks_to_ass`
  (`ass.rs:246-252`) still produces a valid header-only document. The
  sidecar path writes that valid empty document rather than skipping —
  a user toggling sidecar on expects a file out, and Aegisub opens a
  header-only ASS fine.
- File conflict: overwrite silently (user-confirmed; see §Q&A).
- Permissions / disk-full: propagate the error from `std::fs::write`
  back to the frontend as a `Result::Err` string, same convention as
  the existing burn-path temp write at `waveform/commands.rs:439`.
- i18n parity: the new key MUST appear in all 20 locales. The
  `scripts/check-translations.ts` gate catches drift.
- 800-line cap on `PRD.md` and `BLUEPRINT.md` (AGENTS.md rule).
- Local-only: the sidecar is a `std::fs::write` — no network, no
  hosted-inference dependency. Complies with AGENTS.md non-negotiable
  boundaries.

## 6. Data Model

One new field on `AppSettings`
(`src-tauri/src/settings/types.rs` around line 269):

```rust
#[serde(default)]
pub export_ass_sidecar_enabled: bool,
```

Default in `settings::defaults::default_settings()`
(`src-tauri/src/settings/defaults.rs:541` region): `false`.

No new DB, no migration, no schema changes beyond the new boolean. The
field is additive and `#[serde(default)]` handles legacy settings.json
files that predate the field — they load with `false` and the user
opts in explicitly.

## Q&A

Pre-answered by the requester; no additional Q&A pass was performed
(the three canonical open questions were closed in the request).

- Q: What is the default state of the checkbox for a fresh install?
  A: Off. Users opt in per-export.
- Q: Should the checkbox state persist across app restarts?
  A: Yes, via a new `export_ass_sidecar_enabled: bool` settings key
     (defaults to `false`), stored through the typed-handler pattern
     (mirrors `normalize_audio_on_export`).
- Q: What happens if `<basename>.ass` already exists in the target
  directory?
  A: Overwrite silently. This matches existing export behaviour for
     media files (`export_edited_media` does not check
     `output_path` for pre-existence either).
- Q: Is the sidecar produced for audio-only exports?
  A: No. Same gating as the burn path. See §5 for rationale.
