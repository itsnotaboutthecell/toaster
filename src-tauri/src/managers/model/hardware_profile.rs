//! Hardware profile probe for the model recommender.
//!
//! Computed once per `ModelManager` instance (see
//! `managers::model::mod::ModelManager::new`) and cached inside an
//! `Arc<HardwareProfile>`. The frontend never probes; it calls the
//! `get_hardware_profile` Tauri command which hands back the cached
//! value. Probe mechanics are intentionally:
//!
//!   - **In-process** — `std::thread::available_parallelism`,
//!     `sysinfo::System`, `sysinfo::Disks`. No shell-outs, no
//!     `nvidia-smi`, no CUDA/DirectML DLL touches at startup (the
//!     latter is a documented Windows DLL pitfall in
//!     `src-tauri/AGENTS.md`).
//!   - **Local-only** — no network syscalls. Toaster's non-negotiable
//!     local-only-inference rule (`AGENTS.md > Critical rules`)
//!     applies to every module on the ModelManager path.
//!   - **Non-fatal** — any probe error degrades to a conservative
//!     fallback (`FALLBACK_PROFILE`) so scoring still returns a safe
//!     `Fastest`-tier recommendation.
//!
//! Accelerator detection is compile-time via `cfg!(target_os = ...)`
//! because the whisper backend feature set is a function of the target
//! triple (see `src-tauri/Cargo.toml:83,93,96`). No runtime probe of
//! device presence — that would pull in DLLs we don't want to load
//! just to answer "which card do you have".

use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;

/// GPU / CPU accelerator class used for scoring. Detection is
/// compile-time only; the variants exist for forward compatibility
/// (`Cuda` / `DirectMl`) even though today's whisper feature set wires
/// Windows + Linux to Vulkan and macOS to Metal.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum Accelerator {
    Cpu,
    Cuda,
    Metal,
    Vulkan,
    DirectMl,
}

/// Cached view of the user's machine capability. Every field is
/// cheap to compute (< 50 ms total on a cold start) and read-only
/// after probe.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct HardwareProfile {
    pub cpu_cores: u32,
    pub total_ram_mb: u64,
    pub accelerator: Accelerator,
    pub models_dir_free_mb: u64,
}

/// Safe fallback when any probe step fails. Chosen to always land in
/// the `Fastest` tier so a user whose machine misreports itself still
/// gets a runnable model.
pub const FALLBACK_PROFILE: HardwareProfile = HardwareProfile {
    cpu_cores: 2,
    total_ram_mb: 4096,
    accelerator: Accelerator::Cpu,
    models_dir_free_mb: 10_240,
};

/// Whisper backend accelerator for the current target. The mapping
/// mirrors `src-tauri/Cargo.toml:83,93,96`:
///
/// - Windows / Linux: `whisper-vulkan` → `Vulkan`
/// - macOS:            `whisper-metal`  → `Metal`
/// - otherwise:        `Cpu`
pub(crate) fn detect_accelerator() -> Accelerator {
    #[cfg(target_os = "macos")]
    {
        Accelerator::Metal
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        Accelerator::Vulkan
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Accelerator::Cpu
    }
}

/// CPU core count via `std::thread::available_parallelism`. Returns
/// the fallback value on error. Never panics.
fn probe_cpu_cores() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(FALLBACK_PROFILE.cpu_cores)
}

/// Total system RAM in megabytes. `sysinfo::System::total_memory`
/// returns bytes in sysinfo ≥ 0.30 (kilobytes in older releases); we
/// normalize through a `/ (1024 * 1024)` divide. Returns the fallback
/// value on a zero/error read.
fn probe_total_ram_mb() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let bytes = sys.total_memory();
    if bytes == 0 {
        FALLBACK_PROFILE.total_ram_mb
    } else {
        bytes / (1024 * 1024)
    }
}

