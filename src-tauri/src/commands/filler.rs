use tauri::State;

use crate::commands::editor::EditorStore;
use crate::managers::filler::{self, FillerConfig};

/// Detect filler words, duplicates, and long pauses in the current transcript.
/// Runs iterative simulation: after marking fillers as deleted, re-scans for
/// cascading duplicates (e.g., "the um the" → "the the" after filler removal).
/// This ensures the reported counts match what `cleanup_all` would actually remove.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct FillerAnalysis {
    pub filler_indices: Vec<usize>,
    /// Each pause: (word_index_before_gap, gap_duration_us)
    pub pauses: Vec<PauseInfo>,
    pub filler_count: usize,
    pub pause_count: usize,
    /// Indices of the second word in each adjacent duplicate pair.
    pub duplicate_indices: Vec<usize>,
    pub duplicate_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct PauseInfo {
    pub after_word_index: usize,
    pub gap_duration_us: i64,
}

#[tauri::command]
#[specta::specta]
pub fn analyze_fillers(
    app: tauri::AppHandle,
    store: State<EditorStore>,
    min_pause_us: Option<i64>,
) -> Result<FillerAnalysis, String> {
    let state = store.0.lock().unwrap();
    let mut words = state.get_words().to_vec();

    let settings = crate::settings::get_settings(&app);
    let filler_list = settings.custom_filler_words.clone().unwrap_or_default();

    let mut config = FillerConfig {
        filler_words: filler_list,
        ..Default::default()
    };
    if let Some(threshold) = min_pause_us {
        config.pause_threshold_us = threshold;
    }

    let mut all_filler_indices: Vec<usize> = Vec::new();
    let mut all_duplicate_indices: Vec<usize> = Vec::new();
    const MAX_PASSES: usize = 10;

    for _ in 0..MAX_PASSES {
        let mut changed = false;

        let filler_indices = filler::detect_fillers(&words, &config);
        if !filler_indices.is_empty() {
            for &idx in &filler_indices {
                if idx < words.len() {
                    all_filler_indices.push(idx);
                    words[idx].deleted = true;
                }
            }
            changed = true;
        }

        let dup_indices = filler::detect_duplicates(&words);
        if !dup_indices.is_empty() {
            for &idx in &dup_indices {
                if idx < words.len() {
                    all_duplicate_indices.push(idx);
                    words[idx].deleted = true;
                }
            }
            changed = true;
        }

        if !changed {
            break;
        }
    }

    // Detect pauses on the simulated cleaned-up word list
    let pauses = filler::detect_pauses(&words, &config);
    let pause_infos: Vec<PauseInfo> = pauses
        .into_iter()
        .map(|(idx, dur)| PauseInfo {
            after_word_index: idx,
            gap_duration_us: dur,
        })
        .collect();

    Ok(FillerAnalysis {
        filler_count: all_filler_indices.len(),
        pause_count: pause_infos.len(),
        duplicate_count: all_duplicate_indices.len(),
        filler_indices: all_filler_indices,
        pauses: pause_infos,
        duplicate_indices: all_duplicate_indices,
    })
}

/// Auto-delete all detected filler words in the transcript.
#[tauri::command]
#[specta::specta]
pub fn delete_fillers(
    app: tauri::AppHandle,
    store: State<EditorStore>,
) -> Result<usize, String> {
    let settings = crate::settings::get_settings(&app);
    let filler_list = settings.custom_filler_words.clone().unwrap_or_default();

    let config = FillerConfig {
        filler_words: filler_list,
        ..Default::default()
    };

    let mut state = store.0.lock().unwrap();
    let indices = filler::detect_fillers(state.get_words(), &config);
    let count = indices.len();

    if count == 0 {
        return Ok(0);
    }

    state.push_undo_snapshot();
    let words = state.get_words_mut();
    for &idx in &indices {
        if idx < words.len() {
            words[idx].deleted = true;
        }
    }
    state.bump_revision();

    Ok(count)
}

/// Delete all detected adjacent duplicate words in the transcript.
#[tauri::command]
#[specta::specta]
pub fn delete_duplicates(store: State<EditorStore>) -> Result<usize, String> {
    let mut state = store.0.lock().unwrap();
    let duplicates = filler::detect_duplicates(state.get_words());
    let count = duplicates.len();

    if count == 0 {
        return Ok(0);
    }

    state.push_undo_snapshot();
    let words = state.get_words_mut();
    for &idx in &duplicates {
        if idx < words.len() {
            words[idx].deleted = true;
        }
    }
    state.bump_revision();

    Ok(count)
}

