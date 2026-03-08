use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::profile::PrivacyLevel;

/// User preferences for model routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreferences {
    /// Preferred model IDs (tried first)
    pub preferred: Vec<String>,
    /// Blocked model IDs (never used)
    pub blocked: HashSet<String>,
    /// Require local-only execution (no cloud)
    pub require_local: bool,
    /// Maximum cost per task in USD
    pub max_cost_per_task: Option<f64>,
    /// Minimum privacy level required
    pub privacy_minimum: PrivacyLevel,
}

impl Default for ModelPreferences {
    fn default() -> Self {
        Self {
            preferred: vec![],
            blocked: HashSet::new(),
            require_local: false,
            max_cost_per_task: None,
            privacy_minimum: PrivacyLevel::Cloud, // Most permissive default
        }
    }
}

impl ModelPreferences {
    pub fn local_only() -> Self {
        Self {
            require_local: true,
            privacy_minimum: PrivacyLevel::Local,
            ..Default::default()
        }
    }

    pub fn is_blocked(&self, model_id: &str) -> bool {
        self.blocked.contains(model_id)
    }

    pub fn is_preferred(&self, model_id: &str) -> bool {
        self.preferred.iter().any(|p| p == model_id)
    }
}
