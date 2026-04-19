use crate::commands::editor::EditorStore;
use crate::managers::captions::{
    self, CaptionBlock as LayoutBlock, CaptionLayoutConfig, FontRegistry, TimelineDomain,
};
use crate::managers::export::{self, CaptionSegment, ExportConfig, ExportFormat};
use crate::managers::media::MediaStore;
use once_cell::sync::OnceCell;
use tauri::{AppHandle, State};

/// Parsed bundled fonts, built once per process. Parsing is ~milliseconds
/// but gives `build_caption_blocks` an `O(1)` call path.
fn fonts() -> &'static FontRegistry {
    static CELL: OnceCell<FontRegistry> = OnceCell::new();
    CELL.get_or_init(|| {
        FontRegistry::new().expect("bundled caption fonts must parse at startup")
    })
}

#[tauri::command]
#[specta::specta]
pub fn export_transcript(
    store: State<EditorStore>,
    format: ExportFormat,
    max_chars_per_line: Option<usize>,
    include_silenced: Option<bool>,
) -> Result<String, String> {
    let state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
    let words = state.get_words();
    let config = ExportConfig {
        max_chars_per_line: max_chars_per_line.unwrap_or(42),
        include_silenced: include_silenced.unwrap_or(false),
        ..Default::default()
    };
    Ok(export::export(words, format, &config))
}

#[tauri::command]
#[specta::specta]
pub fn export_transcript_to_file(
    store: State<EditorStore>,
    format: ExportFormat,
    path: String,
    max_chars_per_line: Option<usize>,
    include_silenced: Option<bool>,
) -> Result<(), String> {
    let state = crate::lock_recovery::try_lock(store.0.lock()).map_err(|e| e.to_string())?;
    let words = state.get_words();
    let config = ExportConfig {
        max_chars_per_line: max_chars_per_line.unwrap_or(42),
        include_silenced: include_silenced.unwrap_or(false),
        ..Default::default()
    };
    export::export_to_file(words, format, &config, std::path::Path::new(&path))
}

/// Return all caption segments with their time ranges.
///
/// Kept for callers that only need SRT/VTT-style text segments (one line
/// per segment, no geometry). The preview + export caption rendering path
/// uses `get_caption_blocks` which carries per-line wrap and pixel
/// geometry authoritative for both surfaces.
#[tauri::command]
#[specta::specta]
pub fn get_caption_segments(store: State<EditorStore>) -> Vec<CaptionSegment> {
    let state = crate::lock_recovery::recover_lock(store.0.lock());
    let words = state.get_words();
    let config = ExportConfig::default();
    export::build_segments(words, &config)
}

/// Compute laid-out caption blocks consumed verbatim by the live preview.
///
/// The blocks carry per-line wrapped text plus every geometry value in
/// video pixels, so the preview scales them by `rendered / frame_height`
/// and renders a visual match of the export. Pass
/// `TimelineDomain::Source` for preview over the un-edited video;
/// `TimelineDomain::Edited` remaps to the concatenated output clock.
#[tauri::command]
#[specta::specta]
pub fn get_caption_blocks(
    app: AppHandle,
    store: State<EditorStore>,
    media: State<MediaStore>,
    domain: TimelineDomain,
) -> Vec<LayoutBlock> {
    let state = crate::lock_recovery::recover_lock(store.0.lock());
    let media_state = crate::lock_recovery::recover_lock(media.0.lock());
    let frame_size = media_state
        .current()
        .and_then(|m| probe_video_dimensions_cached(&m.path.to_string_lossy()))
        .unwrap_or((1920, 1080));
    drop(media_state);

    let settings = crate::settings::get_settings(&app);
    let config = layout_config_from_settings(&settings, frame_size);

    let keep_segments: Vec<(i64, i64)> = state.get_keep_segments();
    captions::build_blocks(state.get_words(), &keep_segments, &config, fonts(), domain)
}

fn probe_video_dimensions_cached(path: &str) -> Option<(u32, u32)> {
    crate::commands::waveform::probe_video_dimensions(path)
}

/// Pick the effective caption profile for the given orientation,
/// preferring project-level overrides over the app default. Slice B's
/// `get_caption_profile` command calls the same logic via
/// `commands::captions`.
pub fn effective_profile(
    settings: &crate::settings::AppSettings,
    project_profiles: Option<&crate::settings::CaptionProfileSet>,
    orientation: crate::settings::Orientation,
) -> crate::settings::CaptionProfile {
    let set = project_profiles.unwrap_or(&settings.caption_profiles);
    match orientation {
        crate::settings::Orientation::Desktop => set.desktop.clone(),
        crate::settings::Orientation::Mobile => set.mobile.clone(),
    }
}

/// Build a layout config from the active `AppSettings` and the probed
/// video frame size. Extracted so `commands::waveform` can share it with
/// the export path when the caption-block pipeline is wired in.
///
/// Goes through `compute_caption_layout` via
/// `CaptionLayoutConfig::from_profile` so preview and export agree
/// byte-for-byte (Slice B SSOT).
pub fn layout_config_from_settings(
    settings: &crate::settings::AppSettings,
    (frame_width, frame_height): (u32, u32),
) -> CaptionLayoutConfig {
    let orientation = if frame_width >= frame_height {
        crate::settings::Orientation::Desktop
    } else {
        crate::settings::Orientation::Mobile
    };
    let profile = effective_profile(settings, None, orientation);
    CaptionLayoutConfig::from_profile(
        &profile,
        crate::settings::VideoDims {
            width: frame_width,
            height: frame_height,
        },
    )
}

/// Build caption blocks without going through Tauri state — used by
/// `commands::waveform` during export. Accepts explicit words +
/// keep-segments so callers can pass the **canonical** segments used by
/// the FFmpeg concat (which may differ from `editor.get_keep_segments()`
/// when the experimental simplify mode is on). Pass frame dimensions
/// from `ffprobe` on the real input file.
pub fn build_caption_blocks_for_export(
    words: &[crate::managers::editor::Word],
    keep_segments: &[(i64, i64)],
    settings: &crate::settings::AppSettings,
    frame_size: (u32, u32),
) -> Vec<LayoutBlock> {
    let config = layout_config_from_settings(settings, frame_size);
    captions::build_blocks(
        words,
        keep_segments,
        &config,
        fonts(),
        TimelineDomain::Edited,
    )
}

/// Resolve the bundled fonts directory on disk so FFmpeg's libass can
/// find Inter/Roboto via `fontsdir=`. Returns `None` in dev builds
/// where the Tauri resource bundle isn't staged; callers fall back to
/// fontconfig (system-installed fonts).
pub fn bundled_fonts_dir(app: &AppHandle) -> Option<std::path::PathBuf> {
    use tauri::Manager;
    app.path()
        .resolve("assets/fonts", tauri::path::BaseDirectory::Resource)
        .ok()
        .filter(|p| p.exists())
        .or_else(|| {
            // Dev-mode fallback: walk up from CARGO_MANIFEST_DIR.
            let candidate = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("fonts");
            candidate.exists().then_some(candidate)
        })
}
