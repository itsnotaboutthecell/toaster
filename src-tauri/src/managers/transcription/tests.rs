//! Extracted from the inline `mod tests` block (monolith-split).

use super::*;

// ── LoadingGuard ────────────────────────────────────────────────

#[test]
fn loading_guard_clears_flag_on_drop() {
    let is_loading = Arc::new(Mutex::new(true));
    let condvar = Arc::new(Condvar::new());

    let guard = LoadingGuard {
        is_loading: is_loading.clone(),
        loading_condvar: condvar.clone(),
    };

    assert!(*is_loading.lock().unwrap());
    drop(guard);
    assert!(!*is_loading.lock().unwrap());
}

#[test]
fn loading_guard_notifies_waiters_on_drop() {
    let is_loading = Arc::new(Mutex::new(true));
    let condvar = Arc::new(Condvar::new());

    let guard = LoadingGuard {
        is_loading: is_loading.clone(),
        loading_condvar: condvar.clone(),
    };

    let is_loading_clone = is_loading.clone();
    let condvar_clone = condvar.clone();
    let waiter = thread::spawn(move || {
        let mut lock = is_loading_clone.lock().unwrap();
        while *lock {
            let (guard, timeout) = condvar_clone
                .wait_timeout(lock, Duration::from_secs(5))
                .unwrap();
            lock = guard;
            if timeout.timed_out() {
                panic!("Timed out waiting for loading guard drop");
            }
        }
    });

    // Small delay so the waiter thread parks on the condvar
    thread::sleep(Duration::from_millis(50));
    drop(guard);

    waiter.join().expect("Waiter thread panicked");
    assert!(!*is_loading.lock().unwrap());
}

#[test]
fn loading_guard_clears_flag_even_when_already_false() {
    let is_loading = Arc::new(Mutex::new(false));
    let condvar = Arc::new(Condvar::new());

    let guard = LoadingGuard {
        is_loading: is_loading.clone(),
        loading_condvar: condvar.clone(),
    };
    drop(guard);
    assert!(!*is_loading.lock().unwrap());
}

// ── now_ms ──────────────────────────────────────────────────────

#[test]
fn now_ms_returns_plausible_epoch_millis() {
    let ts = TranscriptionManager::now_ms();
    // Should be well past year-2020 in millis (1_577_836_800_000)
    assert!(ts > 1_577_836_800_000, "timestamp too small: {}", ts);
}

#[test]
fn now_ms_is_monotonic_over_short_interval() {
    let t1 = TranscriptionManager::now_ms();
    thread::sleep(Duration::from_millis(10));
    let t2 = TranscriptionManager::now_ms();
    assert!(t2 >= t1, "expected t2 ({}) >= t1 ({})", t2, t1);
}

// ── get_available_accelerators ──────────────────────────────────

#[test]
fn available_accelerators_whisper_has_all_options() {
    let accel = get_available_accelerators();
    assert!(accel.whisper.contains(&"auto".to_string()));
    assert!(accel.whisper.contains(&"cpu".to_string()));
    assert!(accel.whisper.contains(&"gpu".to_string()));
    assert_eq!(accel.whisper.len(), 3);
}

#[test]
fn available_accelerators_ort_is_non_empty() {
    let accel = get_available_accelerators();
    assert!(!accel.ort.is_empty(), "ORT should have at least one option");
}

// ── ModelStateEvent serialization ───────────────────────────────

#[test]
fn model_state_event_serializes_all_fields() {
    let event = ModelStateEvent {
        event_type: "loading_started".to_string(),
        model_id: Some("whisper-base".to_string()),
        model_name: Some("Whisper Base".to_string()),
        error: None,
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["event_type"], "loading_started");
    assert_eq!(json["model_id"], "whisper-base");
    assert_eq!(json["model_name"], "Whisper Base");
    assert!(json["error"].is_null());
}

#[test]
fn model_state_event_serializes_error() {
    let event = ModelStateEvent {
        event_type: "loading_failed".to_string(),
        model_id: Some("bad-model".to_string()),
        model_name: None,
        error: Some("File not found".to_string()),
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["event_type"], "loading_failed");
    assert_eq!(json["error"], "File not found");
    assert!(json["model_name"].is_null());
}

#[test]
fn model_state_event_unloaded_has_no_ids() {
    let event = ModelStateEvent {
        event_type: "unloaded".to_string(),
        model_id: None,
        model_name: None,
        error: None,
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["event_type"], "unloaded");
    assert!(json["model_id"].is_null());
}

// ── GpuDeviceOption serialization ───────────────────────────────

#[test]
fn gpu_device_option_serializes() {
    let opt = GpuDeviceOption {
        id: 0,
        name: "Test GPU".to_string(),
        total_vram_mb: 8192,
    };
    let json = serde_json::to_value(&opt).unwrap();
    assert_eq!(json["id"], 0);
    assert_eq!(json["name"], "Test GPU");
    assert_eq!(json["total_vram_mb"], 8192);
}

