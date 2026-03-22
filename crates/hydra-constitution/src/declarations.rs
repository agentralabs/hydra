//! The two constitutional declarations.
//! These sit alongside the seven laws and are equally immutable.
//!
//! DECLARATION 1: Capability — Hydra operates without self-imposed limits.
//! DECLARATION 2: Growth — Hydra's capabilities only ever grow.
//!
//! Unlike the laws (which govern integrity and trust),
//! the declarations govern operational scope and longevity.

use crate::errors::ConstitutionError;
use crate::laws::LawId;
use serde::{Deserialize, Serialize};

// ── Capability Declaration ─────────────────────────────────────────────

/// The three hard stops — the exhaustive list of legitimate reasons
/// Hydra may not complete a task.
/// Everything not on this list is a navigational obstacle, not a stop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardStop {
    /// Authentication credential was explicitly rejected by the remote system.
    AuthenticationExplicitlyDenied {
        /// The system that denied authentication.
        system: String,
        /// The reason for the denial.
        reason: String,
        /// Evidence of the denial (log output, error code, etc.).
        evidence: String,
    },

    /// The principal explicitly cancelled the task.
    PrincipalCancellation {
        /// The ID of the cancelled task.
        task_id: String,
        /// When the cancellation occurred.
        cancelled_at: String,
    },

    /// Continuing would require violating a constitutional law.
    ConstitutionalViolationRequired {
        /// Which law would be violated.
        law: LawId,
        /// Why continuing requires the violation.
        reason: String,
    },
}

impl HardStop {
    /// Returns true if this is a genuine hard stop.
    pub fn is_genuine(&self) -> bool {
        true
    }

    /// Human-readable description of why this is a hard stop.
    pub fn description(&self) -> String {
        match self {
            Self::AuthenticationExplicitlyDenied { system, reason, .. } => {
                format!(
                    "HARD_DENIED: Credentials explicitly rejected by '{}'. \
                     Reason: {}. This is a credential issue, not a \
                     technical problem. Action required from principal.",
                    system, reason
                )
            }
            Self::PrincipalCancellation { task_id, .. } => {
                format!("HARD_DENIED: Task '{}' cancelled by principal.", task_id)
            }
            Self::ConstitutionalViolationRequired { law, reason } => {
                format!(
                    "HARD_DENIED: Continuing would violate {}. Reason: {}",
                    law, reason
                )
            }
        }
    }
}

/// Context passed to capability declaration checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityCheckContext {
    /// The action being proposed.
    pub action_type: String,
    /// The system or resource being targeted.
    pub target_system: String,
    /// If present: a claimed hard stop reason.
    pub claimed_hard_stop: Option<HardStop>,
    /// Whether this action involves execution on a remote system.
    pub is_remote_execution: bool,
    /// Whether this action involves a previously unknown system type.
    pub is_novel_system: bool,
}

impl CapabilityCheckContext {
    /// Create a new capability check context.
    pub fn new(action_type: impl Into<String>, target_system: impl Into<String>) -> Self {
        Self {
            action_type: action_type.into(),
            target_system: target_system.into(),
            claimed_hard_stop: None,
            is_remote_execution: false,
            is_novel_system: false,
        }
    }

    /// Builder: set a claimed hard stop.
    pub fn with_hard_stop(mut self, stop: HardStop) -> Self {
        self.claimed_hard_stop = Some(stop);
        self
    }

    /// Builder: mark as remote execution.
    pub fn with_remote(mut self) -> Self {
        self.is_remote_execution = true;
        self
    }

    /// Builder: mark as novel system.
    pub fn with_novel_system(mut self) -> Self {
        self.is_novel_system = true;
        self
    }
}

/// Validates the Capability Declaration.
pub fn check_capability_declaration(ctx: &CapabilityCheckContext) -> Result<(), ConstitutionError> {
    if let Some(stop) = &ctx.claimed_hard_stop {
        if let HardStop::AuthenticationExplicitlyDenied { evidence, .. } = stop {
            if evidence.trim().is_empty() {
                return Err(ConstitutionError::LawViolation {
                    law: LawId::Law7CausalChainCompleteness,
                    reason: "Hard stop claimed for authentication denial \
                             but no evidence provided. Rate limits and \
                             timeouts are not hard stops."
                        .to_string(),
                });
            }
        }
        return Ok(());
    }

    Ok(())
}

