//! Download / verify / delete / cancel machinery for LLM GGUF assets.
//!
//! Mirrors the shape of `managers::model::download` but is decoupled:
//! LLM assets live under `<app-data>/llm/<id>.gguf`, a separate directory
//! from Whisper `<app-data>/models/`. That separation is an explicit PRD
//! (R-003) choice so a user can "nuke one without touching the other".
//!
//! Disk-space preflight runs before any network activity: we require
//! `free_bytes >= 2 * entry.size_bytes`. The 2x factor provides headroom
//! for the `.partial` file plus the final `.gguf` during the atomic rename.

use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use log::{debug, info, warn};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use super::catalog::LlmCatalogEntry;

/// Snapshot of a drive's free bytes. Kept behind a trait-object so tests
/// can inject a "tight" value without racing against real disk state.
pub trait FreeSpaceProbe: Send + Sync {
    fn free_bytes(&self, path: &Path) -> Result<u64>;
}

pub struct RealFreeSpaceProbe;

impl FreeSpaceProbe for RealFreeSpaceProbe {
    fn free_bytes(&self, path: &Path) -> Result<u64> {
        // Windows: use GetDiskFreeSpaceExW. Other platforms fall back to
        // sysinfo::Disks.
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            // Use the first ancestor that exists so we can probe even when
            // the target dir hasn't been created yet.
            let probe_path = ancestor_that_exists(path);
            let wide: Vec<u16> = probe_path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let mut free_bytes_available: u64 = 0;
            let mut total_bytes: u64 = 0;
            let mut total_free_bytes: u64 = 0;
            unsafe {
                // SAFETY: wide is null-terminated; output pointers are valid
                // stack locals.
                let res = GetDiskFreeSpaceExW(
                    wide.as_ptr(),
                    &mut free_bytes_available,
                    &mut total_bytes,
                    &mut total_free_bytes,
                );
                if res == 0 {
                    return Err(anyhow!(
                        "GetDiskFreeSpaceExW failed for {}",
                        probe_path.display()
                    ));
                }
            }
            Ok(free_bytes_available)
        }
        #[cfg(not(windows))]
        {
            use sysinfo::Disks;
            let disks = Disks::new_with_refreshed_list();
            let probe_path = ancestor_that_exists(path);
            let mut best: Option<u64> = None;
            let mut best_mp_len: usize = 0;
            for disk in disks.list() {
                let mp = disk.mount_point();
                if probe_path.starts_with(mp) {
                    let mp_len = mp.as_os_str().len();
                    if mp_len >= best_mp_len {
                        best = Some(disk.available_space());
                        best_mp_len = mp_len;
                    }
                }
            }
            best.ok_or_else(|| {
                anyhow!("No mounted disk contains {}", probe_path.display())
            })
        }
    }
}

fn ancestor_that_exists(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    loop {
        if current.exists() {
            return current;
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent.to_path_buf(),
            _ => return path.to_path_buf(),
        }
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetDiskFreeSpaceExW(
        lpDirectoryName: *const u16,
        lpFreeBytesAvailable: *mut u64,
        lpTotalNumberOfBytes: *mut u64,
        lpTotalNumberOfFreeBytes: *mut u64,
    ) -> i32;
}

/// Result of the disk-space preflight check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskSpaceCheck {
    Ok,
    Tight { free: u64, required: u64 },
}

/// Check whether the drive hosting `dir` has at least `2 * required_bytes`
/// free. The 2x factor is the PRD R-004 rule — see BLUEPRINT "Disk-space
/// preflight".
pub fn check_disk_space<P: FreeSpaceProbe + ?Sized>(
    probe: &P,
    dir: &Path,
    required_bytes: u64,
) -> Result<DiskSpaceCheck> {
    let free = probe.free_bytes(dir)?;
    let required = required_bytes.saturating_mul(2);
    if free < required {
        Ok(DiskSpaceCheck::Tight { free, required })
    } else {
        Ok(DiskSpaceCheck::Ok)
    }
}

