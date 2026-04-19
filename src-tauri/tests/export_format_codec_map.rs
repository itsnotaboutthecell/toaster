//! AC-002-a: codec/muxer mapping for audio-only export formats.
//!
//! Lives in `src-tauri/tests/` (not as `#[cfg(test)] mod tests` inside
//! the lib) because an unrelated pre-existing breakage in
//! `commands/transcribe_file/precision_benchmarks.rs` prevents
//! `cargo test -p toaster --lib` from compiling — see the matching
//! comment in `loudness_preflight.rs`. Integration tests build against
//! the lib without `cfg(test)` and bypass that breakage.

use toaster_app_lib::commands::waveform::{
    export_format_codec_map as codec_map, AudioExportFormat,
};

#[test]
fn export_format_codec_map() {
    // Spec from features/export-audio-only/PRD.md R-002.
    assert!(codec_map(AudioExportFormat::Mp4).is_none(), "mp4 has no audio-only spec");

    let mp3 = codec_map(AudioExportFormat::Mp3).expect("mp3 spec must exist");
    assert_eq!(mp3.extension, ".mp3");
    assert_eq!(mp3.codec, "libmp3lame");
    assert_eq!(mp3.bitrate_kbps, Some(192));
    assert_eq!(mp3.bitrate_flag().as_deref(), Some("192k"));

    let wav = codec_map(AudioExportFormat::Wav).expect("wav spec must exist");
    assert_eq!(wav.extension, ".wav");
    assert_eq!(wav.codec, "pcm_s16le");
    assert_eq!(wav.bitrate_kbps, None);
    assert_eq!(wav.bitrate_flag(), None);

    let m4a = codec_map(AudioExportFormat::M4a).expect("m4a spec must exist");
    assert_eq!(m4a.extension, ".m4a");
    assert_eq!(m4a.codec, "aac");
    assert_eq!(m4a.bitrate_kbps, Some(192));

    let opus = codec_map(AudioExportFormat::Opus).expect("opus spec must exist");
    assert_eq!(opus.extension, ".opus");
    assert_eq!(opus.codec, "libopus");
    assert_eq!(opus.bitrate_kbps, Some(128));
}

#[test]
fn export_format_extension_matches_codec_map() {
    for fmt in [
        AudioExportFormat::Mp3,
        AudioExportFormat::Wav,
        AudioExportFormat::M4a,
        AudioExportFormat::Opus,
    ] {
        assert_eq!(
            Some(fmt.extension()),
            codec_map(fmt).map(|s| s.extension),
            "extension mismatch for {fmt:?}",
        );
    }
    assert_eq!(AudioExportFormat::Mp4.extension(), ".mp4");
}

#[test]
fn export_format_is_audio_only_classification() {
    assert!(!AudioExportFormat::Mp4.is_audio_only());
    assert!(AudioExportFormat::Mp3.is_audio_only());
    assert!(AudioExportFormat::Wav.is_audio_only());
    assert!(AudioExportFormat::M4a.is_audio_only());
    assert!(AudioExportFormat::Opus.is_audio_only());
}
