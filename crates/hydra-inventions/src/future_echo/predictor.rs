//! OutcomePredictor — simulate action chain consequences before execution.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::confidence::ConfidenceScore;

/// A single action in a chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub params: serde_json::Value,
    pub risk_level: f32,
}

/// A chain of actions to simulate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionChain {
    pub id: String,
    pub actions: Vec<Action>,
    pub context: HashMap<String, serde_json::Value>,
}

impl ActionChain {
    pub fn new(actions: Vec<Action>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            actions,
            context: HashMap::new(),
        }
    }

    pub fn with_context(mut self, key: &str, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    pub fn total_risk(&self) -> f32 {
        if self.actions.is_empty() {
            return 0.0;
        }
        // Compound risk: 1 - product of (1 - risk_i)
        let safe_prob: f32 = self
            .actions
            .iter()
            .map(|a| 1.0 - a.risk_level.clamp(0.0, 1.0))
            .product();
        1.0 - safe_prob
    }
}

/// A predicted outcome from simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedOutcome {
    pub chain_id: String,
    pub outcome_id: String,
    pub description: String,
    pub confidence: ConfidenceScore,
    pub side_effects: Vec<SideEffect>,
    pub risk_assessment: RiskAssessment,
    pub reversible: bool,
}

/// A potential side effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideEffect {
    pub description: String,
    pub probability: f32,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Risk assessment for predicted outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: f32,
    pub reversibility: f32,
    pub data_loss_risk: f32,
    pub recommendation: RiskRecommendation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskRecommendation {
    Safe,
    Proceed,
    Caution,
    Block,
}

/// Predicts outcomes of action chains
pub struct OutcomePredictor {
    predictions: parking_lot::RwLock<Vec<PredictedOutcome>>,
    cache: parking_lot::RwLock<HashMap<String, Vec<PredictedOutcome>>>,
}