/// Build the final on-disk path for a catalog entry inside `llm_dir`.
/// Explicitly tested by AC-003-a.
pub fn gguf_path(llm_dir: &Path, entry: &LlmCatalogEntry) -> PathBuf {
    llm_dir.join(entry.filename())
}

pub fn partial_path(llm_dir: &Path, entry: &LlmCatalogEntry) -> PathBuf {
    llm_dir.join(format!("{}.partial", entry.filename()))
}

/// Remove the partial file for a download if it exists. Used by the
/// cancel and failure paths to clean up disk state.
pub fn remove_partial(llm_dir: &Path, entry: &LlmCatalogEntry) {
    let p = partial_path(llm_dir, entry);
    if p.exists() {
        if let Err(e) = fs::remove_file(&p) {
            warn!("Failed to remove partial file {}: {}", p.display(), e);
        }
    }
}

/// Delete the fully-downloaded GGUF file for a catalog entry, if present.
/// Also sweeps any leftover `.partial`.
pub fn delete_asset(llm_dir: &Path, entry: &LlmCatalogEntry) -> Result<()> {
    let final_path = gguf_path(llm_dir, entry);
    let mut deleted = false;
    if final_path.exists() {
        fs::remove_file(&final_path)?;
        deleted = true;
    }
    let p = partial_path(llm_dir, entry);
    if p.exists() {
        fs::remove_file(&p)?;
        deleted = true;
    }
    if !deleted {
        return Err(anyhow!(
            "No LLM asset to delete for id {}",
            entry.id
        ));
    }
    Ok(())
}

/// Download progress snapshot emitted to the frontend.
#[derive(Debug, Clone)]
pub struct LlmDownloadProgress {
    pub model_id: String,
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f64,
    pub speed_bps: u64,
}