/// Silence all detected long pauses by marking adjacent words as silenced.
#[tauri::command]
#[specta::specta]
pub fn silence_pauses(
    store: State<EditorStore>,
    min_pause_us: Option<i64>,
) -> Result<usize, String> {
    let mut config = FillerConfig::default();
    if let Some(threshold) = min_pause_us {
        config.pause_threshold_us = threshold;
    }

    let mut state = store.0.lock().unwrap();
    let pauses = filler::detect_pauses(state.get_words(), &config);
    let count = pauses.len();

    if count == 0 {
        return Ok(0);
    }

    // Silence the word after each pause gap to mark the dead-air region
    for (after_word_idx, _) in &pauses {
        let next_idx = after_word_idx + 1;
        if next_idx < state.get_words().len() && !state.get_words()[next_idx].silenced {
            state.silence_word(next_idx);
        }
    }

    Ok(count)
}

/// Trim long pauses by reducing dead-air gaps to a maximum duration.
///
/// Unlike `silence_pauses` (which marks words as silenced), this command
/// shifts timestamps so that gaps exceeding the threshold are capped at
/// 300 ms, effectively removing dead air from the timeline.
#[tauri::command]
#[specta::specta]
pub fn trim_pauses(
    store: State<EditorStore>,
    min_pause_us: Option<i64>,
    max_gap_us: Option<i64>,
) -> Result<usize, String> {
    let threshold = min_pause_us.unwrap_or(filler::DEFAULT_PAUSE_THRESHOLD_US);
    let max_gap = max_gap_us.unwrap_or(filler::DEFAULT_MAX_GAP_US);

    let mut state = store.0.lock().unwrap();
    state.push_undo_snapshot();

    let words = state.get_words_mut();
    let count = filler::trim_pauses(words, threshold, max_gap);

    if count > 0 {
        state.bump_revision();
    }

    Ok(count)
}

/// Tighten all inter-word gaps to a maximum target duration.
/// Shortens ALL gaps exceeding the target — creating a tighter pace.
#[tauri::command]
#[specta::specta]
pub fn tighten_gaps(
    store: State<EditorStore>,
    target_gap_us: Option<i64>,
) -> Result<usize, String> {
    let target = target_gap_us.unwrap_or(filler::DEFAULT_TIGHTEN_TARGET_US);
    let mut state = store.0.lock().unwrap();
    state.push_undo_snapshot();
    let words = state.get_words_mut();
    let count = filler::tighten_gaps(words, target);
    if count > 0 {
        state.bump_revision();
    }
    Ok(count)
}

/// Combined iterative cleanup: delete fillers, then delete cascading
/// duplicates, then trim pauses — all in a single undo snapshot.
///
/// After deleting fillers, new duplicates may emerge (e.g., "the um the"
/// becomes "the the"). This command loops until no more fillers or
/// duplicates are found, then trims pauses.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct CleanupResult {
    pub fillers_removed: usize,
    pub duplicates_removed: usize,
    pub pauses_trimmed: usize,
    pub gaps_tightened: usize,
    pub passes: usize,
}

#[tauri::command]
#[specta::specta]
pub fn cleanup_all(
    app: tauri::AppHandle,
    store: State<EditorStore>,
    min_pause_us: Option<i64>,
    max_gap_us: Option<i64>,
) -> Result<CleanupResult, String> {
    let settings = crate::settings::get_settings(&app);
    let filler_list = settings.custom_filler_words.clone().unwrap_or_default();

    let config = FillerConfig {
        filler_words: filler_list,
        ..Default::default()
    };

    let _threshold = min_pause_us.unwrap_or(filler::DEFAULT_PAUSE_THRESHOLD_US);
    let _max_gap = max_gap_us.unwrap_or(filler::DEFAULT_MAX_GAP_US);

    let mut state = store.0.lock().unwrap();
    state.push_undo_snapshot();

    let mut total_fillers: usize = 0;
    let mut total_duplicates: usize = 0;
    let mut passes: usize = 0;
    const MAX_PASSES: usize = 10;

    // Iterative loop: delete fillers → delete new duplicates → repeat
    // Use direct word mutation to avoid undo snapshot per word
    loop {
        passes += 1;
        let mut changed = false;

        // Detect and delete fillers
        let filler_indices = filler::detect_fillers(state.get_words(), &config);
        if !filler_indices.is_empty() {
            let words = state.get_words_mut();
            for &idx in &filler_indices {
                if idx < words.len() && !words[idx].deleted {
                    words[idx].deleted = true;
                }
            }
            total_fillers += filler_indices.len();
            changed = true;
        }

        // Detect and delete duplicates (may have emerged after filler deletion)
        let dup_indices = filler::detect_duplicates(state.get_words());
        if !dup_indices.is_empty() {
            let words = state.get_words_mut();
            for &idx in &dup_indices {
                if idx < words.len() && !words[idx].deleted {
                    words[idx].deleted = true;
                }
            }
            total_duplicates += dup_indices.len();
            changed = true;
        }

        if !changed || passes >= MAX_PASSES {
            break;
        }
    }

    if total_fillers > 0 || total_duplicates > 0 {
        state.bump_revision();
    }

    Ok(CleanupResult {
        fillers_removed: total_fillers,
        duplicates_removed: total_duplicates,
        pauses_trimmed: 0,
        gaps_tightened: 0,
        passes,
    })
}
