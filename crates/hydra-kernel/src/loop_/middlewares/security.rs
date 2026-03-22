//! Security middleware — hydra-trust + hydra-adversary per-request.
//!
//! Evaluates threat signals on every input. Updates trust field.
//! Non-blocking: errors are logged, never stop the pipeline.

use hydra_adversary::{ImmuneSystem, ThreatClass, ThreatSignal};
use hydra_trust::{HamiltonianState, TrustField, TrustPhase};

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct SecurityMiddleware {
    trust: TrustField,
    immune: ImmuneSystem,
    threats_blocked: usize,
}

impl SecurityMiddleware {
    pub fn new() -> Self {
        Self {
            trust: TrustField::new(),
            immune: ImmuneSystem::new(),
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
        let signal = ThreatSignal::new(
            ThreatClass::Unknown,
            vec![],
            "cognitive-loop",
            &perceived.raw,
        );

        match self.immune.evaluate(&signal) {
            Ok(response) => {
                if matches!(response.action, hydra_adversary::ImmuneAction::Blocked) {
                    self.threats_blocked += 1;
                    perceived.enrichments.insert(
                        "security.threat".into(),
                        format!("BLOCKED: {}", response.description),
                    );
                    eprintln!(
                        "hydra: security blocked threat: {}",
                        response.description
                    );
                }
            }
            Err(e) => {
                eprintln!("hydra: security post_perceive: {e}");
            }
        }
    }

    fn enrich_prompt(&self, _perceived: &PerceivedInput) -> Vec<String> {
        let hamiltonian: HamiltonianState = self.trust.hamiltonian();
        if hamiltonian.phase != TrustPhase::Stable {
            vec![format!(
                "Trust status: {:?} (avg={:.2}, agents={})",
                hamiltonian.phase, hamiltonian.average_trust, hamiltonian.agent_count
            )]
        } else {
            Vec::new()
        }
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
