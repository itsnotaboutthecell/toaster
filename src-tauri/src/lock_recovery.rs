//! Poison-recovery helpers for `std::sync` locks.
//!
//! Bare `lock().unwrap()` / `read().unwrap()` / `write().unwrap()` will panic
//! if any other thread previously panicked while holding the same lock. In a
//! Tauri command those panics tear down the command thread and, in practice,
//! kill the whole app. This module routes every poisoned-lock case through a
//! single decision point.
//!
//! ## Default policy
//!
//! - **`recover_*`** — log a warning at `warn!` level and recover the inner
//!   guard via `into_inner()`. The previous critical section panicked but the
//!   data is still readable; in many cases the right thing is to limp on
//!   rather than crash. Use for read paths, `&self` accessors that don't
//!   return `Result`, `Drop` impls, and background threads that have nowhere
//!   sensible to propagate an error.
//! - **`try_*`** — return `anyhow::Result<Guard>`. Use for write paths that
//!   already propagate errors (Tauri commands returning `Result<_, String>`,
//!   manager methods returning `anyhow::Result<_>`). The caller is then free
//!   to surface a domain error to the UI instead of corrupting more state.
//!
//! Tauri command call sites typically end with `.map_err(|e| e.to_string())?`.

use std::sync::{LockResult, MutexGuard, RwLockReadGuard, RwLockWriteGuard};

/// Recover a poisoned `Mutex` guard, logging a warning. Never panics.
pub fn recover_lock<T>(result: LockResult<MutexGuard<'_, T>>) -> MutexGuard<'_, T> {
    match result {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("mutex poisoned by previous panic; recovering inner guard");
            poisoned.into_inner()
        }
    }
}

/// Convert a poisoned `Mutex` `LockResult` into an `anyhow::Error` for
/// propagation. Use on write paths that already return `Result`.
pub fn try_lock<T>(result: LockResult<MutexGuard<'_, T>>) -> anyhow::Result<MutexGuard<'_, T>> {
    result.map_err(|e| anyhow::anyhow!("mutex poisoned: {}", e))
}

/// Recover a poisoned `RwLock` read guard, logging a warning. Never panics.
#[allow(dead_code)]
pub fn recover_read<T>(result: LockResult<RwLockReadGuard<'_, T>>) -> RwLockReadGuard<'_, T> {
    match result {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("rwlock read poisoned by previous panic; recovering inner guard");
            poisoned.into_inner()
        }
    }
}

/// Propagate a poisoned `RwLock` read as an `anyhow::Error`.
#[allow(dead_code)]
pub fn try_read<T>(
    result: LockResult<RwLockReadGuard<'_, T>>,
) -> anyhow::Result<RwLockReadGuard<'_, T>> {
    result.map_err(|e| anyhow::anyhow!("rwlock poisoned: {}", e))
}

/// Recover a poisoned `RwLock` write guard, logging a warning. Never panics.
#[allow(dead_code)]
pub fn recover_write<T>(result: LockResult<RwLockWriteGuard<'_, T>>) -> RwLockWriteGuard<'_, T> {
    match result {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("rwlock write poisoned by previous panic; recovering inner guard");
            poisoned.into_inner()
        }
    }
}

/// Propagate a poisoned `RwLock` write as an `anyhow::Error`.
#[allow(dead_code)]
pub fn try_write<T>(
    result: LockResult<RwLockWriteGuard<'_, T>>,
) -> anyhow::Result<RwLockWriteGuard<'_, T>> {
    result.map_err(|e| anyhow::anyhow!("rwlock poisoned: {}", e))
}
