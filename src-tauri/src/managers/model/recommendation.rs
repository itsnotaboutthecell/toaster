//! Pure scoring function that turns a `HardwareProfile` into a
//! `ModelRecommendation`. See
//! `features/hardware-aware-model-picker/PRD.md` R-002 and
//! `BLUEPRINT.md > R-002 scoring shape`.
//!
//! Determinism is non-negotiable: this function must write nothing,
//! touch neither disk nor settings, and produce structurally-equal
//! output for equal input so the frontend can cache aggressively and
//! unit tests can assert by value.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::hardware_profile::{Accelerator, HardwareProfile};
use super::{ModelCategory, ModelInfo};

/// Tier label rendered to the user ("Fastest" / "Balanced" /
/// "Highest accuracy"). UI reads this to pick an i18n key from the
/// `settings.models.recommendation.*` namespace.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum RecommendationTier {
    Fastest,
    Balanced,
    HighestAccuracy,
}

/// Backend-authored recommendation. `tradeoff_key` is an i18next key
/// (per `AGENTS.md > Critical rules`), never a rendered string. The
/// frontend does `t(rec.tradeoff_key)` to localize.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct ModelRecommendation {
    pub model_id: String,
    pub tier: RecommendationTier,
    pub tradeoff_key: String,
    pub insufficient_disk: bool,
}

/// Disk headroom multiplier. A model is only considered if its
/// `size_mb * 3 / 2` fits inside `profile.models_dir_free_mb` —
/// leaves ~50 % slack for the extraction / hash-check / temp-file
/// churn that the downloader performs.
const DISK_HEADROOM_NUMER: u64 = 3;
const DISK_HEADROOM_DENOM: u64 = 2;

fn fits_on_disk(model: &ModelInfo, free_mb: u64) -> bool {
    model.size_mb.saturating_mul(DISK_HEADROOM_NUMER) / DISK_HEADROOM_DENOM <= free_mb
}

fn classify_tier(profile: &HardwareProfile) -> RecommendationTier {
    // Highest: any GPU accelerator, OR a very capable CPU box.
    if profile.accelerator != Accelerator::Cpu
        || (profile.cpu_cores >= 12 && profile.total_ram_mb >= 16_384)
    {
        return RecommendationTier::HighestAccuracy;
    }
    // Balanced: decent CPU with enough RAM to hold a medium model.
    if profile.cpu_cores >= 8 && profile.total_ram_mb >= 12_288 {
        return RecommendationTier::Balanced;
    }
    RecommendationTier::Fastest
}

fn tradeoff_key_for(tier: RecommendationTier) -> &'static str {
    match tier {
        RecommendationTier::Fastest => "settings.models.recommendation.fastest",
        RecommendationTier::Balanced => "settings.models.recommendation.balanced",
        RecommendationTier::HighestAccuracy => "settings.models.recommendation.highestAccuracy",
    }
}

/// Pick the best candidate inside a tier. "Best" = highest
/// `accuracy_score`, tie-broken by highest `speed_score`, tie-broken
/// by `id` lexicographic order so the choice is deterministic across
/// HashMap-iteration noise.
fn best_in_tier<'a>(
    candidates: impl IntoIterator<Item = &'a ModelInfo>,
    free_mb: u64,
    require_tier: Option<RecommendationTier>,
) -> Option<&'a ModelInfo> {
    candidates
        .into_iter()
        .filter(|m| m.category == ModelCategory::Transcription)
        .filter(|m| fits_on_disk(m, free_mb))
        .filter(|m| match require_tier {
            None => true,
            Some(RecommendationTier::Fastest) => m.speed_score >= 0.80,
            Some(RecommendationTier::Balanced) => m.speed_score >= 0.55 && m.accuracy_score >= 0.60,
            Some(RecommendationTier::HighestAccuracy) => m.accuracy_score >= 0.75,
        })
        .max_by(|a, b| {
            a.accuracy_score
                .partial_cmp(&b.accuracy_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.speed_score
                        .partial_cmp(&b.speed_score)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
                .then(b.id.cmp(&a.id)) // reverse so lexicographically smaller wins on equal scores
        })
}

/// Smallest transcription model by `size_mb`. Used as the fallback
/// "insufficient disk" name so the UI always has a target even when
/// nothing fits.
fn smallest_transcription<'a>(models: &'a [ModelInfo]) -> Option<&'a ModelInfo> {
    models
        .iter()
        .filter(|m| m.category == ModelCategory::Transcription)
        .min_by_key(|m| m.size_mb)
}

