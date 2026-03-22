//! Spawn logic — constitutional check + Boltzmann trust gate.

use crate::agent::AgentSpecialization;
use crate::constants::SPAWN_MIN_TRUST_SCORE;
use crate::errors::FleetError;
use hydra_constitution::constants::CONSTITUTIONAL_IDENTITY_ID;
use hydra_constitution::{CheckResult, ConstitutionChecker, LawCheckContext};
use hydra_trust::spawn::boltzmann_weight;
use hydra_trust::TrustTier;
use serde::{Deserialize, Serialize};

/// A request to spawn a new fleet agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    /// Desired name for the new agent.
    pub name: String,
    /// Desired specialization.
    pub specialization: AgentSpecialization,
    /// The causal root initiating this spawn.
    pub causal_root: String,
    /// The trust score of the requesting entity.
    pub requester_trust_score: f64,
    /// The trust tier of the requesting entity.
    pub requester_tier: TrustTier,
}

/// The result of a spawn eligibility check.
#[derive(Debug, Clone)]
pub struct SpawnCheckResult {
    /// Whether the spawn is permitted.
    pub permitted: bool,
    /// Constitutional check result.
    pub constitutional: CheckResult,
    /// Boltzmann weight for the requester's tier.
    pub boltzmann: f64,
    /// Reason for rejection, if any.
    pub rejection_reason: Option<String>,
}

/// Check whether a spawn request is permitted.
///
/// Combines constitutional law check with Boltzmann trust gating.
/// Constitutional check runs FIRST — if it fails, the spawn is rejected
/// before the trust gate is even evaluated.
pub fn check_spawn(request: &SpawnRequest) -> Result<SpawnCheckResult, FleetError> {
    let checker = ConstitutionChecker::new();

    let ctx = LawCheckContext::new(format!("spawn-{}", request.name), "agent.spawn")
        .with_causal_chain(vec![
            request.causal_root.clone(),
            CONSTITUTIONAL_IDENTITY_ID.to_string(),
        ]);

    let constitutional = checker.check(&ctx);

    if !constitutional.is_permitted() {
        let reason = constitutional
            .first_violation()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown constitutional violation".to_string());
        return Ok(SpawnCheckResult {
            permitted: false,
            constitutional,
            boltzmann: 0.0,
            rejection_reason: Some(reason),
        });
    }

    // Boltzmann trust gate
    let bw = boltzmann_weight(request.requester_tier, 1.0);

    if request.requester_trust_score < SPAWN_MIN_TRUST_SCORE {
        return Ok(SpawnCheckResult {
            permitted: false,
            constitutional,
            boltzmann: bw,
            rejection_reason: Some(format!(
                "trust score {:.4} below minimum {:.4}",
                request.requester_trust_score, SPAWN_MIN_TRUST_SCORE,
            )),
        });
    }

    Ok(SpawnCheckResult {
        permitted: true,
        constitutional,
        boltzmann: bw,
        rejection_reason: None,
    })
}