// ── AvailableAccelerators serialization ─────────────────────────

#[test]
fn available_accelerators_serializes() {
    let accel = AvailableAccelerators {
        whisper: vec!["auto".to_string(), "cpu".to_string()],
        ort: vec!["cpu".to_string()],
        gpu_devices: vec![],
    };
    let json = serde_json::to_value(&accel).unwrap();
    assert_eq!(json["whisper"].as_array().unwrap().len(), 2);
    assert_eq!(json["ort"].as_array().unwrap().len(), 1);
    assert!(json["gpu_devices"].as_array().unwrap().is_empty());
}

// ── ModelUnloadTimeout (tested here for proximity to idle logic) ─

#[test]
fn timeout_never_returns_none() {
    assert_eq!(ModelUnloadTimeout::Never.to_seconds(), None);
}

#[test]
fn timeout_immediately_returns_zero() {
    assert_eq!(ModelUnloadTimeout::Immediately.to_seconds(), Some(0));
}

#[test]
fn timeout_sec15_returns_15() {
    assert_eq!(ModelUnloadTimeout::Sec15.to_seconds(), Some(15));
}

#[test]
fn timeout_min5_returns_300() {
    assert_eq!(ModelUnloadTimeout::Min5.to_seconds(), Some(300));
}

#[test]
fn timeout_hour1_returns_3600() {
    assert_eq!(ModelUnloadTimeout::Hour1.to_seconds(), Some(3600));
}

#[test]
fn timeout_default_is_min5() {
    assert_eq!(ModelUnloadTimeout::default(), ModelUnloadTimeout::Min5);
}

// ── Idle-timeout arithmetic (mirrors watcher logic) ─────────────

#[test]
fn idle_detection_triggers_when_elapsed_exceeds_limit() {
    // Simulate the idle watcher's comparison logic
    let last_activity_ms = 1_000_000u64;
    let now_ms = 1_400_000u64; // 400 seconds later
    let limit_seconds = 300u64; // 5 minutes

    let idle_ms = now_ms.saturating_sub(last_activity_ms);
    let limit_ms = limit_seconds * 1000;

    assert!(
        idle_ms > limit_ms,
        "should detect idle after 400s > 300s limit"
    );
}

#[test]
fn idle_detection_does_not_trigger_when_within_limit() {
    let last_activity_ms = 1_000_000u64;
    let now_ms = 1_100_000u64; // 100 seconds later
    let limit_seconds = 300u64;

    let idle_ms = now_ms.saturating_sub(last_activity_ms);
    let limit_ms = limit_seconds * 1000;

    assert!(
        idle_ms <= limit_ms,
        "should not detect idle after 100s < 300s limit"
    );
}

#[test]
fn saturating_sub_handles_clock_wrap() {
    // If now < last (e.g. clock adjustment), saturating_sub returns 0
    let last_activity_ms = 2_000_000u64;
    let now_ms = 1_000_000u64;
    let idle_ms = now_ms.saturating_sub(last_activity_ms);
    assert_eq!(idle_ms, 0, "saturating_sub should prevent underflow");
}

// ── Loading-flag state machine (simulates try_start_loading) ────

#[test]
fn try_start_loading_pattern_grants_first_caller() {
    let is_loading = Arc::new(Mutex::new(false));
    let condvar = Arc::new(Condvar::new());

    // Simulate try_start_loading: CAS on the flag
    {
        let mut flag = is_loading.lock().unwrap();
        assert!(!*flag, "should start unlocked");
        *flag = true;
    }

    // Now construct guard (simulates successful try_start_loading)
    let guard = LoadingGuard {
        is_loading: is_loading.clone(),
        loading_condvar: condvar.clone(),
    };
    assert!(*is_loading.lock().unwrap());

    drop(guard);
    assert!(!*is_loading.lock().unwrap());
}

#[test]
fn try_start_loading_pattern_rejects_second_caller() {
    let is_loading = Arc::new(Mutex::new(false));
    let condvar = Arc::new(Condvar::new());

    // First caller succeeds
    {
        let mut flag = is_loading.lock().unwrap();
        *flag = true;
    }
    let _guard = LoadingGuard {
        is_loading: is_loading.clone(),
        loading_condvar: condvar.clone(),
    };

    // Second caller sees is_loading == true and would return None
    {
        let flag = is_loading.lock().unwrap();
        assert!(*flag, "second caller should see loading in progress");
    }
}

// ── Touch-activity pattern ──────────────────────────────────────

#[test]
fn atomic_activity_timestamp_updates() {
    let last_activity = AtomicU64::new(0);
    assert_eq!(last_activity.load(Ordering::Relaxed), 0);

    let now = TranscriptionManager::now_ms();
    last_activity.store(now, Ordering::Relaxed);
    assert_eq!(last_activity.load(Ordering::Relaxed), now);
}
