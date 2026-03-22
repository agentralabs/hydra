//! Oracle engine — generates probabilistic scenario projections.

use hydra_axiom::AxiomPrimitive;

use crate::constants::{
    CASCADE_BASE_PROBABILITY, DEFAULT_BASE_PROBABILITY, HIGH_PRIMITIVE_CONFIDENCE,
    LOW_PRIMITIVE_CONFIDENCE, MAX_SCENARIOS_PER_PROJECTION, MIN_SCENARIO_PROBABILITY,
    OPTIMIZATION_BASE_PROBABILITY, PRIMITIVE_COUNT_THRESHOLD, RISK_BASE_PROBABILITY,
};
use crate::errors::OracleError;
use crate::projection::OracleProjection;
use crate::scenario::Scenario;

/// The oracle engine generates scenario projections from axiom primitives.
#[derive(Debug, Default)]
pub struct OracleEngine {
    /// Previously generated projections.
    projections: Vec<OracleProjection>,
}

impl OracleEngine {
    /// Create a new oracle engine.
    pub fn new() -> Self {
        Self {
            projections: Vec::new(),
        }
    }

    /// Generate a projection from context, domain, and axiom primitives.
    ///
    /// Produces scenarios based on primitive combinations:
    /// - Risk primitives generate adverse scenarios
    /// - Optimization primitives generate positive scenarios
    /// - CausalLink primitives generate cascade scenarios
    pub fn project(
        &mut self,
        context: &str,
        domain: &str,
        primitives: &[AxiomPrimitive],
    ) -> Result<OracleProjection, OracleError> {
        if primitives.is_empty() {
            return Err(OracleError::InsufficientContext);
        }

        let mut scenarios = Vec::new();

        for primitive in primitives {
            if scenarios.len() >= MAX_SCENARIOS_PER_PROJECTION {
                break;
            }

            let scenario = self.scenario_from_primitive(primitive, domain);
            if scenario.probability >= MIN_SCENARIO_PROBABILITY {
                scenarios.push(scenario);
            }
        }

        // If no scenarios generated, add a default baseline.
        if scenarios.is_empty() {
            scenarios.push(Scenario::new(
                format!("{domain}-baseline"),
                DEFAULT_BASE_PROBABILITY,
                false,
                format!("Baseline scenario for {context} in {domain}"),
                None,
            ));
        }

        let confidence = if primitives.len() >= PRIMITIVE_COUNT_THRESHOLD {
            HIGH_PRIMITIVE_CONFIDENCE
        } else {
            LOW_PRIMITIVE_CONFIDENCE
        };

        let projection = OracleProjection::new(context.to_string(), scenarios, confidence);
        self.projections.push(projection.clone());
        Ok(projection)
    }

    /// Return the number of projections generated so far.
    pub fn projection_count(&self) -> usize {
        self.projections.len()
    }

    /// Return the total number of scenarios across all projections.
    pub fn scenario_count(&self) -> usize {
        self.projections.iter().map(|p| p.scenario_count()).sum()
    }

    /// Return a human-readable summary of the engine state.
    pub fn summary(&self) -> String {
        format!(
            "OracleEngine: {} projections, {} total scenarios",
            self.projection_count(),
            self.scenario_count(),
        )
    }

    /// Generate a scenario from a single axiom primitive.
    fn scenario_from_primitive(&self, primitive: &AxiomPrimitive, domain: &str) -> Scenario {
        match primitive {
            AxiomPrimitive::Risk => Scenario::new(
                format!("{domain}-risk-adverse"),
                RISK_BASE_PROBABILITY,
                true,
                format!("Adverse scenario: risk materializes in {domain}"),
                Some("Implement risk mitigation controls".into()),
            ),
            AxiomPrimitive::Optimization => Scenario::new(
                format!("{domain}-optimization-positive"),
                OPTIMIZATION_BASE_PROBABILITY,
                false,
                format!("Positive scenario: optimization succeeds in {domain}"),
                None,
            ),
            AxiomPrimitive::CausalLink => Scenario::new(
                format!("{domain}-cascade"),
                CASCADE_BASE_PROBABILITY,
                true,
                format!("Cascade scenario: causal chain propagates in {domain}"),
                Some("Add circuit breakers at causal boundaries".into()),
            ),
            AxiomPrimitive::AdversarialModel => Scenario::new(
                format!("{domain}-adversarial-adverse"),
                RISK_BASE_PROBABILITY,
                true,
                format!("Adverse scenario: adversarial exploitation in {domain}"),
                Some("Harden attack surfaces".into()),
            ),
            AxiomPrimitive::Dependency => Scenario::new(
                format!("{domain}-dependency-risk"),
                CASCADE_BASE_PROBABILITY,
                true,
                format!("Dependency scenario: upstream failure in {domain}"),
                Some("Add fallback for critical dependencies".into()),
            ),
            _ => Scenario::new(
                format!("{domain}-{}-scenario", primitive.label()),
                DEFAULT_BASE_PROBABILITY,
                false,
                format!("General scenario from {} in {domain}", primitive.label()),
                None,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_risk_generates_adverse() {
        let mut engine = OracleEngine::new();
        let proj = engine
            .project("deploy", "infrastructure", &[AxiomPrimitive::Risk])
            .expect("should project");
        assert!(proj.adverse_count() > 0);
    }

    #[test]
    fn project_optimization_positive() {
        let mut engine = OracleEngine::new();
        let proj = engine
            .project("optimize", "performance", &[AxiomPrimitive::Optimization])
            .expect("should project");
        assert_eq!(proj.adverse_count(), 0);
    }

    #[test]
    fn empty_primitives_fails() {
        let mut engine = OracleEngine::new();
        assert!(engine.project("test", "test", &[]).is_err());
    }

    #[test]
    fn scenario_count_tracks() {
        let mut engine = OracleEngine::new();
        let _ = engine.project("a", "d", &[AxiomPrimitive::Risk]);
        let _ = engine.project("b", "d", &[AxiomPrimitive::Optimization]);
        assert_eq!(engine.projection_count(), 2);
        assert!(engine.scenario_count() >= 2);
    }

    #[test]
    fn summary_format() {
        let mut engine = OracleEngine::new();
        let _ = engine.project("test", "test", &[AxiomPrimitive::Risk]);
        let s = engine.summary();
        assert!(s.contains("OracleEngine"));
        assert!(s.contains("1 projections"));
    }
}
