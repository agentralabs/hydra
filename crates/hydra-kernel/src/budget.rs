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
