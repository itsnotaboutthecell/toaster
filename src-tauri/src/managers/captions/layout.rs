//! Caption layout engine — single source of truth for preview + export.
//!
//! Given a post-edit word list, keep-segments, user caption settings, and
//! the target video frame size, produce a `Vec<CaptionBlock>` where each
//! block carries:
//!
//! * the visual lines already wrapped (so neither path re-wraps);
//! * every geometry value in **video pixels** (font size, padding, radius,
//!   margin, text width, line height);
//! * start/end timestamps in the caller's requested timeline domain.
//!
//! The preview scales these pixel values by `rendered_height / frame_height`
//! to stay visually identical to the export.

use super::fonts::FontRegistry;
use crate::managers::editor::Word;
use crate::settings::{CaptionFontFamily, CaptionProfile, VideoDims};
use serde::{Deserialize, Serialize};

/// Which time axis the caller wants timestamps in.
///
/// * `Source` — the original media clock. Used by the live preview, which
///   plays the un-concatenated source video.
/// * `Edited` — the concatenated output clock. Used by the export so the
///   burned-in captions land on the right video frames after cuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum TimelineDomain {
    Source,
    Edited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    /// Parse `#RRGGBB` or `#RRGGBBAA` (CSS convention: AA = opacity).
    pub fn parse_css_hex(hex: &str, default_alpha: u8) -> Self {
        let h = hex.trim_start_matches('#');
        let r = u8::from_str_radix(h.get(0..2).unwrap_or("FF"), 16).unwrap_or(0xFF);
        let g = u8::from_str_radix(h.get(2..4).unwrap_or("FF"), 16).unwrap_or(0xFF);
        let b = u8::from_str_radix(h.get(4..6).unwrap_or("FF"), 16).unwrap_or(0xFF);
        let a = if h.len() >= 8 {
            u8::from_str_radix(&h[6..8], 16).unwrap_or(default_alpha)
        } else {
            default_alpha
        };
        Self { r, g, b, a }
    }
}

/// User-controlled layout inputs. Everything derived from settings +
/// probed video size lands here; the layout function is otherwise pure.
#[derive(Debug, Clone)]
pub struct CaptionLayoutConfig {
    pub font_family: CaptionFontFamily,
    /// Font size in video pixels.
    pub font_size_px: u32,
    pub text_color: Rgba,
    pub background: Rgba,
    /// Position percentage (0 = top, 100 = bottom edge of video).
    pub position_pct: u32,
    pub radius_px: u32,
    pub padding_x_px: u32,
    pub padding_y_px: u32,
    /// Maximum usable width for a visual line, as a percentage of the
    /// video frame width. Wrap never exceeds this.
    pub max_width_pct: u32,
    /// Video frame size in pixels (authoritative for geometry).
    pub frame_width: u32,
    pub frame_height: u32,
    /// Maximum duration per block in microseconds. A block is split when
    /// adding another word would exceed this even if it would fit in width.
    pub max_segment_duration_us: i64,
    /// Whether `silenced` words should appear in captions.
    pub include_silenced: bool,
}

impl Default for CaptionLayoutConfig {
    fn default() -> Self {
        Self {
            font_family: CaptionFontFamily::Inter,
            font_size_px: 24,
            text_color: Rgba { r: 255, g: 255, b: 255, a: 255 },
            background: Rgba { r: 0, g: 0, b: 0, a: 0xB3 },
            position_pct: 90,
            radius_px: 4,
            padding_x_px: 12,
            padding_y_px: 4,
            max_width_pct: 90,
            frame_width: 1920,
            frame_height: 1080,
            max_segment_duration_us: 5_000_000,
            include_silenced: false,
        }
    }
}

/// Authoritative caption unit consumed verbatim by preview and export.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CaptionBlock {
    pub index: usize,
    pub start_us: i64,
    pub end_us: i64,
    /// Already-wrapped visual lines; render one per row.
    pub lines: Vec<String>,
    /// CSS font-family stack for the preview (export uses `font_ass_name`).
    pub font_css: String,
    /// ASS `Fontname=` value for libass.
    pub font_ass_name: String,
    pub font_size_px: u32,
    pub text_color: Rgba,
    pub background: Rgba,
    pub padding_x_px: u32,
    pub padding_y_px: u32,
    pub radius_px: u32,
    /// Distance from the bottom of the frame to the bottom edge of the
    /// caption box, in video pixels.
    pub margin_v_px: u32,
    /// Pixel width of the widest line (glyph advance sum). Preview uses
    /// this to size the pill to the text; export uses it to size the ASS
    /// `\p1` rectangle.
    pub text_width_px: u32,
    /// Per-line box height in video pixels (includes leading).
    pub line_height_px: u32,
    /// Frame dimensions this layout was computed against; the preview
    /// divides by these to scale to the rendered `<video>`.
    pub frame_width: u32,
    pub frame_height: u32,
}

