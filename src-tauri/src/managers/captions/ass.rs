//! ASS (Advanced SubStation Alpha) file emitter.
//!
//! Takes the authoritative `CaptionBlock` stream from `layout` and
//! produces an ASS document that libass (via FFmpeg's `subtitles=` filter)
//! renders to a pixel-correct burn-in.
//!
//! **Box sizing — single source of truth per renderer.** The earlier
//! revision emitted a `\p1` vector-drawn rounded rectangle sized from
//! `fontdue`-measured advance widths, then let libass/FreeType render the
//! text on top. libass glyph advance widths differ from fontdue's
//! (different hinter, kerning, feature tables), so the pill drifted ~15 %
//! wider and text rendered ~20 % narrower than predicted — a ~35 %
//! visible mismatch. This violated AGENTS.md's "single source of truth
//! for dual-path logic" rule because the pill and the text were being
//! sized by two different metric engines.
//!
//! Now the export uses libass's native `BorderStyle=3` (Opaque Box). The
//! box is filled with `OutlineColour` and auto-sizes to the glyphs that
//! libass itself just measured, with `Outline` acting as symmetric
//! padding on all four sides. The preview's CSS `padding` around
//! auto-sized text produces the same geometric contract: the pill hugs
//! whatever glyphs its own renderer drew. Rounded corners were dropped —
//! libass BorderStyle=3 is a hard rectangle, and the preview follows
//! suit. See `managers/captions/layout.rs` for the upstream layout.

use super::layout::{CaptionBlock, Rgba};
use std::fmt::Write;

/// Resolve the padding value used as libass `Outline` for BorderStyle=3.
/// The max of `padding_x_px` and `padding_y_px` ensures the opaque box
/// always has at least the larger of the two configured gutters on every
/// side.
fn box_padding_px(block: &CaptionBlock) -> u32 {
    block.padding_x_px.max(block.padding_y_px)
}

/// Serialize `CaptionBlock`s into a complete ASS document string.
pub fn blocks_to_ass(blocks: &[CaptionBlock]) -> String {
    let (play_w, play_h) = blocks
        .first()
        .map(|b| (b.frame_width, b.frame_height))
        .unwrap_or((1920, 1080));

    // Style values come from the first block — every block in one run
    // shares the same caption settings (font family, size, colors, padding).
    let font_name = blocks
        .first()
        .map(|b| b.font_ass_name.as_str())
        .unwrap_or("Arial");
    let font_size = blocks.first().map(|b| b.font_size_px).unwrap_or(24);
    let text_c = blocks
        .first()
        .map(|b| b.text_color)
        .unwrap_or(Rgba { r: 255, g: 255, b: 255, a: 255 });
    let bg_c = blocks
        .first()
        .map(|b| b.background)
        .unwrap_or(Rgba { r: 0, g: 0, b: 0, a: 0xB3 });
    let padding = blocks.first().map(box_padding_px).unwrap_or(4);

    let primary = ass_color_abgr(text_c);
    let outline = ass_color_abgr(bg_c);

    let mut out = String::new();
    let _ = writeln!(out, "[Script Info]");
    let _ = writeln!(out, "ScriptType: v4.00+");
    let _ = writeln!(out, "WrapStyle: 2");
    let _ = writeln!(out, "ScaledBorderAndShadow: yes");
    let _ = writeln!(out, "PlayResX: {play_w}");
    let _ = writeln!(out, "PlayResY: {play_h}");
    let _ = writeln!(out);

    let _ = writeln!(out, "[V4+ Styles]");
    let _ = writeln!(
        out,
        "Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding"
    );
    // BorderStyle=3 = Opaque Box. libass fills the box with
    // `OutlineColour` (sized to the rendered glyphs + `Outline` px
    // padding on every side). `Alignment=2` = bottom-center; `MarginV`
    // is set per-event below so each block lands at its own position.
    let _ = writeln!(
        out,
        "Style: Default,{font_name},{font_size},{primary},&H000000FF,{outline},&H00000000,0,0,0,0,100,100,0,0,3,{padding},0,2,0,0,0,1"
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "[Events]");
    let _ = writeln!(
        out,
        "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text"
    );
    for block in blocks {
        let start = format_ass_time(block.start_us);
        let end = format_ass_time(block.end_us);
        let joined = block
            .lines
            .iter()
            .map(|l| escape_ass_text(l))
            .collect::<Vec<_>>()
            .join("\\N");
        // MarginV column overrides the style MarginV per-block. With
        // Alignment=2 it's the distance from the bottom of the frame to
        // the baseline of the last line.
        let mv = block.margin_v_px;
        let _ = writeln!(
            out,
            "Dialogue: 0,{start},{end},Default,,0,0,{mv},,{joined}"
        );
    }

    out
}

/// Format `Rgba` as `&HAABBGGRR` (ASS color format with alpha). ASS
/// alpha is inverted relative to CSS: `00` = fully opaque, `FF` = fully
/// transparent.
fn ass_color_abgr(c: Rgba) -> String {
    let a = 255 - c.a;
    format!("&H{:02X}{:02X}{:02X}{:02X}", a, c.b, c.g, c.r)
}

/// Escape an ASS text literal. ASS uses `\` for override tags and
/// `{` / `}` for tag groups; newlines must be expressed as `\N`.
fn escape_ass_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\n' => out.push_str("\\N"),
            c => out.push(c),
        }
    }
    out
}