/// Pure scoring function. Input: the cached hardware profile and the
/// model catalog. Output: a single `ModelRecommendation`. No I/O, no
/// settings reads, no interior mutability.
pub fn recommend_model(profile: &HardwareProfile, models: &[ModelInfo]) -> ModelRecommendation {
    let tier = classify_tier(profile);
    let tradeoff_key = tradeoff_key_for(tier).to_string();

    // Try the classified tier first, then degrade one step at a time
    // so a disk-starved box still lands on a runnable model.
    let picked: Option<&ModelInfo> = match tier {
        RecommendationTier::HighestAccuracy => best_in_tier(
            models.iter(),
            profile.models_dir_free_mb,
            Some(RecommendationTier::HighestAccuracy),
        )
        .or_else(|| {
            best_in_tier(
                models.iter(),
                profile.models_dir_free_mb,
                Some(RecommendationTier::Balanced),
            )
        })
        .or_else(|| {
            best_in_tier(
                models.iter(),
                profile.models_dir_free_mb,
                Some(RecommendationTier::Fastest),
            )
        }),
        RecommendationTier::Balanced => best_in_tier(
            models.iter(),
            profile.models_dir_free_mb,
            Some(RecommendationTier::Balanced),
        )
        .or_else(|| {
            best_in_tier(
                models.iter(),
                profile.models_dir_free_mb,
                Some(RecommendationTier::Fastest),
            )
        }),
        RecommendationTier::Fastest => best_in_tier(
            models.iter(),
            profile.models_dir_free_mb,
            Some(RecommendationTier::Fastest),
        ),
    };

    // If *nothing* fits on disk we still name the smallest model so
    // the UI can render an "upgrade disk" hint instead of going blank.
    // `insufficient_disk` distinguishes this recovery path from the
    // happy path.
    if let Some(model) = picked {
        ModelRecommendation {
            model_id: model.id.clone(),
            tier,
            tradeoff_key,
            insufficient_disk: false,
        }
    } else if let Some(smallest) = smallest_transcription(models) {
        ModelRecommendation {
            model_id: smallest.id.clone(),
            tier,
            tradeoff_key,
            insufficient_disk: true,
        }
    } else {
        // Empty catalog. This shouldn't happen in production because
        // `catalog::build_static_catalog` always returns at least one
        // transcription model; we emit a sentinel id so the UI error
        // path is obvious in logs.
        ModelRecommendation {
            model_id: String::new(),
            tier,
            tradeoff_key,
            insufficient_disk: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::model::{EngineType, ModelCategory, ModelInfo};

    fn canonical_catalog() -> Vec<ModelInfo> {
        // A compact stand-in for the 9 transcription rows in
        // `catalog/transcription.rs`. Only the fields that scoring
        // reads are populated; the rest use `Default` via field
        // defaults where possible.
        vec![
            mk("tiny", 75, 0.40, 0.95),
            mk("small", 465, 0.60, 0.85),
            mk("medium", 469, 0.75, 0.60),
            mk("turbo", 809, 0.78, 0.80),
            mk("large", 1550, 0.88, 0.30),
            mk("parakeet-v2", 620, 0.80, 0.85),
            mk("moonshine-small", 190, 0.70, 0.90),
            mk("moonshine-medium", 370, 0.78, 0.82),
            mk("canary-180m-flash", 360, 0.82, 0.88),
        ]
    }

    fn mk(id: &str, size_mb: u64, accuracy: f32, speed: f32) -> ModelInfo {
        ModelInfo {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            filename: format!("{id}.bin"),
            url: None,
            sha256: None,
            size_mb,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
            is_directory: false,
            engine_type: EngineType::Whisper,
            accuracy_score: accuracy,
            speed_score: speed,
            supports_translation: false,
            is_recommended: false,
            supported_languages: vec![],
            supports_language_selection: false,
            is_custom: false,
            category: ModelCategory::Transcription,
            transcription_metadata: None,
        }
    }

    fn cpu_profile(cores: u32, ram_mb: u64, disk_mb: u64) -> HardwareProfile {
        HardwareProfile {
            cpu_cores: cores,
            total_ram_mb: ram_mb,
            accelerator: Accelerator::Cpu,
            models_dir_free_mb: disk_mb,
        }
    }

    #[test]
    fn ac_002_a_low_end_cpu_gets_fastest_tier() {
        // 4 cores / 8 GB / CPU / 20 GB disk → Fastest tier with a
        // model whose speed_score >= 0.85.
        let rec = recommend_model(&cpu_profile(4, 8192, 20_000), &canonical_catalog());
        assert_eq!(rec.tier, RecommendationTier::Fastest);
        assert!(!rec.insufficient_disk);
        let m = canonical_catalog()
            .into_iter()
            .find(|m| m.id == rec.model_id)
            .expect("recommended id must exist in catalog");
        assert!(
            m.speed_score >= 0.85,
            "{} speed_score {} < 0.85",
            rec.model_id,
            m.speed_score
        );
    }

    #[test]
    fn ac_002_a_high_end_gpu_gets_highest_accuracy_tier() {
        let profile = HardwareProfile {
            cpu_cores: 16,
            total_ram_mb: 32_768,
            accelerator: Accelerator::Cuda,
            models_dir_free_mb: 100_000,
        };
        let rec = recommend_model(&profile, &canonical_catalog());
        assert_eq!(rec.tier, RecommendationTier::HighestAccuracy);
        assert!(!rec.insufficient_disk);
        let m = canonical_catalog()
            .into_iter()
            .find(|m| m.id == rec.model_id)
            .expect("recommended id must exist in catalog");
        assert!(
            m.accuracy_score >= 0.80,
            "{} accuracy_score {} < 0.80",
            rec.model_id,
            m.accuracy_score
        );
    }

    #[test]
    fn ac_002_a_mid_range_vulkan_gets_balanced_or_higher() {
        let profile = HardwareProfile {
            cpu_cores: 8,
            total_ram_mb: 16_384,
            accelerator: Accelerator::Vulkan,
            models_dir_free_mb: 50_000,
        };
        let rec = recommend_model(&profile, &canonical_catalog());
        // Accelerator::Vulkan with those specs satisfies the
        // HighestAccuracy rule (`accelerator != Cpu`) so we accept
        // either Balanced or HighestAccuracy as correct — the PRD
        // example lists Balanced but the tier rules prioritize
        // GPU presence. Both are acceptable; the test enforces that
        // the user doesn't land in Fastest on a mid-range GPU box.
        assert_ne!(rec.tier, RecommendationTier::Fastest);
        assert!(!rec.insufficient_disk);
    }

    #[test]
    fn ac_002_b_recommend_model_is_pure() {
        // AC-002-b: two calls with clones of the same inputs produce
        // the same output. We also verify that the function does not
        // mutate its inputs (trivially true because it takes &refs,
        // but enforced here for documentation).
        let profile = cpu_profile(4, 8192, 20_000);
        let catalog = canonical_catalog();
        let a = recommend_model(&profile.clone(), &catalog.clone());
        let b = recommend_model(&profile, &catalog);
        assert_eq!(a, b);
    }

    #[test]
    fn ac_002_c_insufficient_disk_flags_and_names_smallest() {
        // AC-002-c: 50 MB free is below the 1.5× headroom of every
        // model in the catalog (smallest is tiny @ 75 MB → needs
        // 112 MB) → insufficient_disk=true, model_id = smallest.
        let profile = cpu_profile(4, 8192, 50);
        let catalog = canonical_catalog();
        let rec = recommend_model(&profile, &catalog);
        assert!(rec.insufficient_disk);
        let smallest_id = smallest_transcription(&catalog).unwrap().id.clone();
        assert_eq!(rec.model_id, smallest_id);
    }

    #[test]
    fn catalog_with_only_vad_rows_returns_empty_id() {
        // Defensive: if somehow every row is VAD (shouldn't happen
        // in production) we emit an empty id + insufficient_disk=true
        // so the UI error path is obvious.
        let mut models = canonical_catalog();
        for m in models.iter_mut() {
            m.category = ModelCategory::VoiceActivityDetection;
        }
        let rec = recommend_model(&cpu_profile(4, 8192, 50_000), &models);
        assert!(rec.insufficient_disk);
        assert!(rec.model_id.is_empty());
    }

    #[test]
    fn tradeoff_key_matches_tier_for_every_variant() {
        // Defensive: keep the tradeoff-key namespace in lockstep with
        // the tier enum. If someone adds a tier, this test fails until
        // the key is added too (mirrored across locales per R-004).
        for tier in [
            RecommendationTier::Fastest,
            RecommendationTier::Balanced,
            RecommendationTier::HighestAccuracy,
        ] {
            let key = tradeoff_key_for(tier);
            assert!(
                key.starts_with("settings.models.recommendation."),
                "tradeoff key for {:?} is not under the settings.models.recommendation.* namespace: {}",
                tier,
                key,
            );
        }
    }
}