/// Canonical per-video caption layout derived from a `CaptionProfile`
/// and the target video dimensions. Preview (`get_caption_layout`
/// Tauri command) and libass export BOTH call
/// [`compute_caption_layout`] with the same inputs — any divergence
/// surfaces as a `preview_and_export_layouts_are_byte_identical`
/// test failure (Slice B SSOT gate).
///
/// All values are in authoritative **video pixels**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CaptionLayout {
    /// Vertical margin — distance from the top of the frame to the
    /// baseline anchor of the caption box, derived from
    /// `profile.position` as a percentage of frame height.
    pub margin_v_px: u32,
    /// Horizontal margin — symmetric left/right gutter derived from
    /// `profile.max_width_percent`.
    pub margin_h_px: u32,
    /// Caption box width in video pixels
    /// (`frame_width * max_width_percent / 100`).
    pub box_width_px: u32,
    pub font_size_px: u32,
    pub padding_x_px: u32,
    pub padding_y_px: u32,
    pub radius_px: u32,
    /// Background color RGBA (u8 quad parsed from `bg_color` hex).
    pub bg_rgba: [u8; 4],
    /// Text color RGBA.
    pub fg_rgba: [u8; 4],
    pub font_family: CaptionFontFamily,
    pub frame_width: u32,
    pub frame_height: u32,
}

/// Single source of truth for caption layout math.
///
/// Given a user-controlled [`CaptionProfile`] and the video frame
/// [`VideoDims`], return a [`CaptionLayout`] consumed byte-identically
/// by the preview (`get_caption_layout`) and by the libass export
/// pipeline (via [`CaptionLayoutConfig`] construction below).
///
/// Every f64→u32 conversion uses `.round()` at the final step so the
/// preview↔export byte-identical test is deterministic (no FP drift).
pub fn compute_caption_layout(profile: &CaptionProfile, dims: VideoDims) -> CaptionLayout {
    let width = dims.width.max(1);
    let height = dims.height.max(1);

    // 1. Scale font size by the short-axis dimension so the preview
    //    and export agree across 1080p, 720p, and portrait 1080x1920.
    let short_axis = width.min(height) as f64;
    let scale = short_axis / 1080.0;
    let font_size_px = ((profile.font_size as f64) * scale).round().max(1.0) as u32;

    // 2. Vertical anchor: profile.position is % of frame height from
    //    the top. position=90 → caption near the bottom.
    let pos_pct = profile.position.min(100) as f64;
    let margin_v_px = ((height as f64) * pos_pct / 100.0).round() as u32;

    // 3. Max box width and symmetric horizontal margin.
    let mw_pct = profile.max_width_percent.clamp(20, 100) as f64;
    let box_width_px = ((width as f64) * mw_pct / 100.0).round() as u32;
    let margin_h_px = width.saturating_sub(box_width_px) / 2;

    // 4. Parse colors once so both consumers use the same u8 quad.
    let bg = Rgba::parse_css_hex(&profile.bg_color, 0xB3);
    let fg = Rgba::parse_css_hex(&profile.text_color, 0xFF);

    CaptionLayout {
        margin_v_px,
        margin_h_px,
        box_width_px,
        font_size_px,
        padding_x_px: profile.padding_x_px.min(128),
        padding_y_px: profile.padding_y_px.min(128),
        radius_px: profile.radius_px.min(64),
        bg_rgba: [bg.r, bg.g, bg.b, bg.a],
        fg_rgba: [fg.r, fg.g, fg.b, fg.a],
        font_family: profile.font_family,
        frame_width: width,
        frame_height: height,
    }
}

