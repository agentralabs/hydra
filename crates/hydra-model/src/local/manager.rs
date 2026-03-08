//! LocalModelManager — detect Ollama, manage local model lifecycle.

use std::collections::HashSet;

use crate::local::ollama::OllamaClient;
use crate::local::registry::{
    find_model, known_local_models, models_for_profile, to_model_profile, LocalModelProfile,
};
use crate::profile::ModelProfile;
use crate::registry::ModelRegistry;

/// Manages local model detection, loading, and registration.
pub struct LocalModelManager {
    client: OllamaClient,
    profile: LocalModelProfile,
    loaded_models: HashSet<String>,
}

impl LocalModelManager {
    pub fn new(profile: LocalModelProfile) -> Self {
        Self {
            client: OllamaClient::new(),
            profile,
            loaded_models: HashSet::new(),
        }
    }

    pub fn with_client(client: OllamaClient, profile: LocalModelProfile) -> Self {
        Self {
            client,
            profile,
            loaded_models: HashSet::new(),
        }
    }

    /// Check if Ollama is installed and running
    pub async fn is_ollama_available(&self) -> bool {
        self.client.is_available().await
    }

    /// Detect which models are available in Ollama and register them.
    /// Returns the list of model profiles that were registered.
    pub async fn detect_and_register(&mut self, registry: &ModelRegistry) -> Vec<ModelProfile> {
        let mut registered = Vec::new();

        // Check if Ollama is running
        if !self.client.is_available().await {
            tracing::info!("Ollama not available, skipping local model detection");
            return registered;
        }

        // Get installed models from Ollama
        let installed = match self.client.list_models().await {
            Ok(models) => models,
            Err(e) => {
                tracing::warn!("Failed to list Ollama models: {}", e);
                return registered;
            }
        };

        let installed_names: HashSet<String> = installed
            .iter()
            .map(|m| {
                // Ollama returns "phi3:latest" — strip the tag
                m.name.split(':').next().unwrap_or(&m.name).to_string()
            })
            .collect();

        // Get desired models for the current profile
        let desired = models_for_profile(self.profile);

        // Register models that are both desired and installed
        for model_name in &desired {
            if installed_names.contains(model_name.as_str()) {
                if let Some(meta) = find_model(model_name) {
                    let mut profile = to_model_profile(&meta);
                    profile.available = true;
                    registry.register(profile.clone());
                    self.loaded_models.insert(model_name.clone());
                    registered.push(profile);
                    tracing::info!(model = %model_name, "Registered local model");
                }
            } else {
                tracing::debug!(model = %model_name, "Desired model not installed in Ollama");
            }
        }

        registered
    }

    /// Register all known local models (regardless of whether they're in Ollama).
    /// Sets `available = false` — caller must check availability.
    pub fn register_all_known(&self, registry: &ModelRegistry) {
        for meta in known_local_models() {
            let profile = to_model_profile(&meta);
            registry.register(profile);
        }
    }

    /// Get the Ollama client for direct use
    pub fn client(&self) -> &OllamaClient {
        &self.client
    }

    /// Get the set of loaded model names
    pub fn loaded_models(&self) -> &HashSet<String> {
        &self.loaded_models
    }

    /// Current profile
    pub fn profile(&self) -> LocalModelProfile {
        self.profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let mgr = LocalModelManager::new(LocalModelProfile::Standard);
        assert_eq!(mgr.profile(), LocalModelProfile::Standard);
        assert!(mgr.loaded_models().is_empty());
    }

    #[test]
    fn test_register_all_known() {
        let mgr = LocalModelManager::new(LocalModelProfile::Unlimited);
        let registry = ModelRegistry::empty();
        mgr.register_all_known(&registry);
        // Should have all 4 local models
        assert!(registry.get("local-phi3").is_some());
        assert!(registry.get("local-llama3").is_some());
        assert!(registry.get("local-mistral").is_some());
        assert!(registry.get("local-codellama").is_some());
        // None should be marked available (not confirmed via Ollama)
        assert!(!registry.get("local-phi3").unwrap().available);
    }

    #[tokio::test]
    async fn test_detect_when_ollama_unavailable() {
        let client = OllamaClient::with_url("http://localhost:19999");
        let mut mgr = LocalModelManager::with_client(client, LocalModelProfile::Standard);
        let registry = ModelRegistry::empty();
        let registered = mgr.detect_and_register(&registry).await;
        assert!(registered.is_empty());
    }

    #[tokio::test]
    #[cfg(feature = "local-llm")]
    async fn test_live_detect_and_register() {
        let mut mgr = LocalModelManager::new(LocalModelProfile::Unlimited);
        if !mgr.is_ollama_available().await {
            eprintln!("Ollama not running, skipping live test");
            return;
        }
        let registry = ModelRegistry::empty();
        let registered = mgr.detect_and_register(&registry).await;
        println!("Registered {} local models", registered.len());
        for profile in &registered {
            println!("  {} (available: {})", profile.id, profile.available);
            assert!(profile.available);
        }
    }
}
