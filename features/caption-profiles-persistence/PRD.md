# PRD: caption-profiles-persistence

## R-001 — `CaptionProfile` + `CaptionProfileSet` types

New Rust structs with `specta::Type` derives so frontend bindings round-trip. Profile carries all 9 fields currently flat on `AppSettings`.

- AC-001-a — cargo test `caption_profile_roundtrip_serde` confirms JSON round-trip equals input.
- AC-001-b — cargo test `caption_profile_set_has_distinct_desktop_and_mobile_defaults` asserts defaults differ on at least `max_width_percent` and `position` (mobile uses narrower width + higher anchor).
- AC-001-c — `cargo check -p toaster --lib` green after type addition.

## R-002 — `AppSettings.caption_profiles` with migration

AppSettings gains `caption_profiles: CaptionProfileSet`. `ensure_caption_defaults(settings)` migrates old flat `caption_*` fields into both `desktop` and `mobile` on first load after upgrade. Flat fields remain deserializable for one release cycle; after the migrated save, the on-disk flat fields are ignored.

- AC-002-a — cargo test `caption_migration_seeds_profiles_from_flat_fields` passes.
- AC-002-b — cargo test `caption_migration_idempotent` passes (running twice does not duplicate work).
- AC-002-c — cargo test `caption_profiles_survive_full_settings_roundtrip` passes.

## R-003 — `ProjectSettings.caption_profiles: Option<CaptionProfileSet>`

New optional field. `None` means "inherit app-level"; `Some(set)` means project owns the profiles.

- AC-003-a — cargo test `project_v1_0_loads_with_none_profiles` passes using a fixture `.toaster` at version 1.0.0.
- AC-003-b — cargo test `project_save_bumps_version_to_1_1_0_and_writes_profiles` passes.
- AC-003-c — cargo test `project_import_preserves_caption_profiles_when_some` passes.

## R-004 — `compute_caption_layout` SSOT helper

New module `src-tauri/src/managers/captions/layout.rs`. Pure function taking `&CaptionProfile + VideoDims`, returning `CaptionLayout`. Called by the preview Tauri command AND the libass export composer. No layout math duplicated in React or in the export path.

- AC-004-a — cargo test `compute_caption_layout_matches_golden_fixture` passes against a committed JSON golden for 1920x1080 and 1080x1920 fixtures.
- AC-004-b — cargo test `preview_and_export_layouts_are_byte_identical` calls the preview command and the export composer against the same profile+dims and asserts equal `CaptionLayout`.
- AC-004-c — gate-ripgrep-absent ensures no duplicate position/margin math in `src/components/settings/captions/` or `src/components/editor/` that bypasses the Tauri command.

## R-005 — Tauri commands

`get_caption_profile(orientation: Orientation) -> CaptionProfile` (reads project-level if set, else app-level).
`set_caption_profile(orientation: Orientation, profile: CaptionProfile, scope: Scope)` where Scope is `App | Project`.
`get_caption_layout(orientation: Orientation, video_dims: VideoDims) -> CaptionLayout`.

- AC-005-a — cargo test `get_caption_profile_returns_project_when_present` passes.
- AC-005-b — cargo test `get_caption_profile_falls_back_to_app_when_project_is_none` passes.
- AC-005-c — cargo test `set_caption_profile_app_scope_persists_to_app_settings` passes.
- AC-005-d — cargo test `set_caption_profile_project_scope_persists_to_open_project` passes.

## R-006 — Orientation radio in the editor

Editor view gains a radio control with options `Desktop`, `Mobile`, `Auto`. Default is `Auto`. Auto picks by aspect ratio: width/height > 1.0 → desktop; ≤ 1.0 → mobile. User override persists for the current editor session but is not saved (profile is what's saved, not the UI toggle).

- AC-006-a — Live-app QC: import a 1920x1080 clip with Auto; confirm Desktop profile is used.
- AC-006-b — Live-app QC: import a 1080x1920 clip with Auto; confirm Mobile profile is used.
- AC-006-c — Live-app QC: override to the other profile; confirm preview updates; unload project; reopen; confirm Auto is the default again.

## R-007 — Settings UI split

`CaptionSettings.tsx` splits into `CaptionDesktopTab.tsx` + `CaptionMobileTab.tsx` + shared `CaptionProfileShared.tsx`. A tab/segmented control at the top of the Captions group selects which profile to edit. Each tab re-uses the Slice A `CaptionMockFrame` with its orientation pinned.

- AC-007-a — gate-tsc green.
- AC-007-b — gate-file-sizes green (no file > 800 LOC).
- AC-007-c — Live-app QC: switch between Desktop and Mobile tabs; confirm independent edit state + preview.

## R-008 — Project file v1.0.0 backward compatibility

A committed fixture `features/caption-profiles-persistence/fixtures/project_v1_0_0.toaster` loads, exercises the editor, round-trips through save, and exits as v1.1.0 with profiles populated.

- AC-008-a — cargo test `project_v1_0_0_fixture_migrates_on_save` passes.
- AC-008-b — Live-app QC: open the fixture, make a tiny edit, save, re-open, confirm profiles are present and version=="1.1.0".

## R-009 — i18n + labels

New keys for Desktop / Mobile / Auto, profile tab labels, orientation radio, and the Captions group description.

- AC-009-a — `bun run scripts/check-translations.ts` exit 0.
- AC-009-b — gate-ripgrep-absent confirms no new hard-coded English strings in `src/components/settings/captions/` or `src/components/editor/` radio.

## R-010 — Precision + boundary evals stay green

- AC-010-a — skill `transcript-precision-eval` run reports green.
- AC-010-b — skill `audio-boundary-eval` run reports green.

## R-011 — Static gates

- AC-011-a — `pwsh scripts/gate-lint.ps1` exit 0.
- AC-011-b — `pwsh scripts/gate-tsc.ps1` exit 0.
- AC-011-c — `pwsh scripts/gate-cargo-check-lib.ps1` exit 0.
- AC-011-d — `cargo test -p toaster --lib captions` exits 0 (all new caption tests).

## R-012 — Live-app smoke

- AC-012-a — `launch-toaster-monitored.ps1 -ObservationSeconds 240` launches cleanly with no error signals after all changes.