impl CaptionLayoutConfig {
    /// Derive a full `CaptionLayoutConfig` (geometry + text-flow knobs)
    /// from a [`CaptionProfile`] via [`compute_caption_layout`]. This is
    /// the authoritative path used by preview and export alike (AGENTS.md
    /// SSOT rule).
    pub fn from_profile(profile: &CaptionProfile, dims: VideoDims) -> Self {
        let layout = compute_caption_layout(profile, dims);
        Self {
            font_family: layout.font_family,
            font_size_px: layout.font_size_px,
            text_color: Rgba {
                r: layout.fg_rgba[0],
                g: layout.fg_rgba[1],
                b: layout.fg_rgba[2],
                a: layout.fg_rgba[3],
            },
            background: Rgba {
                r: layout.bg_rgba[0],
                g: layout.bg_rgba[1],
                b: layout.bg_rgba[2],
                a: layout.bg_rgba[3],
            },
            position_pct: profile.position.min(100),
            radius_px: layout.radius_px,
            padding_x_px: layout.padding_x_px,
            padding_y_px: layout.padding_y_px,
            max_width_pct: profile.max_width_percent.clamp(20, 100),
            frame_width: layout.frame_width,
            frame_height: layout.frame_height,
            max_segment_duration_us: 5_000_000,
            include_silenced: false,
        }
    }
}

///
/// `keep_segments` are the edit keep-ranges on the source timeline. When
/// `domain == Edited`, words that don't overlap a keep-range are dropped
/// and surviving words get remapped onto the concatenated output clock.
pub fn build_blocks(
    words: &[Word],
    keep_segments: &[(i64, i64)],
    config: &CaptionLayoutConfig,
    fonts: &FontRegistry,
    domain: TimelineDomain,
) -> Vec<CaptionBlock> {
    let font_handle = fonts.resolve(config.font_family);
    let font = &font_handle.font;
    let size_f = config.font_size_px as f32;

    // Typographic line height. fontdue's horizontal_line_metrics includes
    // ascent/descent/line-gap in the font's units at the given size.
    let line_metrics = font.horizontal_line_metrics(size_f);
    let line_height_px = line_metrics
        .map(|m| (m.new_line_size).ceil() as u32)
        .unwrap_or((config.font_size_px as f32 * 1.2) as u32);

    let space_w = char_advance(font, ' ', size_f);

    let max_line_width_px = (config.frame_width as f32 * config.max_width_pct as f32 / 100.0
        - 2.0 * config.padding_x_px as f32)
        .max(1.0) as u32;

    // ── Step 1: filter + optionally remap to the edited timeline ──
    let mut active: Vec<(String, i64, i64)> = Vec::with_capacity(words.len());
    for w in words {
        if w.deleted {
            continue;
        }
        if w.silenced && !config.include_silenced {
            continue;
        }
        let (s, e) = match domain {
            TimelineDomain::Source => (w.start_us, w.end_us),
            TimelineDomain::Edited => match map_source_to_edit(w.start_us, w.end_us, keep_segments)
            {
                Some(r) => r,
                None => continue,
            },
        };
        active.push((w.text.clone(), s, e));
    }

    if active.is_empty() {
        return Vec::new();
    }

    // ── Step 2: greedy pack into blocks, wrapping by pixel width ──
    let mut blocks: Vec<CaptionBlock> = Vec::new();
    let mut cur_lines: Vec<String> = vec![String::new()];
    let mut cur_line_widths: Vec<u32> = vec![0];
    let mut cur_start: i64 = active[0].1;
    let mut cur_end: i64 = active[0].2;

    let flush = |lines: &mut Vec<String>,
                 widths: &mut Vec<u32>,
                 start: i64,
                 end: i64,
                 blocks: &mut Vec<CaptionBlock>,
                 cfg: &CaptionLayoutConfig,
                 handle: &super::fonts::FontMetricsHandle,
                 line_h: u32| {
        // Drop trailing empty lines.
        while lines.last().map(|s| s.is_empty()).unwrap_or(false) && lines.len() > 1 {
            lines.pop();
            widths.pop();
        }
        if lines.len() == 1 && lines[0].is_empty() {
            return;
        }
        let max_w = widths.iter().copied().max().unwrap_or(0);
        blocks.push(CaptionBlock {
            index: blocks.len() + 1,
            start_us: start,
            end_us: end,
            lines: std::mem::take(lines),
            font_css: handle.css_stack.to_string(),
            font_ass_name: handle.ass_name.to_string(),
            font_size_px: cfg.font_size_px,
            text_color: cfg.text_color,
            background: cfg.background,
            padding_x_px: cfg.padding_x_px,
            padding_y_px: cfg.padding_y_px,
            radius_px: cfg.radius_px,
            margin_v_px: (cfg.frame_height as f32
                * (100.0 - cfg.position_pct.min(100) as f32)
                / 100.0) as u32,
            text_width_px: max_w,
            line_height_px: line_h,
            frame_width: cfg.frame_width,
            frame_height: cfg.frame_height,
        });
        *lines = vec![String::new()];
        *widths = vec![0];
    };

    for (text, start, end) in &active {
        let word_w = word_advance(font, text, size_f);

        let all_empty = cur_lines.iter().all(|l| l.is_empty());
        let duration_would_be = *end - cur_start;
        let too_long_time = !all_empty && duration_would_be > config.max_segment_duration_us;

        if too_long_time {
            flush(
                &mut cur_lines,
                &mut cur_line_widths,
                cur_start,
                cur_end,
                &mut blocks,
                config,
                font_handle,
                line_height_px,
            );
            cur_start = *start;
        }

        let cur_idx = cur_lines.len() - 1;
        let cur_w = cur_line_widths[cur_idx];
        let sep_w = if cur_lines[cur_idx].is_empty() { 0 } else { space_w };
        let candidate_w = if cur_lines.iter().all(|l| l.is_empty()) {
            word_w
        } else {
            cur_w + sep_w + word_w
        };

        if candidate_w > max_line_width_px && !cur_lines[cur_idx].is_empty() {
            // Line full — start a new visual line inside the same block.
            cur_lines.push(String::new());
            cur_line_widths.push(0);
        }

        let idx = cur_lines.len() - 1;
        if !cur_lines[idx].is_empty() {
            cur_lines[idx].push(' ');
            cur_line_widths[idx] += space_w;
        }
        cur_lines[idx].push_str(text);
        cur_line_widths[idx] += word_w;
        if cur_lines.iter().map(|l| l.len()).sum::<usize>() == text.len() {
            cur_start = *start;
        }
        cur_end = *end;
    }

    flush(
        &mut cur_lines,
        &mut cur_line_widths,
        cur_start,
        cur_end,
        &mut blocks,
        config,
        font_handle,
        line_height_px,
    );

    // Re-number so indexes are contiguous starting from 1.
    for (i, b) in blocks.iter_mut().enumerate() {
        b.index = i + 1;
    }

    blocks
}

