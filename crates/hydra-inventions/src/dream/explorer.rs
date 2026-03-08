//! AlternativeExplorer — what-if scenario exploration.

use serde::{Deserialize, Serialize};

/// A what-if scenario to explore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub description: String,
    pub original_action: String,
    pub alternative_action: String,
    pub context: serde_json::Value,
}

/// Result of exploring a scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub predicted_outcome: String,
    pub confidence: f32,
    pub better_than_original: bool,
    pub risk_delta: f32,
}

/// Explores alternative approaches
pub struct AlternativeExplorer {
    explored: parking_lot::Mutex<Vec<(Scenario, ScenarioResult)>>,
}

impl AlternativeExplorer {
    pub fn new() -> Self {
        Self {
            explored: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// Explore a what-if scenario (simulated)
    pub fn explore(&self, scenario: Scenario) -> ScenarioResult {
        // In production: run through LLM or compiled model
        let result = ScenarioResult {
            scenario_id: scenario.id.clone(),
            predicted_outcome: format!(
                "If {} instead of {}, likely outcome: moderate improvement",
                scenario.alternative_action, scenario.original_action
            ),
            confidence: 0.6,
            better_than_original: false,
            risk_delta: 0.0,
        };

        self.explored.lock().push((scenario, result.clone()));
        result
    }

    /// Get all explored scenarios
    pub fn history(&self) -> Vec<(Scenario, ScenarioResult)> {
        self.explored.lock().clone()
    }

    /// Count explored scenarios
    pub fn count(&self) -> usize {
        self.explored.lock().len()
    }
}

impl Default for AlternativeExplorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alternative_exploration() {
        let explorer = AlternativeExplorer::new();
        let scenario = Scenario {
            id: "s1".into(),
            description: "What if we used Sonnet instead of Haiku?".into(),
            original_action: "haiku_classify".into(),
            alternative_action: "sonnet_classify".into(),
            context: serde_json::json!({"task": "classification"}),
        };

        let result = explorer.explore(scenario);
        assert_eq!(result.scenario_id, "s1");
        assert!(result.confidence > 0.0);
        assert_eq!(explorer.count(), 1);
    }
}
