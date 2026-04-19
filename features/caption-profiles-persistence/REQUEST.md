# Feature request: caption-profiles-persistence

## 1. Problem & Goals

Slice A of `caption-designer-orientation` delivered the visual orientation mock (horizontal + vertical) as an ephemeral React toggle. Users told us the real need is persistent per-video-format profiles: "desktop" (landscape) and "mobile" (portrait) caption configurations that follow a project and auto-switch based on the imported video's aspect ratio.

The current schema has 9 flat `caption_*` fields on `AppSettings` and zero caption data on `ProjectSettings`. This feature introduces per-orientation profiles on both, with `ProjectSettings` taking precedence when an imported project carries its own.

Single-source-of-truth is the dominant risk: captions are a **dual-path** concern (React preview + libass export). The backend must own a single `compute_caption_layout(profile, video_dims)` function consumed verbatim by both paths. A fixture test asserts byte-identical output.

## 2. Desired outcome & acceptance criteria

- `AppSettings.caption_profiles: { desktop: CaptionProfile, mobile: CaptionProfile }` replaces (via migration) the 9 flat `caption_*` fields.
- `ProjectSettings.caption_profiles: Option<CaptionProfileSet>` — when present, overrides app-level; when `None`, app-level is used.
- New Tauri-side `compute_caption_layout(profile, video_dims) -> CaptionLayout` in a new module; React preview and FFmpeg/libass export both consume its output.
- Editor UI gains an orientation radio (Desktop | Mobile | Auto); Auto picks by aspect ratio on project open.
- Opening an older `.toaster` project (version 1.0.0, no caption_profiles) loads with app-level profiles; no data loss.
- Re-saving a project always writes the project-level profile so round-trip is stable.
- Precision + audio-boundary evals stay green.

## 3. Scope boundaries

**In scope:**
- Schema additions to `AppSettings` + `ProjectSettings`, with migration from the 9 flat fields.
- Backend `compute_caption_layout` SSOT helper + fixture byte-identical test.
- Tauri commands to read/write profiles; FE wiring.
- Editor radio (Desktop/Mobile/Auto) with aspect-ratio auto-detect.
- Settings UI splits Captions into Desktop and Mobile sub-tabs.
- Migration: on first-run-after-upgrade, read the old flat fields once, seed both `desktop` and `mobile` with the same values, drop the flat fields on next save.
- Project file format bump to 1.1.0 with backward-compat.
- i18n across all 20 locales.

**Out of scope:**
- >2 profiles (square / custom). Keep landscape+portrait for v1.
- Per-scene profile switching inside one video.
- Caption animation / reveal effects.
- Style import/export between projects.

## 4. References to existing code

- `src-tauri/src/settings/types.rs:297-314` — 9 flat `caption_*` fields to migrate.
- `src-tauri/src/settings/defaults.rs` — `default_caption_*` helpers.
- `src-tauri/src/managers/project.rs` — `ToasterProject` + `ProjectSettings` + `PROJECT_VERSION`.
- `src-tauri/src/managers/export/` and `src-tauri/src/commands/waveform/` — ASS/libass export path that consumes caption fields today.
- `src/components/settings/captions/CaptionSettings.tsx` (518 LOC post-Slice-A).
- `src/components/settings/captions/CaptionMockFrame.tsx` — Slice A component; reuse for per-profile preview.
- `src/components/editor/` — target home for the orientation radio.
- `eval/fixtures/caption-mock-h-and-w.png` — design reference for the profile editor.

## 5. Edge cases & constraints

- Importing a 1.0.0 project must not lose existing caption fields — seed the ProjectSettings profile set from the AppSettings profile set at load time, then persist on next save.
- A user may have tweaked app-level caption_* on an older build, saved a project, then upgraded; the project's loaded caption_profiles (if `None`) must snap to the newly-structured app-level profiles.
- Auto orientation detection: aspect_ratio > 1.0 → desktop; ≤ 1.0 → mobile. Square (1.0) → desktop by tiebreak (landscape is more common for desktop workflows).
- `compute_caption_layout` must be called by both the Tauri-boundary preview command AND the libass export — the byte-identical test is the gate. No frontend duplication of position math.
- File-size cap 800 lines: `CaptionSettings.tsx` is at 518; will likely exceed. Split into `CaptionDesktopTab.tsx`, `CaptionMobileTab.tsx`, `CaptionProfileShared.tsx`.
- Migration logic (flat → profiles) runs once in `ensure_caption_defaults(settings)` in `settings/defaults.rs`, with a one-shot `caption_migration_complete` flag OR by presence of `caption_profiles`.

## 6. Data model

```rust
// settings/types.rs
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
pub struct CaptionProfile {
    pub font_size: u32,
    pub bg_color: String,
    pub text_color: String,
    pub position: u32,          // 0..100, vertical anchor (% of short axis)
    pub font_family: CaptionFontFamily,
    pub radius_px: u32,
    pub padding_x_px: u32,
    pub padding_y_px: u32,
    pub max_width_percent: u32, // 20..100, % of long axis
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
pub struct CaptionProfileSet {
    pub desktop: CaptionProfile, // landscape orientation
    pub mobile: CaptionProfile,  // portrait orientation
}

// AppSettings gains:
#[serde(default = "default_caption_profiles")]
pub caption_profiles: CaptionProfileSet,
// (old 9 flat caption_* fields deprecated; kept on-disk for one migration cycle, then dropped.)

// ProjectSettings gains:
#[serde(default)]
pub caption_profiles: Option<CaptionProfileSet>,

// managers/captions/layout.rs (new module):
pub struct VideoDims { pub width: u32, pub height: u32 }
pub struct CaptionLayout {
    pub margin_v_px: u32,
    pub margin_h_px: u32,
    pub box_width_px: u32,
    pub font_size_px: u32,
    pub padding_x_px: u32,
    pub padding_y_px: u32,
    pub radius_px: u32,
    pub bg_rgba: [u8; 4],
    pub fg_rgba: [u8; 4],
    pub font_family: CaptionFontFamily,
}
pub fn compute_caption_layout(profile: &CaptionProfile, dims: VideoDims) -> CaptionLayout;
```

Project file format bump to `"1.1.0"`. Older `"1.0.0"` loads successfully via `#[serde(default)]` on the new field; next save bumps version.