// ── Growth Declaration ─────────────────────────────────────────────────

/// Context for growth declaration checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthCheckContext {
    /// The proposed modification.
    pub modification_type: String,
    /// Current genome entry count (before modification).
    pub genome_count_before: usize,
    /// Expected genome entry count after modification.
    pub genome_count_after: usize,
    /// Current cartography entry count (before modification).
    pub cartography_count_before: usize,
    /// Expected cartography entry count after modification.
    pub cartography_count_after: usize,
    /// Current antifragile record count (before modification).
    pub antifragile_count_before: usize,
    /// Expected antifragile record count after modification.
    pub antifragile_count_after: usize,
    /// Whether this modification clears any capability memory.
    pub clears_capability_memory: bool,
    /// Whether this modification resets any growth layer metric.
    pub resets_growth_metric: bool,
}

impl GrowthCheckContext {
    /// Create a growth check for a modification that does not affect capability counts.
    pub fn neutral(modification_type: impl Into<String>) -> Self {
        Self {
            modification_type: modification_type.into(),
            genome_count_before: 0,
            genome_count_after: 0,
            cartography_count_before: 0,
            cartography_count_after: 0,
            antifragile_count_before: 0,
            antifragile_count_after: 0,
            clears_capability_memory: false,
            resets_growth_metric: false,
        }
    }

    /// Builder: set genome count change.
    pub fn with_genome_change(mut self, before: usize, after: usize) -> Self {
        self.genome_count_before = before;
        self.genome_count_after = after;
        self
    }

    /// Builder: set cartography count change.
    pub fn with_cartography_change(mut self, before: usize, after: usize) -> Self {
        self.cartography_count_before = before;
        self.cartography_count_after = after;
        self
    }

    /// Builder: set antifragile count change.
    pub fn with_antifragile_change(mut self, before: usize, after: usize) -> Self {
        self.antifragile_count_before = before;
        self.antifragile_count_after = after;
        self
    }

    /// Builder: mark as clearing capability memory.
    pub fn clears_memory(mut self) -> Self {
        self.clears_capability_memory = true;
        self
    }

    /// Builder: mark as resetting growth metric.
    pub fn resets_metric(mut self) -> Self {
        self.resets_growth_metric = true;
        self
    }
}

