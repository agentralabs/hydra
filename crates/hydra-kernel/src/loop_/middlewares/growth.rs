//! Growth middleware — automation, cartography, antifragile, plastic.
//!
//! Detects patterns for crystallization, maps systems encountered,
//! records resistance from obstacles, adapts to environment.

use std::collections::HashMap;

use hydra_antifragile::{AntifragileStore, ObstacleClass};
use hydra_automation::{AutomationEngine, ExecutionObservation};
use hydra_cartography::{CartographyAtlas, SystemClass, SystemProfile};
use hydra_plastic::{ExecutionMode, PlasticityTensor};

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::CycleResult;

pub struct GrowthMiddleware {
    automation: AutomationEngine,
    atlas: CartographyAtlas,
    antifragile: AntifragileStore,
    plasticity: PlasticityTensor,
}

impl GrowthMiddleware {
    pub fn new() -> Self {
        Self {
            automation: AutomationEngine::new(),
            atlas: CartographyAtlas::new(),
            antifragile: AntifragileStore::new(),
            plasticity: PlasticityTensor::new(),
        }
    }
}

impl CycleMiddleware for GrowthMiddleware {
    fn name(&self) -> &'static str {
        "growth"
    }

    fn post_deliver(&mut self, cycle: &CycleResult) {
        // Pattern detection for automation crystallization
        let obs = ExecutionObservation::new(
            &cycle.path,
            &cycle.intent_summary,
            HashMap::new(),
            &cycle.domain,
            cycle.duration_ms,
            cycle.success,
        );
        if let Some(proposal_id) = self.automation.observe(obs) {
            eprintln!("hydra: growth automation proposal: {proposal_id}");
        }

        // Map domain as system encounter in the atlas
        let profile = SystemProfile::new(&cycle.domain, SystemClass::RestApi);
        if let Err(e) = self.atlas.add(profile) {
            // Duplicate add is expected — atlas grows by name
            let _ = e;
        }

        // Record resistance from failures (antifragile grows from obstacles)
        let obstacle = ObstacleClass::RateLimit; // representative obstacle
        if !cycle.success {
            if let Err(e) = self.antifragile.record_encounter(
                &obstacle,
                false,
                Some(&cycle.path),
            ) {
                eprintln!("hydra: growth antifragile record: {e}");
            }
        }

        // Adapt plasticity — record environment encounter
        let env_name = format!("loop-{}", cycle.path);
        if let Some(env) = self.plasticity.get_mut(&env_name) {
            env.record_encounter(cycle.success);
        } else {
            let env = hydra_plastic::EnvironmentProfile::new(
                &env_name,
                ExecutionMode::NativeBinary,
            );
            if let Err(e) = self.plasticity.add(env) {
                eprintln!("hydra: growth plasticity add: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_middleware_name() {
        let mw = GrowthMiddleware::new();
        assert_eq!(mw.name(), "growth");
    }
}

impl Default for GrowthMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