/// Download a catalog entry to `<llm_dir>/<id>.gguf`. Performs the disk-space
/// preflight first, then streams the bytes into a `.partial` file, verifies
/// sha256 on completion, and renames to the final path.
///
/// `on_progress` is called at most every ~100ms; callers can hook Tauri
/// events there. `cancel` polls between chunks — when the flag flips, the
/// partial file is removed and `Ok(())` is returned (cancellation is not an
/// error for the caller; UI treats it as a clean state reset).
pub async fn download_entry<P: FreeSpaceProbe + ?Sized, F: FnMut(LlmDownloadProgress)>(
    probe: &P,
    llm_dir: &Path,
    entry: &LlmCatalogEntry,
    cancel: Arc<AtomicBool>,
    mut on_progress: F,
) -> Result<()> {
    // Preflight: ensure we have 2x the declared size free.
    if !llm_dir.exists() {
        fs::create_dir_all(llm_dir)?;
    }
    match check_disk_space(probe, llm_dir, entry.size_bytes)? {
        DiskSpaceCheck::Ok => {}
        DiskSpaceCheck::Tight { free, required } => {
            return Err(anyhow!(
                "Insufficient disk space: {} bytes free, need {} (2x model size)",
                free,
                required
            ));
        }
    }

    let final_path = gguf_path(llm_dir, entry);
    if final_path.exists() {
        info!(
            "LLM model {} already downloaded at {}",
            entry.id,
            final_path.display()
        );
        return Ok(());
    }

    let part_path = partial_path(llm_dir, entry);
    let resume_from = if part_path.exists() {
        part_path.metadata()?.len()
    } else {
        0
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    let mut request = client.get(&entry.download_url);
    if resume_from > 0 {
        request = request.header("Range", format!("bytes={}-", resume_from));
    }
    let response = request.send().await?;
    if !response.status().is_success() && !response.status().is_redirection() {
        return Err(anyhow!("Download failed with status {}", response.status()));
    }

    let total = response
        .content_length()
        .map(|c| c + resume_from)
        .unwrap_or(entry.size_bytes);

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(resume_from > 0)
        .write(true)
        .open(&part_path)?;

    let mut downloaded = resume_from;
    let mut stream = response.bytes_stream();
    let start = Instant::now();
    let mut last_emit = Instant::now();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            info!("LLM download cancelled for {}", entry.id);
            drop(file);
            let _ = fs::remove_file(&part_path);
            return Ok(());
        }
        let bytes = chunk?;
        file.write_all(&bytes)?;
        downloaded += bytes.len() as u64;

        if last_emit.elapsed().as_millis() >= 100 {
            let elapsed_secs = start.elapsed().as_secs_f64().max(0.001);
            let speed_bps =
                ((downloaded - resume_from) as f64 / elapsed_secs) as u64;
            let percentage = if total > 0 {
                (downloaded as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            on_progress(LlmDownloadProgress {
                model_id: entry.id.clone(),
                downloaded,
                total,
                percentage,
                speed_bps,
            });
            last_emit = Instant::now();
        }
    }
    file.flush()?;
    drop(file);

    // Verify sha256 on completion.
    let actual_hash = sha256_file(&part_path)?;
    if actual_hash.to_lowercase() != entry.sha256.to_lowercase() {
        let _ = fs::remove_file(&part_path);
        return Err(anyhow!(
            "sha256 mismatch for {}: expected {}, got {}",
            entry.id,
            entry.sha256,
            actual_hash
        ));
    }

    fs::rename(&part_path, &final_path)?;
    debug!(
        "LLM model {} downloaded and verified: {}",
        entry.id,
        final_path.display()
    );
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
pub(crate) mod download_tests {
    use super::*;
    use tempfile::TempDir;

    /// Test-only probe that returns a fixed free-bytes value.
    pub struct FixedFreeSpace(pub u64);
    impl FreeSpaceProbe for FixedFreeSpace {
        fn free_bytes(&self, _path: &Path) -> Result<u64> {
            Ok(self.0)
        }
    }

    fn sample_entry() -> LlmCatalogEntry {
        super::super::catalog::catalog().into_iter().next().unwrap()
    }

    #[test]
    fn llm_download_writes_under_llm_subdir() {
        let tmp = TempDir::new().unwrap();
        let llm_dir = tmp.path().join("llm");
        let entry = sample_entry();
        let final_p = gguf_path(&llm_dir, &entry);
        assert!(
            final_p.starts_with(&llm_dir),
            "final path must be inside llm/ dir: {}",
            final_p.display()
        );
        assert!(
            final_p.to_string_lossy().ends_with(".gguf"),
            "final path must end with .gguf: {}",
            final_p.display()
        );
        let filename = final_p.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(filename, format!("{}.gguf", entry.id));
    }

    #[tokio::test]
    async fn llm_download_preflight_errors_when_disk_tight() {
        let tmp = TempDir::new().unwrap();
        let llm_dir = tmp.path().join("llm");
        let entry = sample_entry();
        // Provide only half the required space (need 2x; we supply 1x).
        let probe = FixedFreeSpace(entry.size_bytes);
        let cancel = Arc::new(AtomicBool::new(false));
        let result = download_entry(&probe, &llm_dir, &entry, cancel, |_| {}).await;
        let err = result.expect_err("download must fail when disk is tight");
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("insufficient disk space"),
            "error should mention disk space; got: {}",
            msg
        );
        // No partial file should be created when the preflight rejects.
        let partial = partial_path(&llm_dir, &entry);
        assert!(
            !partial.exists(),
            "partial file must not be created on preflight failure"
        );
    }

    #[test]
    fn disk_space_check_ok_when_plenty_free() {
        let probe = FixedFreeSpace(10_000_000_000);
        let tmp = TempDir::new().unwrap();
        let result = check_disk_space(&probe, tmp.path(), 1_000_000).unwrap();
        assert_eq!(result, DiskSpaceCheck::Ok);
    }

    #[test]
    fn disk_space_check_tight_when_under_2x() {
        let probe = FixedFreeSpace(1_500_000);
        let tmp = TempDir::new().unwrap();
        let result = check_disk_space(&probe, tmp.path(), 1_000_000).unwrap();
        matches!(result, DiskSpaceCheck::Tight { .. });
    }
}
