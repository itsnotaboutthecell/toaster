//! Preview cache filesystem helpers.
//!
//! Extracted from `waveform/mod.rs`. Pure stateless helpers for naming,
//! writing, invalidating, and cleaning up transient preview audio files
//! in `$TEMP/toaster_preview_cache/`.

use log::warn;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const PREVIEW_CACHE_DIR: &str = "toaster_preview_cache";
const PREVIEW_CACHE_FILE_PREFIX: &str = "preview-";
const PREVIEW_CACHE_FILE_SUFFIX: &str = ".m4a";
const PREVIEW_TOKEN_SEPARATOR: &str = "--";
const PREVIEW_CACHE_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24);

#[derive(Debug, Default)]
pub(super) struct PreviewCacheCleanupSummary {
    pub scanned_files: usize,
    pub removed_files: usize,
    pub removed_stale_files: usize,
    pub removed_mismatched_files: usize,
    pub removed_empty_files: usize,
}

pub(super) fn fnv1a_64_hex(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in input.as_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub(super) fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        match ch {
            ' ' => out.push_str("%20"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            '/' | '-' | '_' | '.' | ':' | '~' => out.push(ch),
            c if c.is_ascii_alphanumeric() => out.push(c),
            c => {
                for byte in c.to_string().as_bytes() {
                    out.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    out
}

/// Returns the preview cache directory path.
pub(super) fn preview_cache_dir() -> PathBuf {
    std::env::temp_dir().join(PREVIEW_CACHE_DIR)
}

pub(super) fn preview_generation_token(source_fingerprint: &str, edit_version: &str) -> String {
    format!("{source_fingerprint}{PREVIEW_TOKEN_SEPARATOR}{edit_version}")
}

pub(super) fn preview_output_path(preview_dir: &Path, generation_token: &str) -> PathBuf {
    preview_dir.join(format!(
        "{PREVIEW_CACHE_FILE_PREFIX}{generation_token}{PREVIEW_CACHE_FILE_SUFFIX}"
    ))
}

fn parse_generation_token(generation_token: &str) -> Option<(&str, &str)> {
    generation_token
        .split_once(PREVIEW_TOKEN_SEPARATOR)
        .or_else(|| generation_token.split_once(':'))
}

fn parse_preview_cache_entry(path: &Path) -> Option<(String, String, String)> {
    let file_name = path.file_name()?.to_str()?;
    let generation_token = file_name
        .strip_prefix(PREVIEW_CACHE_FILE_PREFIX)?
        .strip_suffix(PREVIEW_CACHE_FILE_SUFFIX)?;
    let (source_fingerprint, edit_version) = parse_generation_token(generation_token)?;
    Some((
        generation_token.to_string(),
        source_fingerprint.to_string(),
        edit_version.to_string(),
    ))
}

pub(super) fn cleanup_preview_cache(
    preview_dir: &Path,
    active_source_fingerprint: Option<&str>,
    active_edit_version: Option<&str>,
) -> PreviewCacheCleanupSummary {
    let mut summary = PreviewCacheCleanupSummary::default();
    let entries = match std::fs::read_dir(preview_dir) {
        Ok(entries) => entries,
        Err(_) => return summary,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        summary.scanned_files += 1;

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                warn!(
                    "Failed to read preview cache metadata for {}: {}",
                    path.display(),
                    error
                );
                continue;
            }
        };

        let is_empty = metadata.len() == 0;
        let is_stale = metadata
            .modified()
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .map(|age| age > PREVIEW_CACHE_MAX_AGE)
            .unwrap_or(false);

        let parsed_entry = parse_preview_cache_entry(&path);
        let is_mismatched = match (
            active_source_fingerprint,
            active_edit_version,
            parsed_entry.as_ref(),
        ) {
            (Some(active_source), Some(active_edit), Some((_, source, edit))) => {
                source != active_source || edit != active_edit
            }
            (Some(active_source), None, Some((_, source, _))) => source != active_source,
            _ => false,
        };

        if !(is_empty || is_stale || is_mismatched) {
            continue;
        }

        match std::fs::remove_file(&path) {
            Ok(_) => {
                summary.removed_files += 1;
                if is_empty {
                    summary.removed_empty_files += 1;
                }
                if is_stale {
                    summary.removed_stale_files += 1;
                }
                if is_mismatched {
                    summary.removed_mismatched_files += 1;
                }
            }
            Err(error) => {
                warn!(
                    "Failed to remove preview cache file {}: {}",
                    path.display(),
                    error
                );
            }
        }
    }

    summary
}

pub(super) fn invalidate_preview_cache_entries(
    preview_dir: &Path,
    generation_token: Option<&str>,
    source_media_fingerprint: Option<&str>,
) -> usize {
    let entries = match std::fs::read_dir(preview_dir) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    let mut removed_files = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let should_remove = parse_preview_cache_entry(&path)
            .map(|(entry_generation_token, entry_source_fingerprint, _)| {
                if let Some(token) = generation_token {
                    entry_generation_token == token
                } else {
                    source_media_fingerprint
                        .map(|source| entry_source_fingerprint == source)
                        .unwrap_or(false)
                }
            })
            .unwrap_or(false);

        if !should_remove {
            continue;
        }

        match std::fs::remove_file(&path) {
            Ok(_) => removed_files += 1,
            Err(error) => warn!(
                "Failed to invalidate preview cache file {}: {}",
                path.display(),
                error
            ),
        }
    }

    removed_files
}

pub(super) fn source_media_fingerprint(path: &Path) -> Result<String, String> {
    let metadata =
        std::fs::metadata(path).map_err(|e| format!("Cannot read media metadata: {}", e))?;
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let modified_secs = metadata
        .modified()
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let key = format!(
        "{}|{}|{}",
        canonical.to_string_lossy(),
        metadata.len(),
        modified_secs
    );
    Ok(fnv1a_64_hex(&key))
}

pub(super) fn edit_version_token(segments: &[(i64, i64)]) -> String {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write_u64(segments.len() as u64);
    for (start_us, end_us) in segments {
        hasher.write_i64(*start_us);
        hasher.write_i64(*end_us);
    }
    format!("{:016x}", hasher.finish())
}
