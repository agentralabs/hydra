use hydra_core::error::HydraError;
use hydra_core::types::{CognitivePhase, TokenBudget};

/// What conservation mode restricts
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConservationLimits {
    /// Skip reflection/learning phase (just log)
    pub skip_reflection: bool,
    /// Use cache-only for intent compilation (no LLM)
    pub cache_only: bool,
    /// Batch sister calls instead of individual
    pub batch_calls: bool,
}

impl Default for ConservationLimits {
    fn default() -> Self {
        Self {
            skip_reflection: true,
            cache_only: true,
            batch_calls: true,
        }
    }
}

/// Budget manager wrapping TokenBudget with phase-aware spending
pub struct BudgetManager {
    budget: TokenBudget,
    conservation_limits: ConservationLimits,
}

impl BudgetManager {
    pub fn new(total: u64) -> Self {
        Self {
            budget: TokenBudget::new(total),
            conservation_limits: ConservationLimits::default(),
        }
    }

    pub fn from_budget(budget: TokenBudget) -> Self {
        let conservation_mode = budget.conservation_mode;
        Self {
            budget,
            conservation_limits: if conservation_mode {
                ConservationLimits::default()
            } else {
                ConservationLimits {
                    skip_reflection: false,
                    cache_only: false,
                    batch_calls: false,
                }
            },
        }
    }

    pub fn budget(&self) -> &TokenBudget {
        &self.budget
    }

    pub fn into_budget(self) -> TokenBudget {
        self.budget
    }

    /// Spend tokens for a phase. Returns Err if can't afford.
    pub fn spend(&mut self, tokens: u64, phase: CognitivePhase) -> Result<(), HydraError> {
        if !self.budget.can_afford(tokens) {
            return Err(HydraError::TokenBudgetExceeded {
                needed: tokens,
                available: self.budget.remaining,
            });
        }
        self.budget.record_usage(tokens);
        // Check if we need to enter conservation mode
        if self.budget.conservation_mode && !self.conservation_limits.skip_reflection {
            self.enter_conservation_mode();
        }
        let _ = phase; // tracked in budget.per_phase
        Ok(())
    }

    /// Try to spend tokens. Returns false if can't afford.
    pub fn try_spend(&mut self, tokens: u64, phase: CognitivePhase) -> bool {
        self.spend(tokens, phase).is_ok()
    }

    /// Explicitly enter conservation mode — activates all limits
    pub fn enter_conservation_mode(&mut self) {
        self.budget.conservation_mode = true;
        self.conservation_limits = ConservationLimits::default();
    }

    /// Get current conservation limits
    pub fn conservation_limits(&self) -> &ConservationLimits {
        &self.conservation_limits
    }

    /// Check if in conservation mode (< 25% remaining)
    pub fn is_conservation_mode(&self) -> bool {
        self.budget.conservation_mode
    }

    /// Should skip reflection (Learn phase) in conservation mode?
    pub fn should_skip_reflection(&self) -> bool {
        self.budget.conservation_mode && self.conservation_limits.skip_reflection
    }

    /// Should use cache only (no LLM calls) in conservation mode?
    pub fn should_cache_only(&self) -> bool {
        self.budget.conservation_mode && self.conservation_limits.cache_only
    }

    /// Should batch sister calls in conservation mode?
    pub fn should_batch_calls(&self) -> bool {
        self.budget.conservation_mode && self.conservation_limits.batch_calls
    }

    /// Remaining tokens
    pub fn remaining(&self) -> u64 {
        self.budget.remaining
    }