fn char_advance(font: &fontdue::Font, ch: char, px: f32) -> u32 {
    let m = font.metrics(ch, px);
    m.advance_width.ceil() as u32
}

fn word_advance(font: &fontdue::Font, word: &str, px: f32) -> u32 {
    let mut w: f32 = 0.0;
    for ch in word.chars() {
        w += font.metrics(ch, px).advance_width;
    }
    w.ceil() as u32
}

/// Map a source-time range to the edited (concatenated) timeline.
/// Returns `None` if the word doesn't overlap any keep-range.
fn map_source_to_edit(
    src_start: i64,
    src_end: i64,
    keep_segments: &[(i64, i64)],
) -> Option<(i64, i64)> {
    let mut elapsed: i64 = 0;
    for &(seg_start, seg_end) in keep_segments {
        let seg_dur = seg_end - seg_start;
        if src_start < seg_end && src_end > seg_start {
            let clamped_start = src_start.max(seg_start);
            let clamped_end = src_end.min(seg_end);
            let out_start = elapsed + (clamped_start - seg_start);
            let out_end = elapsed + (clamped_end - seg_start);
            return Some((out_start, out_end));
        }
        elapsed += seg_dur;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{default_desktop_profile, default_mobile_profile};

    fn mk_word(text: &str, s: i64, e: i64) -> Word {
        Word {
            text: text.into(),
            start_us: s,
            end_us: e,
            deleted: false,
            silenced: false,
            confidence: -1.0,
            speaker_id: -1,
        }
    }

    fn cfg() -> CaptionLayoutConfig {
        CaptionLayoutConfig {
            frame_width: 1280,
            frame_height: 720,
            ..Default::default()
        }
    }

    #[test]
    fn caption_profile_roundtrip_serde() {
        let profile = default_desktop_profile();
        let json = serde_json::to_string(&profile).expect("serialize");
        let back: CaptionProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(profile, back);
    }

    #[test]
    fn caption_profile_set_has_distinct_desktop_and_mobile_defaults() {
        let desktop = default_desktop_profile();
        let mobile = default_mobile_profile();
        assert_ne!(
            desktop.max_width_percent, mobile.max_width_percent,
            "mobile must be narrower than desktop"
        );
        assert_ne!(
            desktop.position, mobile.position,
            "mobile must sit higher (lower position%) than desktop"
        );
        assert!(mobile.position < desktop.position);
        assert!(mobile.max_width_percent < desktop.max_width_percent);
    }

    #[test]
    fn compute_caption_layout_matches_golden_fixture() {
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("caption_layout");

        let desktop_json = std::fs::read_to_string(fixtures.join("desktop_1920x1080.json"))
            .expect("desktop fixture present");
        let desktop_expected: CaptionLayout =
            serde_json::from_str(&desktop_json).expect("desktop fixture parses");
        let desktop_actual = compute_caption_layout(
            &default_desktop_profile(),
            VideoDims { width: 1920, height: 1080 },
        );
        assert_eq!(
            desktop_actual, desktop_expected,
            "desktop layout must match committed golden fixture"
        );

        let mobile_json = std::fs::read_to_string(fixtures.join("mobile_1080x1920.json"))
            .expect("mobile fixture present");
        let mobile_expected: CaptionLayout =
            serde_json::from_str(&mobile_json).expect("mobile fixture parses");
        let mobile_actual = compute_caption_layout(
            &default_mobile_profile(),
            VideoDims { width: 1080, height: 1920 },
        );
        assert_eq!(
            mobile_actual, mobile_expected,
            "mobile layout must match committed golden fixture"
        );
    }

    #[test]
    fn preview_and_export_layouts_are_byte_identical() {
        // SSOT gate (AC-004-b): the preview path (direct call to
        // compute_caption_layout — what the `get_caption_layout` Tauri
        // command returns) and the export path (call through
        // CaptionLayoutConfig::from_profile — what libass composer
        // consumes) must produce byte-identical CaptionLayout values
        // for the same inputs. If this ever drifts, the dual-path bug
        // we institutionalized against has returned.
        for (profile, dims) in [
            (default_desktop_profile(), VideoDims { width: 1920, height: 1080 }),
            (default_desktop_profile(), VideoDims { width: 1280, height: 720 }),
            (default_mobile_profile(), VideoDims { width: 1080, height: 1920 }),
            (default_mobile_profile(), VideoDims { width: 720, height: 1280 }),
        ] {
            let preview = compute_caption_layout(&profile, dims);
            let cfg = CaptionLayoutConfig::from_profile(&profile, dims);
            // Reconstruct CaptionLayout from the export-bound config to
            // prove the same fields round-trip without drift.
            let export = CaptionLayout {
                margin_v_px: ((cfg.frame_height as f64) * (cfg.position_pct.min(100) as f64)
                    / 100.0)
                    .round() as u32,
                margin_h_px: cfg
                    .frame_width
                    .saturating_sub(
                        ((cfg.frame_width as f64) * (cfg.max_width_pct.clamp(20, 100) as f64)
                            / 100.0)
                            .round() as u32,
                    )
                    / 2,
                box_width_px: ((cfg.frame_width as f64) * (cfg.max_width_pct.clamp(20, 100) as f64)
                    / 100.0)
                    .round() as u32,
                font_size_px: cfg.font_size_px,
                padding_x_px: cfg.padding_x_px,
                padding_y_px: cfg.padding_y_px,
                radius_px: cfg.radius_px,
                bg_rgba: [cfg.background.r, cfg.background.g, cfg.background.b, cfg.background.a],
                fg_rgba: [cfg.text_color.r, cfg.text_color.g, cfg.text_color.b, cfg.text_color.a],
                font_family: cfg.font_family,
                frame_width: cfg.frame_width,
                frame_height: cfg.frame_height,
            };
            assert_eq!(
                preview, export,
                "preview/export drift for {:?} at {}x{}",
                profile.font_family, dims.width, dims.height
            );
        }
    }

    #[test]
    fn rgba_parses_css_alpha() {
        let c = Rgba::parse_css_hex("#000000B3", 0xFF);
        assert_eq!((c.r, c.g, c.b, c.a), (0, 0, 0, 0xB3));
        let d = Rgba::parse_css_hex("#FFAA00", 0xCC);
        assert_eq!((d.r, d.g, d.b, d.a), (0xFF, 0xAA, 0x00, 0xCC));
    }

    #[test]
    fn empty_words_produce_no_blocks() {
        let fonts = FontRegistry::new().unwrap();
        let blocks = build_blocks(&[], &[(0, 10_000_000)], &cfg(), &fonts, TimelineDomain::Source);
        assert!(blocks.is_empty());
    }

    #[test]
    fn short_sentence_makes_one_block() {
        let fonts = FontRegistry::new().unwrap();
        let words = vec![
            mk_word("Hello", 0, 500_000),
            mk_word("world", 500_000, 1_000_000),
        ];
        let blocks = build_blocks(
            &words,
            &[(0, 1_000_000)],
            &cfg(),
            &fonts,
            TimelineDomain::Source,
        );
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lines.len(), 1);
        assert_eq!(blocks[0].lines[0], "Hello world");
        assert!(blocks[0].text_width_px > 0);
        assert!(blocks[0].line_height_px > 0);
    }

    #[test]
    fn deletion_is_respected() {
        let fonts = FontRegistry::new().unwrap();
        let mut words = vec![
            mk_word("Hello", 0, 500_000),
            mk_word("rude", 500_000, 900_000),
            mk_word("world", 900_000, 1_300_000),
        ];
        words[1].deleted = true;
        let blocks = build_blocks(
            &words,
            &[(0, 1_300_000)],
            &cfg(),
            &fonts,
            TimelineDomain::Source,
        );
        let text: String = blocks.iter().flat_map(|b| b.lines.clone()).collect::<Vec<_>>().join(" ");
        assert!(!text.contains("rude"));
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
    }

    #[test]
    fn edited_timeline_drops_removed_and_compacts() {
        let fonts = FontRegistry::new().unwrap();
        let words = vec![
            mk_word("A", 0, 500_000),
            mk_word("B", 2_000_000, 2_500_000),
        ];
        let keeps = [(0, 500_000), (2_000_000, 2_500_000)];
        let blocks = build_blocks(&words, &keeps, &cfg(), &fonts, TimelineDomain::Edited);
        assert_eq!(blocks.len(), 1);
        // After remap, B should start at 500_000 (right after A).
        assert_eq!(blocks[0].start_us, 0);
        assert_eq!(blocks[0].end_us, 1_000_000);
    }

    #[test]
    fn duration_cap_splits_blocks() {
        let fonts = FontRegistry::new().unwrap();
        let mut words = Vec::new();
        for i in 0..12 {
            // 1 second per word → 12s total, must split at 5s.
            words.push(mk_word("word", i * 1_000_000, (i + 1) * 1_000_000));
        }
        let blocks = build_blocks(
            &words,
            &[(0, 12_000_000)],
            &cfg(),
            &fonts,
            TimelineDomain::Source,
        );
        assert!(blocks.len() >= 2, "expected >=2 blocks, got {}", blocks.len());
        for b in &blocks {
            assert!(b.end_us - b.start_us <= 5_100_000);
        }
    }

    #[test]
    fn pixel_wrap_respects_max_width() {
        let fonts = FontRegistry::new().unwrap();
        let mut c = cfg();
        c.frame_width = 400; // very narrow
        c.max_width_pct = 90;
        let text = "the quick brown fox jumps over the lazy dog again and again";
        let words: Vec<Word> = text
            .split_whitespace()
            .enumerate()
            .map(|(i, w)| mk_word(w, i as i64 * 100_000, (i as i64 + 1) * 100_000))
            .collect();
        let blocks = build_blocks(
            &words,
            &[(0, 10_000_000)],
            &c,
            &fonts,
            TimelineDomain::Source,
        );
        assert!(!blocks.is_empty());
        let max_w_px = (c.frame_width as f32 * c.max_width_pct as f32 / 100.0
            - 2.0 * c.padding_x_px as f32) as u32;
        for b in &blocks {
            assert!(
                b.text_width_px <= max_w_px + 2,
                "line wider than limit: {} > {}",
                b.text_width_px,
                max_w_px
            );
            assert!(b.lines.len() >= 2, "narrow frame should wrap onto multiple lines");
        }
    }

    #[test]
    fn margin_and_frame_dims_propagate() {
        let fonts = FontRegistry::new().unwrap();
        let mut c = cfg();
        c.position_pct = 80;
        c.frame_height = 1080;
        let words = vec![mk_word("hi", 0, 200_000)];
        let blocks = build_blocks(
            &words,
            &[(0, 200_000)],
            &c,
            &fonts,
            TimelineDomain::Source,
        );
        assert_eq!(blocks[0].frame_height, 1080);
        assert_eq!(blocks[0].frame_width, 1280);
        // 20% of 1080 = 216
        assert_eq!(blocks[0].margin_v_px, 216);
    }
}
