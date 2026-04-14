/// Caption and script export for Toaster.
///
/// Generates SRT, VTT, and plain-text script exports from
/// the transcript word list, respecting deletions and silenced words.

use crate::managers::editor::Word;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum ExportFormat {
    Srt,
    Vtt,
    Script,
}

/// Configuration for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Maximum characters per caption line.
    pub max_chars_per_line: usize,
    /// Maximum duration per caption segment in microseconds.
    pub max_segment_duration_us: i64,
    /// Whether to include silenced words in export.
    pub include_silenced: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            max_chars_per_line: 42,
            max_segment_duration_us: 5_000_000, // 5 seconds
            include_silenced: false,
        }
    }
}

/// A caption segment for SRT/VTT output.
#[derive(Debug, Clone)]
struct CaptionSegment {
    index: usize,
    start_us: i64,
    end_us: i64,
    text: String,
}

/// Build caption segments from word list, grouping words into segments
/// that respect max line length and duration constraints.
fn build_segments(words: &[Word], config: &ExportConfig) -> Vec<CaptionSegment> {
    let active_words: Vec<&Word> = words
        .iter()
        .filter(|w| !w.deleted && (config.include_silenced || !w.silenced))
        .collect();

    if active_words.is_empty() {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let mut seg_start = active_words[0].start_us;
    let mut seg_text = String::new();
    let mut seg_end = active_words[0].end_us;

    for word in &active_words {
        let would_be = if seg_text.is_empty() {
            word.text.len()
        } else {
            seg_text.len() + 1 + word.text.len()
        };

        let duration = word.end_us - seg_start;

        // Start a new segment if adding this word exceeds limits
        if !seg_text.is_empty()
            && (would_be > config.max_chars_per_line || duration > config.max_segment_duration_us)
        {
            segments.push(CaptionSegment {
                index: segments.len() + 1,
                start_us: seg_start,
                end_us: seg_end,
                text: seg_text.clone(),
            });
            seg_start = word.start_us;
            seg_text.clear();
        }

        if !seg_text.is_empty() {
            seg_text.push(' ');
        }
        seg_text.push_str(&word.text);
        seg_end = word.end_us;
    }

    // Push final segment
    if !seg_text.is_empty() {
        segments.push(CaptionSegment {
            index: segments.len() + 1,
            start_us: seg_start,
            end_us: seg_end,
            text: seg_text,
        });
    }

    segments
}

/// Format microseconds as SRT timestamp: HH:MM:SS,mmm
fn format_srt_time(us: i64) -> String {
    let total_ms = us / 1000;
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
}

/// Format microseconds as VTT timestamp: HH:MM:SS.mmm
fn format_vtt_time(us: i64) -> String {
    let total_ms = us / 1000;
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
}

/// Export transcript as SRT format.
pub fn export_srt(words: &[Word], config: &ExportConfig) -> String {
    let segments = build_segments(words, config);
    let mut output = String::new();

    for seg in &segments {
        output.push_str(&format!("{}\n", seg.index));
        output.push_str(&format!(
            "{} --> {}\n",
            format_srt_time(seg.start_us),
            format_srt_time(seg.end_us)
        ));
        output.push_str(&format!("{}\n\n", seg.text));
    }

    output
}

/// Export transcript as WebVTT format.
pub fn export_vtt(words: &[Word], config: &ExportConfig) -> String {
    let segments = build_segments(words, config);
    let mut output = String::from("WEBVTT\n\n");

    for seg in &segments {
        output.push_str(&format!(
            "{} --> {}\n",
            format_vtt_time(seg.start_us),
            format_vtt_time(seg.end_us)
        ));
        output.push_str(&format!("{}\n\n", seg.text));
    }

    output
}

/// Export transcript as plain text script.
pub fn export_script(words: &[Word], config: &ExportConfig) -> String {
    words
        .iter()
        .filter(|w| !w.deleted && (config.include_silenced || !w.silenced))
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Export transcript in the specified format.
pub fn export(words: &[Word], format: ExportFormat, config: &ExportConfig) -> String {
    match format {
        ExportFormat::Srt => export_srt(words, config),
        ExportFormat::Vtt => export_vtt(words, config),
        ExportFormat::Script => export_script(words, config),
    }
}

/// Save export to a file.
pub fn export_to_file(
    words: &[Word],
    format: ExportFormat,
    config: &ExportConfig,
    path: &std::path::Path,
) -> Result<(), String> {
    let content = export(words, format, config);
    std::fs::write(path, &content).map_err(|e| format!("Failed to write export: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_word(text: &str, start_us: i64, end_us: i64) -> Word {
        Word {
            text: text.to_string(),
            start_us,
            end_us,
            deleted: false,
            silenced: false,
            confidence: 1.0,
            speaker_id: -1,
        }
    }

    fn sample_words() -> Vec<Word> {
        vec![
            make_word("Hello", 0, 500_000),
            make_word("world", 500_000, 1_000_000),
            make_word("this", 1_500_000, 2_000_000),
            make_word("is", 2_000_000, 2_300_000),
            make_word("a", 2_300_000, 2_500_000),
            make_word("test", 2_500_000, 3_000_000),
        ]
    }

    #[test]
    fn srt_format_basic() {
        let words = sample_words();
        let config = ExportConfig {
            max_chars_per_line: 100,
            ..Default::default()
        };
        let srt = export_srt(&words, &config);
        assert!(srt.contains("1\n"));
        assert!(srt.contains("00:00:00,000 --> "));
        assert!(srt.contains("Hello world this is a test"));
    }

    #[test]
    fn srt_splits_on_max_chars() {
        let words = sample_words();
        let config = ExportConfig {
            max_chars_per_line: 12,
            ..Default::default()
        };
        let srt = export_srt(&words, &config);
        // Should have multiple segments
        assert!(srt.contains("1\n"));
        assert!(srt.contains("2\n"));
    }

    #[test]
    fn vtt_has_header() {
        let words = sample_words();
        let config = ExportConfig::default();
        let vtt = export_vtt(&words, &config);
        assert!(vtt.starts_with("WEBVTT\n\n"));
    }

    #[test]
    fn vtt_uses_dot_separator() {
        let words = sample_words();
        let config = ExportConfig::default();
        let vtt = export_vtt(&words, &config);
        assert!(vtt.contains("00:00:00.000"));
    }

    #[test]
    fn script_plain_text() {
        let words = sample_words();
        let config = ExportConfig::default();
        let script = export_script(&words, &config);
        assert_eq!(script, "Hello world this is a test");
    }

    #[test]
    fn deleted_words_excluded() {
        let mut words = sample_words();
        words[1].deleted = true; // "world"
        let config = ExportConfig::default();
        let script = export_script(&words, &config);
        assert!(!script.contains("world"));
        assert!(script.contains("Hello"));
    }

    #[test]
    fn silenced_words_excluded_by_default() {
        let mut words = sample_words();
        words[2].silenced = true; // "this"
        let config = ExportConfig::default();
        let script = export_script(&words, &config);
        assert!(!script.contains("this"));
    }

    #[test]
    fn silenced_words_included_when_configured() {
        let mut words = sample_words();
        words[2].silenced = true;
        let config = ExportConfig {
            include_silenced: true,
            ..Default::default()
        };
        let script = export_script(&words, &config);
        assert!(script.contains("this"));
    }

    #[test]
    fn empty_words_produces_empty_output() {
        let config = ExportConfig::default();
        assert!(export_srt(&[], &config).is_empty());
        assert_eq!(export_vtt(&[], &config), "WEBVTT\n\n");
        assert!(export_script(&[], &config).is_empty());
    }

    #[test]
    fn format_srt_time_correct() {
        assert_eq!(format_srt_time(0), "00:00:00,000");
        assert_eq!(format_srt_time(1_500_000), "00:00:01,500");
        assert_eq!(format_srt_time(3_661_234_000), "01:01:01,234");
    }

    #[test]
    fn format_vtt_time_correct() {
        assert_eq!(format_vtt_time(0), "00:00:00.000");
        assert_eq!(format_vtt_time(1_500_000), "00:00:01.500");
    }

    #[test]
    fn export_to_file_works() {
        let words = sample_words();
        let config = ExportConfig::default();
        let path = std::env::temp_dir().join("toaster_export_test.srt");
        export_to_file(&words, ExportFormat::Srt, &config, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("Hello"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn export_to_file_invalid_path() {
        let words = sample_words();
        let config = ExportConfig::default();
        let result = export_to_file(
            &words,
            ExportFormat::Srt,
            &config,
            std::path::Path::new("Z:\\nonexistent\\dir\\file.srt"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn export_dispatch_matches_format() {
        let words = sample_words();
        let config = ExportConfig::default();
        let srt = export(&words, ExportFormat::Srt, &config);
        let vtt = export(&words, ExportFormat::Vtt, &config);
        let script = export(&words, ExportFormat::Script, &config);
        assert!(srt.contains(","));      // SRT uses comma
        assert!(vtt.contains("WEBVTT")); // VTT has header
        assert!(!script.contains("-->")); // Script has no timestamps
    }
}