    /// Check affordability
    pub fn can_afford(&self, tokens: u64) -> bool {
        self.budget.can_afford(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_budget_manager_has_full_remaining() {
        let bm = BudgetManager::new(10_000);
        assert_eq!(bm.remaining(), 10_000);
        assert!(!bm.is_conservation_mode());
    }

    #[test]
    fn test_spend_reduces_remaining() {
        let mut bm = BudgetManager::new(1_000);
        let ok = bm.try_spend(300, CognitivePhase::Perceive);
        assert!(ok);
        assert_eq!(bm.remaining(), 700);
    }

    #[test]
    fn test_spend_returns_error_when_insufficient() {
        let mut bm = BudgetManager::new(100);
        let result = bm.spend(200, CognitivePhase::Think);
        assert!(result.is_err());
        // Remaining should be unchanged
        assert_eq!(bm.remaining(), 100);
    }

    #[test]
    fn test_try_spend_returns_false_when_insufficient() {
        let mut bm = BudgetManager::new(50);
        assert!(!bm.try_spend(100, CognitivePhase::Act));
        assert_eq!(bm.remaining(), 50);
    }

    #[test]
    fn test_can_afford_exact_amount() {
        let bm = BudgetManager::new(500);
        assert!(bm.can_afford(500));
        assert!(!bm.can_afford(501));
    }

    #[test]
    fn test_can_afford_zero() {
        let bm = BudgetManager::new(0);
        assert!(bm.can_afford(0));
        assert!(!bm.can_afford(1));
    }

    #[test]
    fn test_conservation_mode_enters_when_below_threshold() {
        // TokenBudget enters conservation mode at < 25%
        let mut bm = BudgetManager::new(1_000);
        assert!(!bm.is_conservation_mode());
        // Spend 800, leaving 200 (20% < 25%)
        bm.try_spend(800, CognitivePhase::Think);
        assert!(bm.is_conservation_mode());
    }

    #[test]
    fn test_conservation_limits_default_all_true() {
        let limits = ConservationLimits::default();
        assert!(limits.skip_reflection);
        assert!(limits.cache_only);
        assert!(limits.batch_calls);
    }

    #[test]
    fn test_enter_conservation_mode_explicitly() {
        let mut bm = BudgetManager::new(10_000);
        assert!(!bm.is_conservation_mode());
        bm.enter_conservation_mode();
        assert!(bm.is_conservation_mode());
        assert!(bm.should_skip_reflection());
        assert!(bm.should_cache_only());
        assert!(bm.should_batch_calls());
    }

    #[test]
    fn test_from_budget_non_conservation() {
        let budget = TokenBudget::new(5_000);
        let bm = BudgetManager::from_budget(budget);
        assert!(!bm.is_conservation_mode());
        assert!(!bm.should_skip_reflection());
        assert!(!bm.should_cache_only());
        assert!(!bm.should_batch_calls());
    }

    #[test]
    fn test_from_budget_in_conservation() {
        let mut budget = TokenBudget::new(5_000);
        budget.conservation_mode = true;
        let bm = BudgetManager::from_budget(budget);
        assert!(bm.is_conservation_mode());
        assert!(bm.should_skip_reflection());
    }

    #[test]
    fn test_sequential_spending_across_phases() {
        let mut bm = BudgetManager::new(1_000);
        assert!(bm.try_spend(100, CognitivePhase::Perceive));
        assert!(bm.try_spend(500, CognitivePhase::Think));
        assert!(bm.try_spend(200, CognitivePhase::Decide));
        assert_eq!(bm.remaining(), 200);
        assert!(bm.try_spend(100, CognitivePhase::Act));
        assert!(bm.try_spend(100, CognitivePhase::Learn));
        assert_eq!(bm.remaining(), 0);
        assert!(!bm.try_spend(1, CognitivePhase::Perceive));
    }

    #[test]
    fn test_budget_accessor() {
        let bm = BudgetManager::new(42_000);
        assert_eq!(bm.budget().total, 42_000);
        assert_eq!(bm.budget().remaining, 42_000);
    }

    #[test]
    fn test_into_budget_consumes() {
        let bm = BudgetManager::new(7_777);
        let budget = bm.into_budget();
        assert_eq!(budget.total, 7_777);
    }
}
