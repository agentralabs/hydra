//! Federation middleware — peer discovery, consensus, collective intelligence.

use hydra_collective::PatternObservation;
use hydra_consensus::ConsensusEngine;
use hydra_diplomat::DiplomatEngine;
use hydra_federation::FederationEngine;

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct FederationMiddleware {
    federation: FederationEngine,
    diplomat: DiplomatEngine,
    consensus: ConsensusEngine,
    cycles_shared: u64,
}

impl FederationMiddleware {
    pub fn new() -> Self {
        Self {
            federation: FederationEngine::new("hydra-local"),
            diplomat: DiplomatEngine::new(),
            consensus: ConsensusEngine::new(),
            cycles_shared: 0,
        }
    }
}

impl Default for FederationMiddleware {
    fn default() -> Self { Self::new() }
}

impl CycleMiddleware for FederationMiddleware {
    fn name(&self) -> &'static str { "federation" }

    fn enrich_prompt(&self, _perceived: &PerceivedInput) -> Vec<String> {
        let peer_count = self.federation.peer_count();
        if peer_count > 0 {
            vec![format!("Federation: {peer_count} peers connected")]
        } else {
            Vec::new()
        }
    }

    fn post_deliver(&mut self, _cycle: &CycleResult) {
        self.cycles_shared += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn federation_middleware_name() {
        let mw = FederationMiddleware::new();
        assert_eq!(mw.name(), "federation");
    }
}
