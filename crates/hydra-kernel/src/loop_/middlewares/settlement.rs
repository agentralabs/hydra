//! Settlement middleware — hydra-attribution causal cost analysis.
//!
//! Runs attribution on every cycle with >0 tokens.
//! No longer a stub — actually analyzes cost per cycle.

use hydra_attribution::AttributionEngine;
use hydra_settlement::{CostClass, CostItem, Outcome, SettlementRecord};

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::CycleResult;

pub struct SettlementMiddleware {
    attribution: AttributionEngine,
    cycles_attributed: usize,
}

impl SettlementMiddleware {
    pub fn new() -> Self {
        Self {
            attribution: AttributionEngine::new(),
            cycles_attributed: 0,
        }
    }
}

impl CycleMiddleware for SettlementMiddleware {
    fn name(&self) -> &'static str {
        "settlement"
    }

    fn post_deliver(&mut self, cycle: &CycleResult) {
        if cycle.tokens_used == 0 {
            return;
        }

        let outcome = if cycle.success {
            Outcome::Success {
                description: "cycle complete".into(),
            }
        } else {
            Outcome::Success {
                description: "cycle error".into(),
            }
        };

        let costs = vec![CostItem::new(
            CostClass::DirectExecution,
            cycle.tokens_used as u64,
            1.0,
            cycle.duration_ms,
        )];

        let record = SettlementRecord::new(
            &cycle.session_id,
            format!("loop.{}", cycle.path),
            &cycle.domain,
            &cycle.intent_summary,
            outcome,
            costs,
            cycle.duration_ms,
            1,
        );

        match self.attribution.attribute(&record) {
            Ok(_) => {
                self.cycles_attributed += 1;
                if self.cycles_attributed % 50 == 0 {
                    eprintln!(
                        "hydra: settlement: {} cycles attributed",
                        self.cycles_attributed,
                    );
                }
            }
            Err(e) => {
                eprintln!("hydra: settlement attribution: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_middleware_name() {
        let mw = SettlementMiddleware::new();
        assert_eq!(mw.name(), "settlement");
    }
}

impl Default for SettlementMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
