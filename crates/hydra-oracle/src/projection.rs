//! Oracle projection — the output of scenario generation.

use serde::{Deserialize, Serialize};

use crate::scenario::Scenario;

/// A complete oracle projection containing multiple scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleProjection {
    /// The context that produced this projection.
    pub context: String,
    /// All generated scenarios.
    pub scenarios: Vec<Scenario>,
    /// Overall confidence in this projection (0.0 to 1.0).
    pub confidence: f64,
    /// Index of the most likely scenario (by probability).
    pub most_likely: Option<usize>,
    /// Index of the most adverse scenario (highest probability among adverse).
    pub most_adverse: Option<usize>,
}

impl OracleProjection {
    /// Create a new projection and compute most_likely / most_adverse indices.
    pub fn new(context: String, scenarios: Vec<Scenario>, confidence: f64) -> Self {
        let most_likely = scenarios
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                a.probability
                    .partial_cmp(&b.probability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i);

        let most_adverse = scenarios
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_adverse)
            .max_by(|(_, a), (_, b)| {
                a.probability
                    .partial_cmp(&b.probability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i);

        Self {
            context,
            scenarios,
            confidence: confidence.clamp(0.0, 1.0),
            most_likely,
            most_adverse,
        }
    }

    /// Return the number of scenarios in this projection.
    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Return the number of adverse scenarios.
    pub fn adverse_count(&self) -> usize {
        self.scenarios.iter().filter(|s| s.is_adverse).count()
    }

    /// Return the most likely scenario, if any.
    pub fn most_likely_scenario(&self) -> Option<&Scenario> {
        self.most_likely.and_then(|i| self.scenarios.get(i))
    }

    /// Return the most adverse scenario, if any.
    pub fn most_adverse_scenario(&self) -> Option<&Scenario> {
        self.most_adverse.and_then(|i| self.scenarios.get(i))
    }

    /// Return a human-readable summary of this projection.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "Oracle Projection [{}]: {} scenarios, confidence={:.2}",
            self.context,
            self.scenario_count(),
            self.confidence,
        ));

        for scenario in &self.scenarios {
            lines.push(format!("  {}", scenario.label()));
        }

        if let Some(s) = self.most_likely_scenario() {
            lines.push(format!("  Most likely: {}", s.name));
        }
        if let Some(s) = self.most_adverse_scenario() {
            lines.push(format!("  Most adverse: {}", s.name));
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scenarios() -> Vec<Scenario> {
        vec![
            Scenario::new("good-outcome", 0.6, false, "things go well", None),
            Scenario::new(
                "risk-cascade",
                0.35,
                true,
                "cascade failure",
                Some("add circuit breaker".into()),
            ),
        ]
    }

    #[test]
    fn most_likely_computed() {
        let proj = OracleProjection::new("test".into(), make_scenarios(), 0.8);
        assert_eq!(proj.most_likely, Some(0));
    }

    #[test]
    fn most_adverse_computed() {
        let proj = OracleProjection::new("test".into(), make_scenarios(), 0.8);
        assert_eq!(proj.most_adverse, Some(1));
    }

    #[test]
    fn adverse_count() {
        let proj = OracleProjection::new("test".into(), make_scenarios(), 0.8);
        assert_eq!(proj.adverse_count(), 1);
    }

    #[test]
    fn summary_contains_context() {
        let proj = OracleProjection::new("deploy".into(), make_scenarios(), 0.8);
        let s = proj.summary();
        assert!(s.contains("deploy"));
        assert!(s.contains("2 scenarios"));
    }
}
