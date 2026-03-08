//! FutureQuery — "what if" interface for exploring future states.

use serde::{Deserialize, Serialize};

use super::predictor::{Action, ActionChain, OutcomePredictor, PredictedOutcome};

/// A "what if" query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FutureQuery {
    pub id: String,
    pub question: String,
    pub actions: Vec<Action>,
    pub constraints: Vec<String>,
}

impl FutureQuery {
    pub fn new(question: &str, actions: Vec<Action>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            question: question.into(),
            actions,
            constraints: Vec::new(),
        }
    }

    pub fn with_constraint(mut self, constraint: &str) -> Self {
        self.constraints.push(constraint.into());
        self
    }
}

/// Result of a future query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FutureQueryResult {
    pub query_id: String,
    pub question: String,
    pub outcomes: Vec<PredictedOutcome>,
    pub best_outcome: Option<String>,
    pub worst_outcome: Option<String>,
    pub recommended_action: String,
}

/// Execute future queries
pub struct FutureQueryEngine {
    predictor: OutcomePredictor,
    query_count: parking_lot::Mutex<u64>,
}

impl FutureQueryEngine {
    pub fn new() -> Self {
        Self {
            predictor: OutcomePredictor::new(),
            query_count: parking_lot::Mutex::new(0),
        }
    }

    /// Answer a "what if" query
    pub fn query(&self, q: &FutureQuery) -> FutureQueryResult {
        *self.query_count.lock() += 1;

        let chain = ActionChain::new(q.actions.clone());
        let outcomes = self.predictor.predict(&chain);

        let best = outcomes
            .iter()
            .max_by(|a, b| a.confidence.value.partial_cmp(&b.confidence.value).unwrap())
            .map(|o| o.outcome_id.clone());

        let worst = outcomes
            .iter()
            .filter(|o| o.risk_assessment.overall_risk > 0.0)
            .max_by(|a, b| {
                a.risk_assessment
                    .overall_risk
                    .partial_cmp(&b.risk_assessment.overall_risk)
                    .unwrap()
            })
            .map(|o| o.outcome_id.clone());

        let recommended = if outcomes
            .iter()
            .any(|o| o.risk_assessment.overall_risk > 0.7)
        {
            "Consider alternative approach — high risk detected".into()
        } else if outcomes
            .iter()
            .any(|o| o.risk_assessment.overall_risk > 0.3)
        {
            "Proceed with caution — moderate risk".into()
        } else {
            "Safe to proceed".into()
        };

        FutureQueryResult {
            query_id: q.id.clone(),
            question: q.question.clone(),
            outcomes,
            best_outcome: best,
            worst_outcome: worst,
            recommended_action: recommended,
        }
    }

    pub fn query_count(&self) -> u64 {
        *self.query_count.lock()
    }
}

impl Default for FutureQueryEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_what_if_query() {
        let engine = FutureQueryEngine::new();
        let query = FutureQuery::new(
            "What if I delete and recreate the database?",
            vec![
                Action {
                    name: "drop_database".into(),
                    params: serde_json::json!({}),
                    risk_level: 0.9,
                },
                Action {
                    name: "create_database".into(),
                    params: serde_json::json!({}),
                    risk_level: 0.1,
                },
            ],
        );

        let result = engine.query(&query);
        assert!(!result.outcomes.is_empty());
        assert!(result.best_outcome.is_some());
        assert!(result.recommended_action.contains("risk"));
        assert_eq!(engine.query_count(), 1);
    }

    #[test]
    fn test_multiple_outcomes() {
        let engine = FutureQueryEngine::new();
        let query = FutureQuery::new(
            "Safe operation",
            vec![Action {
                name: "read".into(),
                params: serde_json::json!({}),
                risk_level: 0.02,
            }],
        );

        let result = engine.query(&query);
        assert!(result.recommended_action.contains("Safe"));
    }
}
