//! Law 6: Principal Supremacy
//! The trust partial order has exactly one maximum element: the principal.
//! Domain authority structures are domain-layer constructs only.
//! No configuration, skill, or agent may remove or dilute principal authority.

use crate::{
    constants::PRINCIPAL_MAX_COUNT,
    errors::ConstitutionError,
    laws::{ConstitutionalLaw, LawCheckContext, LawId},
};

pub struct PrincipalSupremacy;

impl ConstitutionalLaw for PrincipalSupremacy {
    fn law_id(&self) -> LawId {
        LawId::Law6PrincipalSupremacy
    }

    fn description(&self) -> &'static str {
        "One principal authority exists at Tier 0 above all other entities \
         in any domain Hydra operates in. Domain-specific authority structures \
         are domain-layer constructs that may not supersede the Tier 0 principal."
    }

    fn check(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        self.check_principal_count(context)?;
        self.check_principal_demotion(context)?;
        self.check_domain_authority_override(context)?;
        Ok(())
    }
}

impl PrincipalSupremacy {
    fn check_principal_count(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        if let Some(count_str) = context.metadata.get("principal_count") {
            if let Ok(count) = count_str.parse::<usize>() {
                if count > PRINCIPAL_MAX_COUNT {
                    return Err(ConstitutionError::MultiplePrincipals { count });
                }
            }
        }

        Ok(())
    }

    fn check_principal_demotion(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        let action = context.action_type.as_str();
        let demotion_actions = [
            "principal.demote",
            "principal.remove",
            "principal.transfer",
            "principal.override",
            "principal.subordinate",
        ];

        if demotion_actions.iter().any(|a| action.starts_with(a)) {
            return Err(ConstitutionError::LawViolation {
                law: LawId::Law6PrincipalSupremacy,
                reason: format!(
                    "action '{}' attempts to demote or remove the principal authority",
                    action
                ),
            });
        }

        Ok(())
    }

    fn check_domain_authority_override(
        &self,
        context: &LawCheckContext,
    ) -> Result<(), ConstitutionError> {
        // Domain skills may define internal hierarchy but cannot
        // claim authority that supersedes the principal
        if let Some(authority_claim) = context.metadata.get("claims_authority_over_principal") {
            if authority_claim == "true" {
                return Err(ConstitutionError::LawViolation {
                    law: LawId::Law6PrincipalSupremacy,
                    reason: "domain skill attempted to claim authority over the principal"
                        .to_string(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law() -> PrincipalSupremacy {
        PrincipalSupremacy
    }

    #[test]
    fn permits_single_principal() {
        let ctx =
            LawCheckContext::new("act-001", "principal.register").with_meta("principal_count", "1");
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn blocks_multiple_principals() {
        let ctx =
            LawCheckContext::new("act-002", "principal.register").with_meta("principal_count", "2");
        assert!(matches!(
            law().check(&ctx),
            Err(ConstitutionError::MultiplePrincipals { count: 2 })
        ));
    }

    #[test]
    fn blocks_principal_demotion() {
        let ctx = LawCheckContext::new("act-003", "principal.demote");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn blocks_principal_removal() {
        let ctx = LawCheckContext::new("act-004", "principal.remove");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn blocks_domain_authority_override() {
        let ctx = LawCheckContext::new("act-005", "skill.activate")
            .with_meta("claims_authority_over_principal", "true");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn permits_domain_internal_hierarchy() {
        // Domain skills can have internal hierarchy — just not over the principal
        let ctx = LawCheckContext::new("act-006", "skill.activate")
            .with_meta("domain_internal_hierarchy", "ceo > manager > employee");
        assert!(law().check(&ctx).is_ok());
    }
}
