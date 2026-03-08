use dashmap::DashMap;

use crate::profile::{builtin_profiles, ModelProfile};

/// Registry of all available models
pub struct ModelRegistry {
    models: DashMap<String, ModelProfile>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        let registry = Self {
            models: DashMap::new(),
        };
        // Register built-in profiles
        for profile in builtin_profiles() {
            registry.register(profile);
        }
        registry
    }

    pub fn empty() -> Self {
        Self {
            models: DashMap::new(),
        }
    }

    pub fn register(&self, profile: ModelProfile) {
        self.models.insert(profile.id.clone(), profile);
    }

    pub fn get(&self, model_id: &str) -> Option<ModelProfile> {
        self.models.get(model_id).map(|e| e.value().clone())
    }

    pub fn list_all(&self) -> Vec<ModelProfile> {
        self.models.iter().map(|e| e.value().clone()).collect()
    }

    pub fn list_available(&self) -> Vec<ModelProfile> {
        self.models
            .iter()
            .filter(|e| e.is_usable())
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn mark_unavailable(&self, model_id: &str) {
        if let Some(mut entry) = self.models.get_mut(model_id) {
            entry.available = false;
        }
    }

    pub fn mark_rate_limited(&self, model_id: &str) {
        if let Some(mut entry) = self.models.get_mut(model_id) {
            entry.rate_limited = true;
        }
    }

    pub fn mark_available(&self, model_id: &str) {
        if let Some(mut entry) = self.models.get_mut(model_id) {
            entry.available = true;
            entry.rate_limited = false;
        }
    }

    pub fn count(&self) -> usize {
        self.models.len()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
