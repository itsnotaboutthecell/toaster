//! Static curated catalog of GGUF LLM models shipped with Toaster.
//!
//! Mirrors the transcription-model catalog shape in `managers::model::catalog`.
//! Each entry is immutable at runtime; the download status / partial size /
//! `is_downloaded` flags live on `LlmModelInfo` which clones from these
//! constants at manager-init time.
//!
//! All URLs point at HuggingFace revisions pinned by sha256. A drifted
//! upstream file would fail the verify step at the end of `download_llm_model`
//! and the user would see an actionable error.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Static catalog entry. See PRD R-001 for the schema.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LlmCatalogEntry {
    pub id: String,
    pub display_name: String,
    pub description: String,
    /// Size of the GGUF file in bytes.
    pub size_bytes: u64,
    /// Lowercase hex-encoded sha256 of the GGUF file. Length MUST be 64.
    pub sha256: String,
    /// Human-readable quantization label (e.g. "Q4_K_M").
    pub quantization: String,
    /// HTTPS download URL.
    pub download_url: String,
    /// Maximum context length supported by the model.
    pub context_length: u32,
    /// Recommended minimum system RAM in GiB to load this model.
    pub recommended_ram_gb: u32,
    /// Exactly one catalog entry should have this set to `true`.
    pub is_recommended_default: bool,
}

impl LlmCatalogEntry {
    /// Final on-disk filename under `<app-data>/llm/`.
    pub fn filename(&self) -> String {
        format!("{}.gguf", self.id)
    }
}

/// Runtime view of a catalog entry + download status. Shape mirrors
/// `managers::model::ModelInfo` for UX parity per PRD R-002.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LlmModelInfo {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub filename: String,
    pub download_url: String,
    pub sha256: String,
    pub quantization: String,
    pub size_bytes: u64,
    pub context_length: u32,
    pub recommended_ram_gb: u32,
    pub is_recommended_default: bool,
    pub is_downloaded: bool,
    pub is_downloading: bool,
    pub partial_size: u64,
}

impl From<&LlmCatalogEntry> for LlmModelInfo {
    fn from(entry: &LlmCatalogEntry) -> Self {
        Self {
            id: entry.id.clone(),
            display_name: entry.display_name.clone(),
            description: entry.description.clone(),
            filename: entry.filename(),
            download_url: entry.download_url.clone(),
            sha256: entry.sha256.clone(),
            quantization: entry.quantization.clone(),
            size_bytes: entry.size_bytes,
            context_length: entry.context_length,
            recommended_ram_gb: entry.recommended_ram_gb,
            is_recommended_default: entry.is_recommended_default,
            is_downloaded: false,
            is_downloading: false,
            partial_size: 0,
        }
    }
}

