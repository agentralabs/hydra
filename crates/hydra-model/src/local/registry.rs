//! Local model registry — metadata, capabilities, and memory requirements.

use serde::{Deserialize, Serialize};

use crate::profile::{ModelCapabilities, ModelProfile, PrivacyLevel};

/// Memory requirement tier for local models
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryTier {
    /// < 2 GB VRAM — tiny models
    Tiny,
    /// 2-4 GB VRAM — small models (phi-3-mini, gemma-2b)
    Small,
    /// 4-8 GB VRAM — medium models (llama3-8b, mistral-7b)
    Medium,
    /// 8-16 GB VRAM — large models (llama3-70b Q4, codellama-34b)
    Large,
    /// 16+ GB VRAM — very large models
    XLarge,
}

impl MemoryTier {
    /// Minimum VRAM in MB needed for this tier
    pub fn min_vram_mb(&self) -> u64 {
        match self {
            Self::Tiny => 512,
            Self::Small => 2048,
            Self::Medium => 4096,
            Self::Large => 8192,
            Self::XLarge => 16384,
        }
    }
}

/// Metadata for a local model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelMeta {
    pub ollama_name: String,
    pub display_name: String,
    pub memory_tier: MemoryTier,
    pub vram_mb: u64,
    pub capabilities: ModelCapabilities,
    pub context_window: u32,
    pub latency_ms: u32,
}

/// Resource profile for selecting which local models to load
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalModelProfile {
    /// No local models
    Minimal,
    /// One small/fast model (phi3)
    Standard,
    /// Fast + strong model (phi3 + llama3)
    Performance,
    /// All available models
    Unlimited,
}

/// Get the list of known local models with their metadata
pub fn known_local_models() -> Vec<LocalModelMeta> {
    vec![
        LocalModelMeta {
            ollama_name: "phi3".into(),
            display_name: "Phi-3 Mini".into(),
            memory_tier: MemoryTier::Small,
            vram_mb: 2400,
            capabilities: ModelCapabilities {
                reasoning: 72,
                code: 70,
                creative: 65,
                math: 70,
                instruction_following: 75,
                vision: false,
                function_calling: false,
                context_window: 4096,
                max_output_tokens: 2048,
            },
            context_window: 4096,
            latency_ms: 800,
        },
        LocalModelMeta {
            ollama_name: "llama3".into(),
            display_name: "Llama 3 8B".into(),
            memory_tier: MemoryTier::Medium,
            vram_mb: 4800,
            capabilities: ModelCapabilities {
                reasoning: 78,
                code: 76,
                creative: 72,
                math: 75,
                instruction_following: 80,
                vision: false,
                function_calling: false,
                context_window: 8192,
                max_output_tokens: 4096,
            },
            context_window: 8192,
            latency_ms: 1500,
        },
        LocalModelMeta {
            ollama_name: "mistral".into(),
            display_name: "Mistral 7B".into(),
            memory_tier: MemoryTier::Medium,
            vram_mb: 4200,
            capabilities: ModelCapabilities {
                reasoning: 75,
                code: 73,
                creative: 70,
                math: 72,
                instruction_following: 78,
                vision: false,
                function_calling: false,
                context_window: 8192,
                max_output_tokens: 4096,
            },
            context_window: 8192,
            latency_ms: 1200,
        },
        LocalModelMeta {
            ollama_name: "codellama".into(),
            display_name: "Code Llama 7B".into(),
            memory_tier: MemoryTier::Medium,
            vram_mb: 4500,
            capabilities: ModelCapabilities {
                reasoning: 65,
                code: 85,
                creative: 50,
                math: 70,
                instruction_following: 72,
                vision: false,
                function_calling: false,
                context_window: 16384,
                max_output_tokens: 4096,
            },
            context_window: 16384,
            latency_ms: 1300,
        },
    ]
}

/// Select which models to load based on the resource profile
pub fn models_for_profile(profile: LocalModelProfile) -> Vec<String> {
    match profile {
        LocalModelProfile::Minimal => vec![],
        LocalModelProfile::Standard => vec!["phi3".into()],
        LocalModelProfile::Performance => vec!["phi3".into(), "llama3".into()],
        LocalModelProfile::Unlimited => known_local_models()
            .into_iter()
            .map(|m| m.ollama_name)
            .collect(),
    }
}

/// Convert a LocalModelMeta to a ModelProfile for the registry
pub fn to_model_profile(meta: &LocalModelMeta) -> ModelProfile {
    ModelProfile {
        id: format!("local-{}", meta.ollama_name),
        name: meta.display_name.clone(),
        provider: "ollama".into(),
        capabilities: meta.capabilities.clone(),
        cost_per_1k_input: 0.0,
        cost_per_1k_output: 0.0,
        latency_ms: meta.latency_ms,
        privacy: PrivacyLevel::Local,
        available: false, // Set to true after Ollama confirms it's loaded
        rate_limited: false,
    }
}

/// Find model metadata by ollama name
pub fn find_model(ollama_name: &str) -> Option<LocalModelMeta> {
    known_local_models()
        .into_iter()
        .find(|m| m.ollama_name == ollama_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_model_registry() {
        let models = known_local_models();
        assert_eq!(models.len(), 4);
        assert_eq!(models[0].ollama_name, "phi3");
        assert_eq!(models[1].ollama_name, "llama3");
        assert_eq!(models[2].ollama_name, "mistral");
        assert_eq!(models[3].ollama_name, "codellama");
    }

    #[test]
    fn test_model_memory_requirements() {
        let phi3 = find_model("phi3").unwrap();
        assert_eq!(phi3.memory_tier, MemoryTier::Small);
        assert!(phi3.vram_mb < 3000);

        let llama3 = find_model("llama3").unwrap();
        assert_eq!(llama3.memory_tier, MemoryTier::Medium);
        assert!(llama3.vram_mb >= 4096);
    }

    #[test]
    fn test_profile_model_selection() {
        assert!(models_for_profile(LocalModelProfile::Minimal).is_empty());
        assert_eq!(
            models_for_profile(LocalModelProfile::Standard),
            vec!["phi3"]
        );
        assert_eq!(
            models_for_profile(LocalModelProfile::Performance),
            vec!["phi3", "llama3"]
        );
        assert_eq!(models_for_profile(LocalModelProfile::Unlimited).len(), 4);
    }

    #[test]
    fn test_to_model_profile() {
        let meta = find_model("phi3").unwrap();
        let profile = to_model_profile(&meta);
        assert_eq!(profile.id, "local-phi3");
        assert_eq!(profile.provider, "ollama");
        assert_eq!(profile.privacy, PrivacyLevel::Local);
        assert_eq!(profile.cost_per_1k_input, 0.0);
        assert!(!profile.available); // Not confirmed yet
    }

    #[test]
    fn test_memory_tier_ordering() {
        assert!(MemoryTier::Tiny < MemoryTier::Small);
        assert!(MemoryTier::Small < MemoryTier::Medium);
        assert!(MemoryTier::Medium < MemoryTier::Large);
    }

    #[test]
    fn test_find_model_not_found() {
        assert!(find_model("nonexistent").is_none());
    }
}
