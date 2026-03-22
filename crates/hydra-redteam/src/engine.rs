//! RedTeamEngine -- the proactive adversarial simulation coordinator.

use crate::{
    constants::MAX_STORED_SCENARIOS,
    errors::RedTeamError,
    scenario::{GoNoGo, RedTeamScenario},
    surface::identify_surfaces,
    threat::threats_from_primitives,
};
use hydra_axiom::primitives::AxiomPrimitive;

/// The red team engine.
pub struct RedTeamEngine {
    scenarios: Vec<RedTeamScenario>,
}

impl RedTeamEngine {
    pub fn new() -> Self {
        Self {
            scenarios: Vec::new(),
        }
    }

    /// Run a red team analysis on a proposed action.
    pub fn analyze(
        &mut self,
        context: &str,
        primitives: &[AxiomPrimitive],
    ) -> Result<&RedTeamScenario, RedTeamError> {
        let surfaces = identify_surfaces(context);
        let threats = threats_from_primitives(context, primitives);

        let scenario = RedTeamScenario::new(context, surfaces, threats);

        if self.scenarios.len() >= MAX_STORED_SCENARIOS {
            self.scenarios.remove(0);
        }
        self.scenarios.push(scenario);
        // Safe: we just pushed an element
        Ok(self.scenarios.last().expect("just pushed"))
    }

    /// How many scenarios resulted in NO-GO.
    pub fn no_go_count(&self) -> usize {
        self.scenarios
            .iter()
            .filter(|s| matches!(s.go_no_go, GoNoGo::NoGo { .. }))
            .count()
    }

    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    pub fn latest(&self) -> Option<&RedTeamScenario> {
        self.scenarios.last()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        let critical = self
            .scenarios
            .iter()
            .filter(|s| s.has_critical_threats())
            .count();
        format!(
            "redteam: scenarios={} critical={}",
            self.scenario_count(),
            critical,
        )
    }
}

impl Default for RedTeamEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_deployment_analyzed() {
        let mut engine = RedTeamEngine::new();
        let prims = vec![AxiomPrimitive::TrustRelation, AxiomPrimitive::Risk];
        let scenario = engine
            .analyze("deploy auth service with token rotation", &prims)
            .expect("should succeed");
        assert!(scenario.threat_count() > 0);
        assert_ne!(scenario.go_no_go.label(), "");
    }

    #[test]
    fn summary_format() {
        let engine = RedTeamEngine::new();
        let s = engine.summary();
        assert!(s.contains("redteam:"));
        assert!(s.contains("scenarios="));
    }
}
