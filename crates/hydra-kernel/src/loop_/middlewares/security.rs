//! Security middleware — hydra-trust + hydra-adversary per-request.
//!
//! Evaluates threat signals on every input. Updates trust field.
//! Non-blocking: errors are logged, never stop the pipeline.

use hydra_adversary::{ImmuneSystem, ThreatClass, ThreatSignal};
use hydra_redteam::RedTeamEngine;
use hydra_trust::{HamiltonianState, TrustField, TrustPhase};

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct SecurityMiddleware {
    trust: TrustField,
    immune: ImmuneSystem,
    redteam: RedTeamEngine,
    threats_blocked: usize,
}

impl SecurityMiddleware {
    pub fn new() -> Self {
        Self {
            trust: TrustField::new(),
            immune: ImmuneSystem::new(),
            redteam: RedTeamEngine::new(),
            threats_blocked: 0,
        }
    }
}

impl CycleMiddleware for SecurityMiddleware {
    fn name(&self) -> &'static str {
        "security"
    }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        // Evaluate input for threats
        // Real feature extraction — feeds the immune system with actual threat signals
        let (threat_class, features) = crate::security::features::extract_features(&perceived.raw);
        let signal = ThreatSignal::new(
            threat_class,
            features,
            "cognitive-loop",
            &perceived.raw,
        );

        match self.immune.evaluate(&signal) {
            Ok(response) => {
                if matches!(response.action, hydra_adversary::ImmuneAction::Blocked) {
                    self.threats_blocked += 1;
                    perceived.enrichments.insert(
                        "security.blocked".into(),
                        format!("BLOCKED: {}", response.description),
                    );
                    // SEC-4: ENFORCE the block — neutralize the input
                    perceived.raw = format!("[Security blocked: {}. Original input neutralized.]", response.description);
                    eprintln!("hydra: SECURITY ENFORCED BLOCK: {}", response.description);
                }
            }
            Err(e) => {
                eprintln!("hydra: security post_perceive: {e}");
            }
        }

        // RedTeam pre-action threat assessment
        if let Ok(scenario) = self.redteam.analyze(&perceived.raw, &perceived.comprehended.primitives) {
            if scenario.go_no_go != hydra_redteam::GoNoGo::Go {
                perceived.enrichments.insert(
                    "security.redteam".into(),
                    format!("{}: {} threats, {} surfaces", scenario.go_no_go.label(), scenario.threat_count(), scenario.surface_count()),
                );
            }
        }
    }

    fn enrich_prompt(&self, perceived: &PerceivedInput) -> Vec<String> {
        let mut items = Vec::new();
        let hamiltonian: HamiltonianState = self.trust.hamiltonian();
        if hamiltonian.phase != TrustPhase::Stable {
            items.push(format!(
                "Trust status: {:?} (avg={:.2}, agents={})",
                hamiltonian.phase, hamiltonian.average_trust, hamiltonian.agent_count
            ));
        }
        items
    }

    fn post_deliver(&mut self, _cycle: &CycleResult) {
        // Trust field tracking is per-agent (UUID), not per-cycle.
        // The field is populated when agents are added via TrustField::add_agent().
        // Per-cycle, we just note the hamiltonian state.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_middleware_name() {
        let mw = SecurityMiddleware::new();
        assert_eq!(mw.name(), "security");
    }

    #[test]
    fn security_starts_with_zero_threats() {
        let mw = SecurityMiddleware::new();
        assert_eq!(mw.threats_blocked, 0);
    }
}

impl Default for SecurityMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
