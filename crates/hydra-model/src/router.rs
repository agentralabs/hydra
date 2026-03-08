use crate::preferences::ModelPreferences;
use crate::profile::{ModelProfile, PrivacyLevel, TaskType};
use crate::registry::ModelRegistry;

/// Routing decision with selected model and alternatives
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub model: ModelProfile,
    pub score: f64,
    pub fallbacks: Vec<ModelProfile>,
    pub warnings: Vec<String>,
}

impl RoutingDecision {
    pub fn model_id(&self) -> &str {
        &self.model.id
    }

    pub fn model_cost(&self) -> f64 {
        self.model.cost_per_1k()
    }

    pub fn warns_about_cost(&self) -> bool {
        self.warnings.iter().any(|w| w.contains("cost"))
    }

    pub fn is_cloud_model(&self) -> bool {
        self.model.privacy == PrivacyLevel::Cloud
    }
}

/// Model router — routes tasks to the best model
pub struct ModelRouter {
    registry: ModelRegistry,
}

impl ModelRouter {
    pub fn new(registry: ModelRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &ModelRegistry {
        &self.registry
    }

    /// Route a task to the best model given user preferences
    /// Score = capability(40%) + cost(30%) + latency(20%) + privacy(10%)
    pub fn route(
        &self,
        task_type: TaskType,
        prefs: &ModelPreferences,
    ) -> Result<RoutingDecision, RouterError> {
        let mut candidates: Vec<(ModelProfile, f64)> = self
            .registry
            .list_available()
            .into_iter()
            .filter(|m| !prefs.is_blocked(&m.id))
            .filter(|m| {
                // Privacy enforcement — checked BEFORE routing
                if prefs.require_local && m.privacy == PrivacyLevel::Cloud {
                    return false;
                }
                m.privacy >= prefs.privacy_minimum
            })
            .filter(|m| {
                // Vision tasks need vision capability
                if task_type == TaskType::Vision && !m.capabilities.vision {
                    return false;
                }
                true
            })
            .map(|m| {
                let score = self.score_model(&m, task_type, prefs);
                (m, score)
            })
            .collect();

        if candidates.is_empty() {
            // Check if it's a privacy conflict
            let all_models = self.registry.list_all();
            let has_cloud_only = !all_models.is_empty()
                && all_models.iter().all(|m| m.privacy == PrivacyLevel::Cloud);
            if prefs.require_local && has_cloud_only {
                return Err(RouterError::PrivacyConflict);
            }
            return Err(RouterError::NoModelAvailable);
        }

        // Sort by score descending
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Boost preferred models to top if they're competitive (within 20% of best)
        let best_score = candidates[0].1;
        for pref_id in &prefs.preferred {
            if let Some(pos) = candidates
                .iter()
                .position(|(m, s)| m.id == *pref_id && *s >= best_score * 0.8)
            {
                if pos > 0 {
                    let item = candidates.remove(pos);
                    candidates.insert(0, item);
                }
            }
        }

        let (model, score) = candidates.remove(0);
        let fallbacks: Vec<ModelProfile> = candidates.into_iter().map(|(m, _)| m).collect();

        // Cost warnings
        let mut warnings = Vec::new();
        if let Some(max_cost) = prefs.max_cost_per_task {
            if model.cost_per_1k() > max_cost {
                warnings.push(format!(
                    "Model {} costs ${:.4}/1K tokens, which exceeds your limit of ${:.4}.",
                    model.id,
                    model.cost_per_1k(),
                    max_cost
                ));
            }
        }

        Ok(RoutingDecision {
            model,
            score,
            fallbacks,
            warnings,
        })
    }

    fn score_model(
        &self,
        model: &ModelProfile,
        task_type: TaskType,
        _prefs: &ModelPreferences,
    ) -> f64 {
        let capability = model.capabilities.score_for_task(task_type) as f64 / 100.0;
        let cost = 1.0 / (1.0 + model.cost_per_1k() * 100.0); // Cheaper = higher
        let latency = 1.0 / (1.0 + model.latency_ms as f64 / 1000.0);
        let privacy = match model.privacy {
            PrivacyLevel::AirGapped => 1.0,
            PrivacyLevel::Local => 0.8,
            PrivacyLevel::Cloud => 0.5,
        };

        capability * 0.4 + cost * 0.3 + latency * 0.2 + privacy * 0.1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterError {
    NoModelAvailable,
    PrivacyConflict,
    AllRateLimited,
}

impl std::fmt::Display for RouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoModelAvailable => write!(f, "No model is available for this task. All models may be offline or blocked by your preferences. Check model status."),
            Self::PrivacyConflict => write!(f, "Your privacy settings require local execution, but no local model is available. Install a local model or adjust privacy settings."),
            Self::AllRateLimited => write!(f, "All available models are currently rate-limited. Wait a moment and try again."),
        }
    }
}

impl std::error::Error for RouterError {}
