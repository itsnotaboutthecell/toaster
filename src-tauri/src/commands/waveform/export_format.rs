//! Export format presets and codec/muxer mapping.
//!
//! Single source of truth for which FFmpeg codec, container extension,
//! and bitrate flag belong with each user-facing export format.
//! AGENTS.md "Single source of truth for dual-path logic" — frontend
//! sends the enum; backend is the only place that builds `-c:a` /
//! `-b:a` / `-vn` flags. See `build_export_args` in the parent module
//! for how these specs are composed into the final FFmpeg argv.
//!
//! The audio post-filter chain (`build_audio_post_filters`, including
//! the loudnorm stage from `splice::loudness`) is applied identically
//! to video and audio-only renders — see R-005 in
//! `features/export-audio-only/PRD.md`.

use serde::{Deserialize, Serialize};
use specta::Type;

/// User-facing export format. Default is `Mp4` (current behavior:
/// H.264 video + AAC audio in mp4). The four audio-only variants drop
/// the video stream (`-vn`) and re-mux the post-edit audio with the
/// codec / bitrate listed in `export_format_codec_map`.
///
/// Serialized lowercase per PRD R-001 / data model:
/// `"mp4" | "mp3" | "wav" | "m4a" | "opus"`.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum AudioExportFormat {
    #[default]
    Mp4,
    Mp3,
    Wav,
    M4a,
    Opus,
}

impl AudioExportFormat {
    /// True for the four audio-only formats. Audio-only renders force
    /// `-vn`, omit `-c:v`, and select extension/codec/bitrate from
    /// `export_format_codec_map`.
    pub fn is_audio_only(self) -> bool {
        !matches!(self, AudioExportFormat::Mp4)
    }

    /// User-facing default file extension for the format (with leading
    /// dot). For Mp4 this is `.mp4`; for the audio-only formats it is
    /// the value returned by `export_format_codec_map`.
    pub fn extension(self) -> &'static str {
        match self {
            AudioExportFormat::Mp4 => ".mp4",
            AudioExportFormat::Mp3 => ".mp3",
            AudioExportFormat::Wav => ".wav",
            AudioExportFormat::M4a => ".m4a",
            AudioExportFormat::Opus => ".opus",
        }
    }
}

/// Codec / muxer / bitrate spec for an audio-only export format.
///
/// `bitrate_kbps` is `None` for `pcm_s16le` (wav) where bitrate is
/// determined by sample rate + bit depth and `-b:a` would be ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodecSpec {
    pub extension: &'static str,
    pub codec: &'static str,
    pub bitrate_kbps: Option<u32>,
}

impl CodecSpec {
    /// `-b:a <bitrate>k` formatted string, or `None` when no bitrate
    /// flag is appropriate (e.g. PCM).
    pub fn bitrate_flag(&self) -> Option<String> {
        self.bitrate_kbps.map(|k| format!("{k}k"))
    }
}

/// Map an audio-only `AudioExportFormat` to its FFmpeg codec spec.
///
/// Returns `None` for `AudioExportFormat::Mp4` (the video pipeline owns
/// codec selection there — see Bundle 3 `export-hardware-encoder`).
///
/// Spec (PRD R-002 / AC-002-a):
/// - mp3  -> ".mp3",  "libmp3lame", -b:a 192k
/// - wav  -> ".wav",  "pcm_s16le",  no bitrate flag
/// - m4a  -> ".m4a",  "aac",        -b:a 192k
/// - opus -> ".opus", "libopus",    -b:a 128k
pub fn export_format_codec_map(format: AudioExportFormat) -> Option<CodecSpec> {
    match format {
        AudioExportFormat::Mp4 => None,
        AudioExportFormat::Mp3 => Some(CodecSpec {
            extension: ".mp3",
            codec: "libmp3lame",
            bitrate_kbps: Some(192),
        }),
        AudioExportFormat::Wav => Some(CodecSpec {
            extension: ".wav",
            codec: "pcm_s16le",
            bitrate_kbps: None,
        }),
        AudioExportFormat::M4a => Some(CodecSpec {
            extension: ".m4a",
            codec: "aac",
            bitrate_kbps: Some(192),
        }),
        AudioExportFormat::Opus => Some(CodecSpec {
            extension: ".opus",
            codec: "libopus",
            bitrate_kbps: Some(128),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_format_codec_map_matches_prd_spec() {
        // AC-002-a: backed-by-test mapping exactly as written in
        // features/export-audio-only/PRD.md R-002.
        assert_eq!(export_format_codec_map(AudioExportFormat::Mp4), None);
        assert_eq!(
            export_format_codec_map(AudioExportFormat::Mp3),
            Some(CodecSpec {
                extension: ".mp3",
                codec: "libmp3lame",
                bitrate_kbps: Some(192),
            })
        );
        assert_eq!(
            export_format_codec_map(AudioExportFormat::Wav),
            Some(CodecSpec {
                extension: ".wav",
                codec: "pcm_s16le",
                bitrate_kbps: None,
            })
        );
        assert_eq!(
            export_format_codec_map(AudioExportFormat::M4a),
            Some(CodecSpec {
                extension: ".m4a",
                codec: "aac",
                bitrate_kbps: Some(192),
            })
        );
        assert_eq!(
            export_format_codec_map(AudioExportFormat::Opus),
            Some(CodecSpec {
                extension: ".opus",
                codec: "libopus",
                bitrate_kbps: Some(128),
            })
        );
    }

    #[test]
    fn audio_only_formats_report_audio_only() {
        assert!(!AudioExportFormat::Mp4.is_audio_only());
        assert!(AudioExportFormat::Mp3.is_audio_only());
        assert!(AudioExportFormat::Wav.is_audio_only());
        assert!(AudioExportFormat::M4a.is_audio_only());
        assert!(AudioExportFormat::Opus.is_audio_only());
    }

    #[test]
    fn extensions_match_codec_map() {
        for fmt in [
            AudioExportFormat::Mp3,
            AudioExportFormat::Wav,
            AudioExportFormat::M4a,
            AudioExportFormat::Opus,
        ] {
            assert_eq!(
                Some(fmt.extension()),
                export_format_codec_map(fmt).map(|s| s.extension),
            );
        }
    }

    #[test]
    fn bitrate_flag_formats_kbps() {
        let mp3 = export_format_codec_map(AudioExportFormat::Mp3).unwrap();
        assert_eq!(mp3.bitrate_flag().as_deref(), Some("192k"));
        let wav = export_format_codec_map(AudioExportFormat::Wav).unwrap();
        assert_eq!(wav.bitrate_flag(), None);
    }
}
