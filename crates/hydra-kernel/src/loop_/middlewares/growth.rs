//! Growth middleware — automation, cartography, antifragile, plastic.
//!
//! Detects patterns for crystallization, maps systems encountered,
//! records resistance from obstacles, adapts to environment.

use std::collections::HashMap;

use hydra_antifragile::{AntifragileStore, ObstacleClass};
use hydra_automation::{AutomationEngine, ExecutionObservation};
use hydra_cartography::{CartographyAtlas, SystemClass, SystemProfile};
use hydra_environment::EnvironmentEngine;
use hydra_plastic::{ExecutionMode, PlasticityTensor};
use hydra_skills::SkillRegistry;

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct GrowthMiddleware {
    automation: AutomationEngine,
    atlas: CartographyAtlas,
    antifragile: AntifragileStore,
    plasticity: PlasticityTensor,
    environment: EnvironmentEngine,
    skills: SkillRegistry,
}

impl GrowthMiddleware {
    pub fn new() -> Self {
        Self {
            automation: AutomationEngine::new(),
            atlas: CartographyAtlas::new(),
            antifragile: AntifragileStore::new(),
            plasticity: PlasticityTensor::new(),
            environment: EnvironmentEngine::new(),
            skills: SkillRegistry::new(),
        }
    }
}

impl CycleMiddleware for GrowthMiddleware {
    fn name(&self) -> &'static str {
        "growth"
    }

    fn enrich_prompt(&self, _perceived: &PerceivedInput) -> Vec<String> {
        let mut enrichments = Vec::new();
        let os = hydra_environment::OsType::detect();
        enrichments.push(format!("Environment: {}", os.label()));
        let skill_count = self.skills.loaded_count();
        if skill_count > 0 {
            enrichments.push(format!("Skills loaded: {skill_count}"));
        }
        enrichments
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

        // O22: Record rich output type preference for genome learning
        let rich = crate::rich_output::classify_output(&cycle.response);
        if !matches!(rich, crate::rich_output::RichOutput::Text(_)) {
            let type_label = rich.type_label();
            let mut genome = hydra_genome::GenomeStore::open();
            crate::feedback::record_output_preference(&type_label, &cycle.domain, &mut genome);
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