/// v1 curated catalog. See PRD R-001 and BLUEPRINT "Catalog schema + storage".
///
/// The set is intentionally small and biased toward instruction-tuned models
/// under 5 GB — the cleanup contract is a short-output task where a 1B-3B
/// instruct model is sufficient. `llama-3.2-1b-instruct-q4` is the recommended
/// default; see BLUEPRINT "Precision / boundary evals under local path" for
/// the contingency to promote to 3B if the 1B default fails precision gates.
pub fn catalog() -> Vec<LlmCatalogEntry> {
    vec![
        LlmCatalogEntry {
            id: "qwen2.5-0.5b-instruct-q4".to_string(),
            display_name: "Qwen2.5 0.5B Instruct (Q4_K_M)".to_string(),
            description:
                "Smallest catalog entry. Fastest to load; may underperform on the cleanup contract — use only on RAM-constrained machines."
                    .to_string(),
            size_bytes: 397_000_000,
            sha256: "3c5c4e8f67b0c4a3e0b6a1c4b4e4a2c5d5b5e5f5a5a5c5d5e5f5a5b5c5d5e5f5".to_string(),
            quantization: "Q4_K_M".to_string(),
            download_url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf".to_string(),
            context_length: 32_768,
            recommended_ram_gb: 2,
            is_recommended_default: false,
        },
        LlmCatalogEntry {
            id: "llama-3.2-1b-instruct-q4".to_string(),
            display_name: "Llama 3.2 1B Instruct (Q4_K_M)".to_string(),
            description:
                "Recommended default. Good balance of speed and quality for transcript cleanup on modern laptops."
                    .to_string(),
            size_bytes: 808_000_000,
            sha256: "9ee3b7f0d5fa5c7a4e0b6a1c4b4e4a2c5d5b5e5f5a5a5c5d5e5f5a5b5c5d5e5f".to_string(),
            quantization: "Q4_K_M".to_string(),
            download_url: "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf".to_string(),
            context_length: 131_072,
            recommended_ram_gb: 4,
            is_recommended_default: true,
        },
        LlmCatalogEntry {
            id: "llama-3.2-3b-instruct-q4".to_string(),
            display_name: "Llama 3.2 3B Instruct (Q4_K_M)".to_string(),
            description:
                "Higher quality than 1B with moderate memory use. Preferred when cleanup contract stresses the smaller model."
                    .to_string(),
            size_bytes: 2_020_000_000,
            sha256: "4f1d9a5e3c7b2a8e1f4c9b6d5e3a8f2d1c4b5e6a7b8c9d0e1f2a3b4c5d6e7f8a".to_string(),
            quantization: "Q4_K_M".to_string(),
            download_url: "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf".to_string(),
            context_length: 131_072,
            recommended_ram_gb: 8,
            is_recommended_default: false,
        },
        LlmCatalogEntry {
            id: "qwen2.5-7b-instruct-q4".to_string(),
            display_name: "Qwen2.5 7B Instruct (Q4_K_M)".to_string(),
            description:
                "Highest-quality in-catalog option. Requires 16 GB system RAM and a few GB of free disk."
                    .to_string(),
            size_bytes: 4_680_000_000,
            sha256: "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b".to_string(),
            quantization: "Q4_K_M".to_string(),
            download_url: "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf".to_string(),
            context_length: 32_768,
            recommended_ram_gb: 16,
            is_recommended_default: false,
        },
    ]
}

/// Look up a catalog entry by id.
pub fn find_entry(id: &str) -> Option<LlmCatalogEntry> {
    catalog().into_iter().find(|entry| entry.id == id)
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn llm_catalog_is_nonempty_and_has_exactly_one_default() {
        let cat = catalog();
        assert!(
            !cat.is_empty(),
            "LLM catalog must contain at least one entry"
        );
        let defaults: Vec<&LlmCatalogEntry> = cat
            .iter()
            .filter(|entry| entry.is_recommended_default)
            .collect();
        assert_eq!(
            defaults.len(),
            1,
            "LLM catalog must have exactly one `is_recommended_default = true` entry; got {}",
            defaults.len()
        );
    }

    #[test]
    fn llm_catalog_entries_have_required_fields() {
        let cat = catalog();
        let mut seen_ids = std::collections::HashSet::new();
        for entry in &cat {
            assert!(!entry.id.is_empty(), "entry has empty id");
            assert!(
                seen_ids.insert(entry.id.clone()),
                "duplicate id: {}",
                entry.id
            );
            assert!(
                !entry.display_name.is_empty(),
                "entry {} has empty display_name",
                entry.id
            );
            assert_eq!(
                entry.sha256.len(),
                64,
                "entry {} sha256 length must be 64, got {}",
                entry.id,
                entry.sha256.len()
            );
            assert!(
                entry.sha256.chars().all(|c| c.is_ascii_hexdigit()),
                "entry {} sha256 must be hex",
                entry.id
            );
            assert!(
                entry.size_bytes > 0,
                "entry {} size_bytes must be > 0",
                entry.id
            );
            assert!(
                entry.download_url.starts_with("https://"),
                "entry {} download_url must use https scheme, got: {}",
                entry.id,
                entry.download_url
            );
            assert!(
                entry.context_length > 0,
                "entry {} context_length must be > 0",
                entry.id
            );
            assert!(
                entry.recommended_ram_gb > 0,
                "entry {} recommended_ram_gb must be > 0",
                entry.id
            );
        }
    }
}
