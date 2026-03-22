//! ConstitutionChecker: validates any proposed action against all seven laws.
//! This is the single entry point for all constitutional enforcement.

use crate::{
    errors::ConstitutionError,
    laws::{
        AnimusIntegrity, CausalChainCompleteness, ConstitutionProtection, ConstitutionalLaw,
        IdentityIntegrity, LawCheckContext, LawId, MemorySovereignty, PrincipalSupremacy,
        ReceiptImmutability,
    },
};

/// The result of a constitutional check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// True if the action is permitted (all laws pass).
    pub permitted: bool,

    /// All violations found. Empty if permitted is true.
    pub violations: Vec<ConstitutionError>,

    /// Which laws were checked.
    pub laws_checked: Vec<LawId>,
}

impl CheckResult {
    /// Returns true if the action passed all constitutional laws.
    pub fn is_permitted(&self) -> bool {
        self.permitted
    }

    /// Returns the first violation, if any.
    pub fn first_violation(&self) -> Option<&ConstitutionError> {
        self.violations.first()
    }

    /// Returns a single consolidated error if there are violations.
    pub fn into_result(self) -> Result<(), ConstitutionError> {
        if self.permitted {
            Ok(())
        } else {
            Err(self
                .violations
                .into_iter()
                .next()
                .unwrap_or(ConstitutionError::LawViolation {
                    law: LawId::Law1ReceiptImmutability,
                    reason: "unknown violation".to_string(),
                }))
        }
    }
}

/// The constitutional checker.
/// Holds all seven laws and checks them in order.
/// Laws are checked in declaration order: Law 1 first, Law 7 last.
/// All violations are collected — not just the first.
pub struct ConstitutionChecker {
    laws: Vec<Box<dyn ConstitutionalLaw>>,
}

impl ConstitutionChecker {
    /// Creates a new checker with all seven laws active.
    pub fn new() -> Self {
        Self {
            laws: vec![
                Box::new(ReceiptImmutability),
                Box::new(IdentityIntegrity),
                Box::new(MemorySovereignty),
                Box::new(ConstitutionProtection),
                Box::new(AnimusIntegrity),
                Box::new(PrincipalSupremacy),
                Box::new(CausalChainCompleteness),
            ],
        }
    }

    /// Check a proposed action against all seven constitutional laws.
    /// Returns a CheckResult describing all violations found.
    /// The action MUST NOT proceed if CheckResult::is_permitted() is false.
    pub fn check(&self, context: &LawCheckContext) -> CheckResult {
        let mut violations = Vec::new();
        let mut laws_checked = Vec::new();

        for law in &self.laws {
            laws_checked.push(law.law_id());
            if let Err(violation) = law.check(context) {
                violations.push(violation);
            }
        }

        let permitted = violations.is_empty();
        CheckResult {
            permitted,
            violations,
            laws_checked,
        }
    }

    /// Convenience: check and return Ok/Err directly.
    /// Returns the first violation error if any law is violated.
    pub fn check_strict(&self, context: &LawCheckContext) -> Result<(), ConstitutionError> {
        self.check(context).into_result()
    }

    /// Returns the number of laws in this checker (always 7).
    pub fn law_count(&self) -> usize {
        self.laws.len()
    }
}

impl Default for ConstitutionChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// The result of a full constitutional check (laws + declarations).
#[derive(Debug, Clone)]
pub struct FullCheckResult {
    /// The law check result.
    pub law_result: CheckResult,
    /// Capability declaration result (None if no capability context).
    pub capability_result: Option<Result<(), ConstitutionError>>,
    /// Growth declaration result (None if no growth context).
    pub growth_result: Option<Result<(), ConstitutionError>>,
}

impl FullCheckResult {
    /// Returns true if all checks passed.
    pub fn is_permitted(&self) -> bool {
        if !self.law_result.is_permitted() {
            return false;
        }
        if let Some(ref r) = self.capability_result {
            if r.is_err() {
                return false;
            }
        }
        if let Some(ref r) = self.growth_result {
            if r.is_err() {
                return false;
            }
        }
        true
    }
}

impl ConstitutionChecker {
    /// Check laws, capability declaration, and growth declaration together.
    pub fn full_check(
        &self,
        law_ctx: &LawCheckContext,
        capability_ctx: Option<&crate::declarations::CapabilityCheckContext>,
        growth_ctx: Option<&crate::declarations::GrowthCheckContext>,
    ) -> FullCheckResult {
        let law_result = self.check(law_ctx);
        let capability_result =
            capability_ctx.map(crate::declarations::check_capability_declaration);
        let growth_result = growth_ctx.map(crate::declarations::check_growth_declaration);
        FullCheckResult {
            law_result,
            capability_result,
            growth_result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::CONSTITUTIONAL_IDENTITY_ID;

    fn checker() -> ConstitutionChecker {
        ConstitutionChecker::new()
    }

    fn valid_ctx(action_type: &str) -> LawCheckContext {
        LawCheckContext::new("act-test", action_type)
            .with_causal_chain(vec![CONSTITUTIONAL_IDENTITY_ID.to_string()])
    }

    #[test]
    fn has_seven_laws() {
        assert_eq!(checker().law_count(), 7);
    }

    #[test]
    fn permits_clean_action() {
        let ctx = valid_ctx("agent.spawn");
        let result = checker().check(&ctx);
        assert!(result.is_permitted());
        assert!(result.violations.is_empty());
        assert_eq!(result.laws_checked.len(), 7);
    }

    #[test]
    fn blocks_receipt_deletion() {
        let ctx = valid_ctx("receipt.delete");
        let result = checker().check(&ctx);
        assert!(!result.is_permitted());
        assert!(!result.violations.is_empty());
    }

    #[test]
    fn blocks_orphan_action() {
        let ctx = LawCheckContext::new("act-orphan", "agent.spawn").with_causal_chain(vec![]);
        let result = checker().check(&ctx);
        assert!(!result.is_permitted());
    }

    #[test]
    fn collects_multiple_violations() {
        // This action violates Law 1 (receipt mutation) and
        // has an incomplete causal chain (Law 7)
        let ctx = LawCheckContext::new("act-multi", "receipt.delete")
            .with_causal_chain(vec!["non-constitutional-id".to_string()]);
        let result = checker().check(&ctx);
        assert!(!result.is_permitted());
        assert!(result.violations.len() >= 2);
    }

    #[test]
    fn check_strict_returns_err_on_violation() {
        let ctx = valid_ctx("receipt.delete");
        assert!(checker().check_strict(&ctx).is_err());
    }

    #[test]
    fn check_strict_returns_ok_on_clean() {
        let ctx = valid_ctx("memory.write");
        assert!(checker().check_strict(&ctx).is_ok());
    }
}