/// Free space in MB on the disk that contains `models_dir`. Walks the
/// mounted-disk list and picks the one whose `mount_point` is an
/// ancestor of `models_dir`, preferring the longest match so nested
/// mounts (e.g. a user-data volume mounted inside `/home`) win over
/// the root volume. Falls back to `FALLBACK_PROFILE.models_dir_free_mb`
/// if no match or on any error.
fn probe_free_disk_mb(models_dir: &Path) -> u64 {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let canon = models_dir
        .canonicalize()
        .unwrap_or_else(|_| models_dir.to_path_buf());

    let mut best: Option<(usize, u64)> = None;
    for disk in disks.iter() {
        let mount = disk.mount_point();
        if canon.starts_with(mount) {
            let mount_len = mount.as_os_str().len();
            let avail = disk.available_space() / (1024 * 1024);
            match best {
                Some((prev_len, _)) if prev_len >= mount_len => {}
                _ => best = Some((mount_len, avail)),
            }
        }
    }

    best.map(|(_, mb)| mb)
        .unwrap_or(FALLBACK_PROFILE.models_dir_free_mb)
}

/// Probe the machine once. Cheap (< 50 ms) and local-only. Any
/// error in a sub-probe degrades only that field — accelerator and
/// CPU count are independent of RAM and disk.
pub fn probe(models_dir: &Path) -> HardwareProfile {
    HardwareProfile {
        cpu_cores: probe_cpu_cores(),
        total_ram_mb: probe_total_ram_mb(),
        accelerator: detect_accelerator(),
        models_dir_free_mb: probe_free_disk_mb(models_dir),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_cpu_cores_at_least_one() {
        assert!(probe_cpu_cores() >= 1);
    }

    #[test]
    fn probe_total_ram_mb_at_least_512() {
        // AC-001-a: any real machine running Toaster has > 512 MB.
        // If the sysinfo probe zeroes out we fall back to 4096.
        assert!(probe_total_ram_mb() >= 512);
    }

    #[test]
    fn detect_accelerator_matches_target_os() {
        let acc = detect_accelerator();
        #[cfg(target_os = "macos")]
        assert_eq!(acc, Accelerator::Metal);
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        assert_eq!(acc, Accelerator::Vulkan);
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        assert_eq!(acc, Accelerator::Cpu);
    }

    #[test]
    fn probe_returns_all_fields_populated() {
        // AC-001-a: a freshly probed profile has sensible fields.
        let tmp = std::env::temp_dir();
        let p = probe(&tmp);
        assert!(p.cpu_cores >= 1);
        assert!(p.total_ram_mb >= 512);
        // models_dir_free_mb can legitimately be very small on a
        // container; we only assert it's not zero (fallback kicks in).
        assert!(p.models_dir_free_mb >= 1);
    }

    #[test]
    fn probe_is_stable_on_repeat_calls() {
        // Two probes of the same directory should agree on the
        // static fields (cores + accelerator). RAM + disk can drift
        // a tiny amount between calls on a busy system; we don't
        // assert equality there.
        let tmp = std::env::temp_dir();
        let a = probe(&tmp);
        let b = probe(&tmp);
        assert_eq!(a.cpu_cores, b.cpu_cores);
        assert_eq!(a.accelerator, b.accelerator);
    }

    #[test]
    fn fallback_profile_is_conservative_fastest_tier() {
        // AC-001-a / edge-case: the fallback must always produce a
        // runnable recommendation. 2 cores + 4 GB + CPU = Fastest
        // tier per the scoring function (see recommendation.rs).
        assert_eq!(FALLBACK_PROFILE.cpu_cores, 2);
        assert_eq!(FALLBACK_PROFILE.total_ram_mb, 4096);
        assert_eq!(FALLBACK_PROFILE.accelerator, Accelerator::Cpu);
        // Non-const comparison to dodge clippy's
        // `assertions_on_constants` — we want this asserted at test
        // runtime even though both sides are const today.
        let free_mb = FALLBACK_PROFILE.models_dir_free_mb;
        assert!(free_mb >= 1024);
    }
}
