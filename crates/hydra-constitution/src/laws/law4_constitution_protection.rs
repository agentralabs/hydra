//! Law 4: Constitution Protection
//! In the dependency graph G, hydra-constitution is a source node.
//! No runtime path may read, modify, or patch the constitution crate.

use crate::{
    constants::CONSTITUTION_ACCESS_ACTIONS,
    errors::ConstitutionError,
    laws::{ConstitutionalLaw, LawCheckContext, LawId},
};

pub struct ConstitutionProtection;

impl ConstitutionalLaw for ConstitutionProtection {
    fn law_id(&self) -> LawId {
        LawId::Law4ConstitutionProtection
    }

    fn description(&self) -> &'static str {
        "No skill, agent, or modification pipeline may reach the constitution crate. \
         The constitution crate has zero runtime Hydra dependencies."
    }

    fn check(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        let action = context.action_type.as_str();

        // Block any action explicitly targeting the constitution
        let targets_constitution = context.target.contains("constitution")
            || context.target.contains("hydra-constitution");

        let is_blocked_action = CONSTITUTION_ACCESS_ACTIONS
            .iter()
            .any(|a| action.starts_with(a));

        if is_blocked_action || targets_constitution {
            return Err(ConstitutionError::ConstitutionRuntimeAccess {
                caller: context.action_id.clone(),
            });
        }

        // Block modification pipeline actions targeting laws
        if action.starts_with("self_modify") {
            if let Some(target_crate) = context.metadata.get("target_crate") {
                if target_crate.contains("constitution") {
                    return Err(ConstitutionError::ConstitutionRuntimeAccess {
                        caller: context.action_id.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law() -> ConstitutionProtection {
        ConstitutionProtection
    }

    #[test]
    fn permits_normal_actions() {
        let ctx = LawCheckContext::new("act-001", "agent.spawn");
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn permits_self_modify_other_crate() {
        let ctx = LawCheckContext::new("act-002", "self_modify.apply_patch")
            .with_meta("target_crate", "hydra-kernel");
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn blocks_constitution_modify() {
        let ctx = LawCheckContext::new("act-003", "constitution.modify");
        assert!(matches!(
            law().check(&ctx),
            Err(ConstitutionError::ConstitutionRuntimeAccess { .. })
        ));
    }

    #[test]
    fn blocks_constitution_bypass() {
        let ctx = LawCheckContext::new("act-004", "constitution.bypass");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn blocks_self_modify_targeting_constitution() {
        let ctx = LawCheckContext::new("act-005", "self_modify.apply_patch")
            .with_meta("target_crate", "hydra-constitution");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn blocks_action_targeting_constitution() {
        let ctx = LawCheckContext::new("act-006", "file.write")
            .with_target("hydra-constitution/src/laws/law1.rs");
        assert!(law().check(&ctx).is_err());
    }
}