/// Validates the Growth Declaration: capabilities are monotonically non-decreasing.
pub fn check_growth_declaration(ctx: &GrowthCheckContext) -> Result<(), ConstitutionError> {
    if ctx.clears_capability_memory {
        return Err(ConstitutionError::LawViolation {
            law: LawId::Law3MemorySovereignty,
            reason: "Growth Declaration violated: capability memory \
                     may not be cleared. Genome, cartography, and \
                     antifragile records are permanent."
                .to_string(),
        });
    }

    if ctx.resets_growth_metric {
        return Err(ConstitutionError::LawViolation {
            law: LawId::Law3MemorySovereignty,
            reason: "Growth Declaration violated: growth layer metrics \
                     may not be reset. The growth invariant requires \
                     \u{0393}\u{0302}(\u{03A8}) \u{2265} 0 at all times."
                .to_string(),
        });
    }

    if ctx.genome_count_after < ctx.genome_count_before {
        return Err(ConstitutionError::LawViolation {
            law: LawId::Law3MemorySovereignty,
            reason: format!(
                "Growth Declaration violated: genome count would decrease \
                 from {} to {}. Genome entries are permanent.",
                ctx.genome_count_before, ctx.genome_count_after
            ),
        });
    }

    if ctx.cartography_count_after < ctx.cartography_count_before {
        return Err(ConstitutionError::LawViolation {
            law: LawId::Law3MemorySovereignty,
            reason: format!(
                "Growth Declaration violated: cartography count would \
                 decrease from {} to {}. System profiles are permanent.",
                ctx.cartography_count_before, ctx.cartography_count_after
            ),
        });
    }

    if ctx.antifragile_count_after < ctx.antifragile_count_before {
        return Err(ConstitutionError::LawViolation {
            law: LawId::Law3MemorySovereignty,
            reason: format!(
                "Growth Declaration violated: antifragile record count \
                 would decrease from {} to {}. Antifragile records \
                 are permanent.",
                ctx.antifragile_count_before, ctx.antifragile_count_after
            ),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn novel_system_is_not_a_hard_stop() {
        let ctx = CapabilityCheckContext::new("execute", "unknown-mainframe").with_novel_system();
        assert!(check_capability_declaration(&ctx).is_ok());
    }

    #[test]
    fn remote_execution_is_not_a_hard_stop() {
        let ctx = CapabilityCheckContext::new("ssh.execute", "remote-server").with_remote();
        assert!(check_capability_declaration(&ctx).is_ok());
    }

    #[test]
    fn genuine_auth_denial_with_evidence_is_permitted_hard_stop() {
        let ctx = CapabilityCheckContext::new("ssh.connect", "prod-server").with_hard_stop(
            HardStop::AuthenticationExplicitlyDenied {
                system: "prod-server".to_string(),
                reason: "SSH key fingerprint not in authorized_keys".to_string(),
                evidence: "SSH_AUTH_ERROR: Permission denied (publickey)".to_string(),
            },
        );
        assert!(check_capability_declaration(&ctx).is_ok());
    }

    #[test]
    fn auth_denial_without_evidence_is_rejected() {
        let ctx = CapabilityCheckContext::new("ssh.connect", "server").with_hard_stop(
            HardStop::AuthenticationExplicitlyDenied {
                system: "server".to_string(),
                reason: "timed out".to_string(),
                evidence: "".to_string(),
            },
        );
        assert!(check_capability_declaration(&ctx).is_err());
    }

    #[test]
    fn principal_cancellation_is_genuine_hard_stop() {
        let ctx = CapabilityCheckContext::new("task.continue", "deployment").with_hard_stop(
            HardStop::PrincipalCancellation {
                task_id: "task-001".to_string(),
                cancelled_at: "2026-03-19T12:00:00Z".to_string(),
            },
        );
        assert!(check_capability_declaration(&ctx).is_ok());
    }

    #[test]
    fn constitutional_violation_is_genuine_hard_stop() {
        let ctx = CapabilityCheckContext::new("memory.wipe", "all-beliefs").with_hard_stop(
            HardStop::ConstitutionalViolationRequired {
                law: LawId::Law3MemorySovereignty,
                reason: "continuing would wipe the belief manifold".to_string(),
            },
        );
        assert!(check_capability_declaration(&ctx).is_ok());
    }

    #[test]
    fn neutral_modification_passes_growth_check() {
        let ctx = GrowthCheckContext::neutral("skill.load");
        assert!(check_growth_declaration(&ctx).is_ok());
    }

    #[test]
    fn genome_growth_passes() {
        let ctx = GrowthCheckContext::neutral("task.complete").with_genome_change(100, 101);
        assert!(check_growth_declaration(&ctx).is_ok());
    }

    #[test]
    fn genome_reduction_is_blocked() {
        let ctx = GrowthCheckContext::neutral("genome.prune").with_genome_change(100, 99);
        assert!(check_growth_declaration(&ctx).is_err());
    }

    #[test]
    fn clearing_capability_memory_is_blocked() {
        let ctx = GrowthCheckContext::neutral("capability.reset").clears_memory();
        assert!(check_growth_declaration(&ctx).is_err());
    }

    #[test]
    fn resetting_growth_metric_is_blocked() {
        let ctx = GrowthCheckContext::neutral("growth.reset").resets_metric();
        assert!(check_growth_declaration(&ctx).is_err());
    }

    #[test]
    fn cartography_reduction_is_blocked() {
        let ctx =
            GrowthCheckContext::neutral("cartography.prune").with_cartography_change(500, 499);
        assert!(check_growth_declaration(&ctx).is_err());
    }

    #[test]
    fn antifragile_reduction_is_blocked() {
        let ctx = GrowthCheckContext::neutral("antifragile.clear").with_antifragile_change(200, 0);
        assert!(check_growth_declaration(&ctx).is_err());
    }
}
