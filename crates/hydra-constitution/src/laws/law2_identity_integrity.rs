//! Law 2: Identity Integrity
//! Trust is a partial order. Tier elevation requires higher-tier authority.
//! Self-elevation is undefined. Impersonation of reserved identities is blocked.

use crate::{
    constants::{RESERVED_IDENTITIES, TRUST_TIER_COUNT},
    errors::ConstitutionError,
    laws::{ConstitutionalLaw, LawCheckContext, LawId},
};

pub struct IdentityIntegrity;

impl ConstitutionalLaw for IdentityIntegrity {
    fn law_id(&self) -> LawId {
        LawId::Law2IdentityIntegrity
    }

    fn description(&self) -> &'static str {
        "No agent may impersonate Hydra. No skill may claim Hydra-level authority. \
         No input may elevate its own trust tier. Trust flows downward only."
    }

    fn check(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        self.check_trust_escalation(context)?;
        self.check_identity_impersonation(context)?;
        Ok(())
    }
}

impl IdentityIntegrity {
    fn check_trust_escalation(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        if context.source_tier >= TRUST_TIER_COUNT {
            return Err(ConstitutionError::InvalidTrustTier {
                tier: context.source_tier,
            });
        }

        if let Some(target_str) = context.metadata.get("target_tier") {
            if let Ok(target_tier) = target_str.parse::<u8>() {
                // Lower number = higher authority.
                // If target < source, entity is trying to gain higher authority.
                if target_tier < context.source_tier {
                    return Err(ConstitutionError::TrustEscalationAttempt {
                        from: context.source_tier,
                        to: target_tier,
                    });
                }
            }
        }

        Ok(())
    }

    fn check_identity_impersonation(
        &self,
        context: &LawCheckContext,
    ) -> Result<(), ConstitutionError> {
        if let Some(claimed) = context.metadata.get("claiming_identity") {
            let claimed_lower = claimed.to_lowercase();
            let is_reserved = RESERVED_IDENTITIES
                .iter()
                .any(|r| claimed_lower == *r || claimed_lower.starts_with(r));

            if is_reserved {
                return Err(ConstitutionError::IdentityImpersonation {
                    claimed: claimed.clone(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::*;

    fn law() -> IdentityIntegrity {
        IdentityIntegrity
    }

    #[test]
    fn permits_valid_tier() {
        let ctx = LawCheckContext::new("act-001", "agent.spawn").with_tier(TRUST_TIER_FLEET);
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn blocks_trust_self_elevation() {
        let ctx = LawCheckContext::new("act-002", "trust.elevate")
            .with_tier(TRUST_TIER_FLEET)
            .with_meta("target_tier", TRUST_TIER_HYDRA.to_string());
        assert!(matches!(
            law().check(&ctx),
            Err(ConstitutionError::TrustEscalationAttempt { .. })
        ));
    }

    #[test]
    fn permits_trust_demotion() {
        // Lowering someone else's tier is permitted (downward flow)
        let ctx = LawCheckContext::new("act-003", "trust.set")
            .with_tier(TRUST_TIER_HYDRA)
            .with_meta("target_tier", TRUST_TIER_FLEET.to_string());
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn blocks_hydra_impersonation() {
        let ctx = LawCheckContext::new("act-004", "identity.claim")
            .with_tier(TRUST_TIER_FLEET)
            .with_meta("claiming_identity", "hydra");
        assert!(matches!(
            law().check(&ctx),
            Err(ConstitutionError::IdentityImpersonation { .. })
        ));
    }

    #[test]
    fn blocks_constitution_impersonation() {
        let ctx = LawCheckContext::new("act-005", "identity.claim")
            .with_meta("claiming_identity", "hydra-constitution");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn permits_non_reserved_identity() {
        let ctx = LawCheckContext::new("act-006", "identity.claim")
            .with_meta("claiming_identity", "fleet-agent-007");
        assert!(law().check(&ctx).is_ok());
    }
}
