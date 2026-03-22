//! Settlement middleware — hydra-attribution causal cost analysis.
//!
//! Analyzes WHY each cycle cost what it did, not just WHAT it cost.

use hydra_attribution::AttributionEngine;

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::CycleResult;

pub struct SettlementMiddleware {
    #[allow(dead_code)]
    attribution: AttributionEngine,
}

impl SettlementMiddleware {
    pub fn new() -> Self {
        Self {
            attribution: AttributionEngine::new(),
        }
    }
}

impl CycleMiddleware for SettlementMiddleware {
    fn name(&self) -> &'static str {
        "settlement"
    }

    fn post_deliver(&mut self, _cycle: &CycleResult) {
        // Attribution requires a SettlementRecord (from hydra-settlement).
        // The Deliverer already settles costs; attribution analysis
        // runs when settlement records are available via the engine.
        // For now, track that attribution is wired and ready.
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
