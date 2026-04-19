# Blueprint: caption-profiles-persistence

## Single source of truth (non-negotiable)

Caption layout is a classic dual-path bug surface (preview looked right, export was shifted). This feature institutionalizes the fix:

```
                    ┌──────────────────────────────┐
                    │  managers::captions::layout  │
                    │    compute_caption_layout    │
                    └──────────────┬───────────────┘
                                   │
                    ┌──────────────┴──────────────┐
                    ▼                             ▼
       get_caption_layout (Tauri cmd)    export::ass::compose_style
                    │                             │
                    ▼                             ▼
            React preview CSS              libass .ass file
```

Both consumers must import `CaptionLayout` byte-identically. The `preview_and_export_layouts_are_byte_identical` test is a CI gate.

## Default profile values

```rust
pub fn default_desktop_profile() -> CaptionProfile {
    CaptionProfile {
        font_size: 40,           // matches current flat default
        bg_color: "#000000B3".into(),
        text_color: "#FFFFFF".into(),
        position: 90,            // anchor near bottom of frame
        font_family: CaptionFontFamily::default(),
        radius_px: 0,
        padding_x_px: 12,
        padding_y_px: 4,
        max_width_percent: 90,
    }
}

pub fn default_mobile_profile() -> CaptionProfile {
    CaptionProfile {
        font_size: 48,           // bigger text on narrow screens
        bg_color: "#000000B3".into(),
        text_color: "#FFFFFF".into(),
        position: 80,            // higher anchor; thumbs are at the bottom
        font_family: CaptionFontFamily::default(),
        radius_px: 8,            // modern mobile aesthetic
        padding_x_px: 14,
        padding_y_px: 6,
        max_width_percent: 80,
    }
}
```

These values give AC-001-b a real differential (position + max_width_percent + radius + padding + font_size differ). Desktop matches current flat defaults so migration is a no-op for existing users.

## Migration strategy

```rust
pub fn ensure_caption_defaults(settings: &mut AppSettings) -> bool {
    let mut changed = false;

    // If caption_profiles is present, we're done (migration already ran).
    if settings.caption_profiles_was_migrated {
        return false;
    }

    let flat = CaptionProfile {
        font_size: settings.caption_font_size,
        bg_color: settings.caption_bg_color.clone(),
        text_color: settings.caption_text_color.clone(),
        position: settings.caption_position,
        font_family: settings.caption_font_family,
        radius_px: settings.caption_radius_px,
        padding_x_px: settings.caption_padding_x_px,
        padding_y_px: settings.caption_padding_y_px,
        max_width_percent: settings.caption_max_width_percent,
    };

    settings.caption_profiles = CaptionProfileSet {
        desktop: flat.clone(),
        mobile: flat,  // same on first migration; user tweaks mobile later
    };
    settings.caption_profiles_was_migrated = true;
    changed = true;
    changed
}
```

Flat fields stay on the struct for one release cycle with `#[deprecated]` Rustdoc, then removed in a follow-up. They continue to deserialize via `#[serde(default = "default_caption_font_size")]` — missing-on-disk is fine.

## Project file v1.0.0 compat

```rust
const PROJECT_VERSION: &str = "1.1.0"; // bumped
```

`ProjectSettings.caption_profiles: Option<CaptionProfileSet>` with `#[serde(default)]`. A v1.0.0 file has no field → deserializes as `None` → editor falls back to AppSettings at runtime. On next save, version becomes "1.1.0" and caption_profiles is populated from the current app-level values (so the project crystallizes its style).

Cross-version load test: `features/caption-profiles-persistence/fixtures/project_v1_0_0.toaster` is a committed fixture with no caption_profiles. Load → edit → save round-trip test asserts the resulting file has version 1.1.0 + profiles present.

## `compute_caption_layout` contract