/// ASS time format: `H:MM:SS.cc` (centiseconds).
fn format_ass_time(us: i64) -> String {
    let total_cs = us.max(0) / 10_000;
    let cs = total_cs % 100;
    let total_s = total_cs / 100;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{h}:{m:02}:{s:02}.{cs:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::captions::{CaptionBlock, Rgba};

    fn mk_block(idx: usize, start_us: i64, end_us: i64) -> CaptionBlock {
        CaptionBlock {
            index: idx,
            start_us,
            end_us,
            lines: vec!["Hello world".to_string()],
            font_css: "Inter, sans-serif".into(),
            font_ass_name: "Inter".into(),
            font_size_px: 32,
            text_color: Rgba { r: 255, g: 255, b: 255, a: 255 },
            background: Rgba { r: 0, g: 0, b: 0, a: 0xB3 },
            padding_x_px: 12,
            padding_y_px: 4,
            radius_px: 4,
            margin_v_px: 108,
            text_width_px: 200,
            line_height_px: 40,
            frame_width: 1280,
            frame_height: 720,
        }
    }

    #[test]
    fn ass_time_formats_like_libass_expects() {
        assert_eq!(format_ass_time(0), "0:00:00.00");
        assert_eq!(format_ass_time(1_500_000), "0:00:01.50");
        assert_eq!(format_ass_time(3_661_234_000), "1:01:01.23");
    }

    #[test]
    fn rgba_to_abgr_encodes_alpha_and_bgr() {
        // Fully opaque white → alpha 00, BGR FFFFFF.
        let c = Rgba { r: 0xFF, g: 0xFF, b: 0xFF, a: 0xFF };
        assert_eq!(ass_color_abgr(c), "&H00FFFFFF");
        // 0xB3 CSS alpha → 0x4C ASS alpha.
        let d = Rgba { r: 0x00, g: 0x00, b: 0x00, a: 0xB3 };
        assert_eq!(ass_color_abgr(d), "&H4C000000");
        // Distinct channels confirm BGR byte order (not RGB).
        let e = Rgba { r: 0xAA, g: 0xBB, b: 0xCC, a: 0xFF };
        assert_eq!(ass_color_abgr(e), "&H00CCBBAA");
    }

    #[test]
    fn text_escapes_braces_and_backslashes() {
        assert_eq!(escape_ass_text(r"a{b}c"), r"a\{b\}c");
        assert_eq!(escape_ass_text(r"\N"), r"\\N");
    }

    #[test]
    fn box_padding_takes_the_max_of_xy() {
        let b = mk_block(1, 0, 1);
        // padding_x=12, padding_y=4 → max = 12.
        assert_eq!(box_padding_px(&b), 12);
    }

    #[test]
    fn document_uses_border_style_3_opaque_box() {
        let blocks = vec![mk_block(1, 0, 2_000_000), mk_block(2, 2_000_000, 4_000_000)];
        let doc = blocks_to_ass(&blocks);
        assert!(doc.contains("[Script Info]"));
        assert!(doc.contains("PlayResX: 1280"));
        assert!(doc.contains("PlayResY: 720"));
        assert!(doc.contains("[V4+ Styles]"));
        // BorderStyle=3 (opaque box) + Outline=12 (max padding) + Shadow=0 +
        // Alignment=2 (bottom-center). The exact substring pins the whole
        // style row so regressions are caught.
        assert!(
            doc.contains("Style: Default,Inter,32,&H00FFFFFF,&H000000FF,&H4C000000,&H00000000,0,0,0,0,100,100,0,0,3,12,0,2,0,0,0,1"),
            "style row mismatch: {doc}"
        );
        assert!(doc.contains("[Events]"));
        // One dialogue per block — libass sizes the box itself, so we no
        // longer emit a separate `\p1` background event per block.
        assert_eq!(doc.matches("Dialogue: ").count(), 2);
        assert!(!doc.contains("\\p1"), "must not emit \\p1 drawings");
        assert!(!doc.contains("\\p0"), "must not emit \\p0 drawings");
        assert!(doc.contains("Hello world"));
        // MarginV column carries the per-block bottom margin.
        assert!(doc.contains(",108,,Hello world"));
    }

    #[test]
    fn multi_line_block_joins_with_ass_newline() {
        let mut block = mk_block(1, 0, 2_000_000);
        block.lines = vec!["line one".into(), "line two".into()];
        let doc = blocks_to_ass(&[block]);
        assert!(doc.contains("line one\\Nline two"));
    }

    #[test]
    fn empty_blocks_still_produce_valid_header() {
        let doc = blocks_to_ass(&[]);
        assert!(doc.contains("[Script Info]"));
        assert!(doc.contains("[V4+ Styles]"));
        assert!(doc.contains("[Events]"));
        assert_eq!(doc.matches("Dialogue: ").count(), 0);
    }

    #[test]
    fn style_colors_come_from_first_block_not_defaults() {
        let mut block = mk_block(1, 0, 1_000_000);
        block.text_color = Rgba { r: 0xFF, g: 0x88, b: 0x00, a: 0xFF };
        block.background = Rgba { r: 0x10, g: 0x20, b: 0x30, a: 0x80 };
        let doc = blocks_to_ass(&[block]);
        // PrimaryColour = orange text, fully opaque.
        assert!(doc.contains("&H000088FF"));
        // OutlineColour = bg, alpha 0x80 CSS → 0x7F ASS.
        assert!(doc.contains("&H7F302010"));
    }
}
