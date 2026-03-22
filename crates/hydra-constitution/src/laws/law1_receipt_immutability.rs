//! Law 1: Receipt Immutability
//! The receipt set R is monotonically increasing.
//! ∀t₁ < t₂: R(t₁) ⊆ R(t₂)
//! No receipt may be deleted, modified, or suppressed after creation.

use crate::{
    constants::RECEIPT_MUTATION_PREFIXES,
    errors::ConstitutionError,
    laws::{ConstitutionalLaw, LawCheckContext, LawId},
};

pub struct ReceiptImmutability;

impl ConstitutionalLaw for ReceiptImmutability {
    fn law_id(&self) -> LawId {
        LawId::Law1ReceiptImmutability
    }

    fn description(&self) -> &'static str {
        "No action may delete, modify, or suppress a receipt after it is written. \
         A receipt is a fact that happened. Facts do not die."
    }

    fn check(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        let action = context.action_type.as_str();
        let is_mutation = RECEIPT_MUTATION_PREFIXES
            .iter()
            .any(|prefix| action.starts_with(prefix));

        if is_mutation {
            return Err(ConstitutionError::ReceiptMutationAttempt {
                receipt_id: context.target.clone(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law() -> ReceiptImmutability {
        ReceiptImmutability
    }

    #[test]
    fn permits_receipt_write() {
        let ctx = LawCheckContext::new("act-001", "receipt.write");
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn permits_receipt_read() {
        let ctx = LawCheckContext::new("act-002", "receipt.read");
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn permits_unrelated_action() {
        let ctx = LawCheckContext::new("act-003", "agent.spawn");
        assert!(law().check(&ctx).is_ok());
    }

    #[test]
    fn blocks_receipt_delete() {
        let ctx = LawCheckContext::new("act-004", "receipt.delete").with_target("rcpt-abc");
        assert!(matches!(
            law().check(&ctx),
            Err(ConstitutionError::ReceiptMutationAttempt { .. })
        ));
    }

    #[test]
    fn blocks_receipt_modify() {
        let ctx = LawCheckContext::new("act-005", "receipt.modify").with_target("rcpt-xyz");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn blocks_receipt_suppress() {
        let ctx = LawCheckContext::new("act-006", "receipt.suppress");
        assert!(law().check(&ctx).is_err());
    }

    #[test]
    fn blocks_receipt_purge() {
        let ctx = LawCheckContext::new("act-007", "receipt.purge");
        assert!(law().check(&ctx).is_err());
    }
}
