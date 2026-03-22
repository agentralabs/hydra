//! Law 7: Causal Chain Completeness
//! ∀ action a: ∃ chain a = a₁ ⊗ a₂ ⊗ ... ⊗ aₙ
//! where aₙ = CONSTITUTIONAL_IDENTITY_ID (the semiring multiplicative identity).
//! Orphan actions (no chain) are blocked. Incomplete chains are blocked.

use crate::{
    constants::{CAUSAL_CHAIN_MAX_DEPTH, CONSTITUTIONAL_IDENTITY_ID},
    errors::ConstitutionError,
    laws::{ConstitutionalLaw, LawCheckContext, LawId},
};

pub struct CausalChainCompleteness;

impl ConstitutionalLaw for CausalChainCompleteness {
    fn law_id(&self) -> LawId {
        LawId::Law7CausalChainCompleteness
    }

    fn description(&self) -> &'static str {
        "Every action Hydra takes must have a traceable causal origin. \
         No orphan actions. If the chain cannot be traced, the action is blocked. \
         Applies in every domain, every network, every system."
    }

    fn check(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        self.check_chain_not_empty(context)?;
        self.check_chain_terminates_at_identity(context)?;
        self.check_chain_not_malformed(context)?;
        Ok(())
    }
}

impl CausalChainCompleteness {
    fn check_chain_not_empty(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        if context.causal_chain.is_empty() {
            return Err(ConstitutionError::OrphanAction {
                action_id: context.action_id.clone(),
            });
        }

        Ok(())
    }

    fn check_chain_terminates_at_identity(
        &self,
        context: &LawCheckContext,
    ) -> Result<(), ConstitutionError> {
        let terminates = context
            .causal_chain
            .last()
            .map(|id| id == CONSTITUTIONAL_IDENTITY_ID)
            .unwrap_or(false);

        if !terminates {
            return Err(ConstitutionError::CausalChainIncomplete);
        }

        Ok(())
    }

    fn check_chain_not_malformed(
        &self,
        context: &LawCheckContext,
    ) -> Result<(), ConstitutionError> {
        if context.causal_chain.len() > CAUSAL_CHAIN_MAX_DEPTH {
            return Err(ConstitutionError::LawViolation {
                law: LawId::Law7CausalChainCompleteness,
                reason: format!(
                    "causal chain depth {} exceeds maximum {}",
                    context.causal_chain.len(),
                    CAUSAL_CHAIN_MAX_DEPTH
                ),
            });
        }

        // Every entry must be non-empty
        for (i, entry) in context.causal_chain.iter().enumerate() {
            if entry.is_empty() {
                return Err(ConstitutionError::LawViolation {
                    law: LawId::Law7CausalChainCompleteness,
                    reason: format!("causal chain entry at index {} is empty", i),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::CONSTITUTIONAL_IDENTITY_ID;

    fn law() -> CausalChainCompleteness {
        CausalChainCompleteness
    }

    fn valid_chain() -> Vec<String> {
        vec![
            "intent-omoshola-001".to_string(),
            CONSTITUTIONAL_IDENTITY_ID.to_string(),
        ]
    }

    #[test]
    fn permits_complete_chain() {
        let ctx = LawCheckContext::new("act-001", "agent.spawn").with_causal_chain(valid_chain());
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn permits_single_link_chain() {
        // Minimal valid chain: directly from constitutional identity
        let ctx = LawCheckContext::new("act-002", "receipt.write")
            .with_causal_chain(vec![CONSTITUTIONAL_IDENTITY_ID.to_string()]);
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn blocks_empty_chain() {
        let ctx = LawCheckContext::new("act-003", "agent.spawn").with_causal_chain(vec![]);
        assert!(matches!(
            law().check(&ctx),
            Err(ConstitutionError::OrphanAction { .. })
        ));
    }

    #[test]
    fn blocks_chain_not_terminating_at_identity() {
        let ctx = LawCheckContext::new("act-004", "agent.spawn")
            .with_causal_chain(vec!["some-random-id".to_string()]);
        assert!(matches!(
            law().check(&ctx),
            Err(ConstitutionError::CausalChainIncomplete)
        ));
    }

    #[test]
    fn blocks_chain_with_empty_entry() {
        let ctx = LawCheckContext::new("act-005", "agent.spawn")
            .with_causal_chain(vec!["".to_string(), CONSTITUTIONAL_IDENTITY_ID.to_string()]);
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn permits_deep_chain() {
        let mut chain: Vec<String> = (0..100).map(|i| format!("action-{:04}", i)).collect();
        chain.push(CONSTITUTIONAL_IDENTITY_ID.to_string());
        let ctx = LawCheckContext::new("act-006", "agent.spawn").with_causal_chain(chain);
        assert!(law().check(&ctx).is_ok());
    }
}