```rust
pub fn compute_caption_layout(profile: &CaptionProfile, dims: VideoDims) -> CaptionLayout {
    // 1. Scale font_size by the short-axis dimension so preview and export agree.
    let short_axis = dims.width.min(dims.height);
    let scale = short_axis as f64 / 1080.0;
    let font_size_px = ((profile.font_size as f64) * scale).round() as u32;

    // 2. Vertical anchor: profile.position is % of short axis from top.
    let margin_v_px = ((dims.height as f64) * (profile.position as f64 / 100.0)).round() as u32;

    // 3. Max box width.
    let box_width_px = ((dims.width as f64) * (profile.max_width_percent as f64 / 100.0)).round() as u32;
    let margin_h_px = (dims.width - box_width_px) / 2;

    // 4. Colors parsed once here; both paths use the u8 quad.
    let bg_rgba = parse_hex_rgba(&profile.bg_color).unwrap_or([0, 0, 0, 0xB3]);
    let fg_rgba = parse_hex_rgba(&profile.text_color).unwrap_or([0xFF; 4]);

    CaptionLayout {
        margin_v_px, margin_h_px, box_width_px,
        font_size_px,
        padding_x_px: profile.padding_x_px,
        padding_y_px: profile.padding_y_px,
        radius_px: profile.radius_px,
        bg_rgba, fg_rgba,
        font_family: profile.font_family,
    }
}
```

Golden fixture test feeds `(desktop_default, 1920x1080)` and `(mobile_default, 1080x1920)` and compares against committed JSON. Drift in the math = loud test failure.

## Preview integration

`CaptionMockFrame` (Slice A) grows a prop `layout?: CaptionLayout` populated via a `useCaptionLayout(orientation, videoDims)` hook that calls `get_caption_layout`. Without `layout`, it falls back to schematic rendering (for the Settings-page mock). With `layout`, it positions the caption box using the backend-computed margins.

## Editor radio

`src/components/editor/CaptionOrientationRadio.tsx` — three-way radio: Desktop / Mobile / Auto. The hook derives `effective_orientation` based on video_dims when Auto. The selection is React state, not persisted. Profile is what's saved.

## Settings UI split

`CaptionSettings.tsx` becomes a thin shell with a tab control. Children:
- `CaptionDesktopTab.tsx` — uses `CaptionProfile` for `desktop`.
- `CaptionMobileTab.tsx` — same for `mobile`.
- `CaptionProfileShared.tsx` — the actual control surface (font, colors, position, etc.) parameterized by `{ profile, onChange }`.

`CaptionProfileShared.tsx` carries ~400 LOC (controls + CaptionMockFrame preview). Each tab is <50 LOC. Total surface stays well under the 800-line cap.

## Scope of test updates

Existing cleanup + audio-boundary tests do not touch caption layout, so they pass unchanged. The `transcript-precision-eval` and `audio-boundary-eval` skills are sanity checks against regressions in the export path once the SSOT layout function is wired in.

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| Migration double-runs and overwrites user-tweaked mobile profile with desktop values | low | medium | `caption_profiles_was_migrated` bool latch; test AC-002-b. |
| Dropping flat fields prematurely breaks users on stale installs | low | high | Retain flat fields + `#[deprecated]` for one release; removal is a follow-up feature. |
| Preview↔export byte-identical test is flaky due to floating-point rounding | low | high | All math in `compute_caption_layout` uses `round()` on f64→u32 at the final step; golden JSON is committed with u32 values so there's no FP comparison at test time. |
| v1.0.0 projects load but lose caption data on first save | low | high | `ensure_caption_defaults` runs at project-load as well as settings-load; project's `None` caption_profiles gets seeded with the migrated app profiles at load time (optional) or left `None` and resolved at read time via `get_caption_profile` fallback (chosen: less intrusive). |
| File-size cap breach in settings/captions | medium | low | Split into 3 files at the start (don't grow CaptionSettings.tsx). |
| Specta binding drift until next full `cargo tauri dev` | medium | low | Hand-patch `src/bindings.ts` to stay tsc-green between rebuilds; document in journal. |
| Editor radio UX confusion (Auto vs manual override not persisting) | medium | low | Label radio clearly; include one-line description per the Settings UI contract. |
| Square-aspect videos (1:1) tiebreak | low | low | Documented: 1.0 → desktop. |

## Implementation order

1. Add `CaptionProfile`, `CaptionProfileSet`, default helpers (R-001).
2. Add fields to `AppSettings`; write migration; cargo tests R-002.
3. Add optional field to `ProjectSettings`; bump `PROJECT_VERSION`; cargo tests R-003 + R-008.
4. Create `managers/captions/layout.rs` with `compute_caption_layout` + golden fixture; cargo tests R-004-a.
5. Rewire export path (libass composer) to call `compute_caption_layout`; existing export tests must stay green.
6. Add Tauri commands R-005; tests for scope/fallback.
7. Preview→export byte-identical test (R-004-b) — the SSOT gate.
8. Frontend: split CaptionSettings.tsx, add hook, add editor radio.
9. i18n across 20 locales.
10. Static gates + live QC.