impl OutcomePredictor {
    pub fn new() -> Self {
        Self {
            predictions: parking_lot::RwLock::new(Vec::new()),
            cache: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Predict outcomes for an action chain
    pub fn predict(&self, chain: &ActionChain) -> Vec<PredictedOutcome> {
        // Check cache
        if let Some(cached) = self.cache.read().get(&chain.id) {
            return cached.clone();
        }

        let mut outcomes = Vec::new();
        let total_risk = chain.total_risk();

        // Generate primary outcome (success)
        let success_confidence = (1.0 - total_risk).max(0.1);
        outcomes.push(PredictedOutcome {
            chain_id: chain.id.clone(),
            outcome_id: format!("{}-success", chain.id),
            description: format!("All {} actions complete successfully", chain.actions.len()),
            confidence: ConfidenceScore::new(success_confidence),
            side_effects: self.predict_side_effects(chain),
            risk_assessment: self.assess_risk(chain),
            reversible: chain.actions.iter().all(|a| a.risk_level < 0.5),
        });

        // Generate partial failure outcome if chain has multiple steps
        if chain.actions.len() > 1 {
            outcomes.push(PredictedOutcome {
                chain_id: chain.id.clone(),
                outcome_id: format!("{}-partial", chain.id),
                description: "Some actions fail, partial completion".into(),
                confidence: ConfidenceScore::new(total_risk * 0.6),
                side_effects: Vec::new(),
                risk_assessment: RiskAssessment {
                    overall_risk: total_risk,
                    reversibility: 0.5,
                    data_loss_risk: total_risk * 0.3,
                    recommendation: RiskRecommendation::Caution,
                },
                reversible: false,
            });
        }

        // Generate total failure outcome for risky chains
        if total_risk > 0.3 {
            outcomes.push(PredictedOutcome {
                chain_id: chain.id.clone(),
                outcome_id: format!("{}-failure", chain.id),
                description: "Action chain fails completely".into(),
                confidence: ConfidenceScore::new(total_risk * 0.4),
                side_effects: Vec::new(),
                risk_assessment: RiskAssessment {
                    overall_risk: total_risk,
                    reversibility: 0.2,
                    data_loss_risk: total_risk * 0.5,
                    recommendation: if total_risk > 0.7 {
                        RiskRecommendation::Block
                    } else {
                        RiskRecommendation::Caution
                    },
                },
                reversible: false,
            });
        }

        // Cache and store
        self.cache
            .write()
            .insert(chain.id.clone(), outcomes.clone());
        self.predictions.write().extend(outcomes.clone());

        outcomes
    }

    fn predict_side_effects(&self, chain: &ActionChain) -> Vec<SideEffect> {
        let mut effects = Vec::new();
        for action in &chain.actions {
            if action.risk_level > 0.3 {
                effects.push(SideEffect {
                    description: format!("'{}' may have unintended consequences", action.name),
                    probability: action.risk_level,
                    severity: if action.risk_level > 0.7 {
                        Severity::High
                    } else {
                        Severity::Medium
                    },
                });
            }
        }
        effects
    }

    fn assess_risk(&self, chain: &ActionChain) -> RiskAssessment {
        let total_risk = chain.total_risk();
        let recommendation = match total_risk {
            r if r < 0.1 => RiskRecommendation::Safe,
            r if r < 0.3 => RiskRecommendation::Proceed,
            r if r < 0.7 => RiskRecommendation::Caution,
            _ => RiskRecommendation::Block,
        };

        RiskAssessment {
            overall_risk: total_risk,
            reversibility: 1.0 - total_risk,
            data_loss_risk: total_risk * 0.2,
            recommendation,
        }
    }

    /// Get cached prediction
    pub fn cached(&self, chain_id: &str) -> Option<Vec<PredictedOutcome>> {
        self.cache.read().get(chain_id).cloned()
    }

    /// Total predictions made
    pub fn prediction_count(&self) -> usize {
        self.predictions.read().len()
    }
}

impl Default for OutcomePredictor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outcome_prediction() {
        let predictor = OutcomePredictor::new();
        let chain = ActionChain::new(vec![
            Action {
                name: "read_file".into(),
                params: serde_json::json!({}),
                risk_level: 0.05,
            },
            Action {
                name: "write_file".into(),
                params: serde_json::json!({}),
                risk_level: 0.2,
            },
        ]);

        let outcomes = predictor.predict(&chain);
        assert!(outcomes.len() >= 2); // success + partial
        assert!(outcomes[0].confidence.value > 0.0);
    }

    #[test]
    fn test_chain_simulation() {
        let chain = ActionChain::new(vec![
            Action {
                name: "a".into(),
                params: serde_json::json!({}),
                risk_level: 0.1,
            },
            Action {
                name: "b".into(),
                params: serde_json::json!({}),
                risk_level: 0.2,
            },
            Action {
                name: "c".into(),
                params: serde_json::json!({}),
                risk_level: 0.3,
            },
        ]);
        let total = chain.total_risk();
        // 1 - (0.9 * 0.8 * 0.7) = 1 - 0.504 = 0.496
        assert!((total - 0.496).abs() < 0.001);
    }

    #[test]
    fn test_risk_assessment() {
        let predictor = OutcomePredictor::new();
        let risky_chain = ActionChain::new(vec![Action {
            name: "delete_all".into(),
            params: serde_json::json!({}),
            risk_level: 0.8,
        }]);

        let outcomes = predictor.predict(&risky_chain);
        let success = &outcomes[0];
        assert_eq!(
            success.risk_assessment.recommendation,
            RiskRecommendation::Block
        );
    }

    #[test]
    fn test_echo_caching() {
        let predictor = OutcomePredictor::new();
        let chain = ActionChain::new(vec![Action {
            name: "safe".into(),
            params: serde_json::json!({}),
            risk_level: 0.05,
        }]);

        let first = predictor.predict(&chain);
        let cached = predictor.cached(&chain.id).unwrap();
        assert_eq!(first.len(), cached.len());
    }
}
