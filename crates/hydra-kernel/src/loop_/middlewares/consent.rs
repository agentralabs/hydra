//! Consent middleware — sharing policies, exchange, influence, legacy.

use hydra_consent::ConsentRegistry;
use hydra_exchange::ExchangeEngine;
use hydra_influence::PublishedPattern;
use hydra_legacy::LegacyEngine;

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct ConsentMiddleware {
    consent: ConsentRegistry,
    exchange: ExchangeEngine,
    legacy: LegacyEngine,
    shares_total: u64,
}

impl ConsentMiddleware {
    pub fn new() -> Self {
        Self {
            consent: ConsentRegistry::new(),
            exchange: ExchangeEngine::new(),
            legacy: LegacyEngine::new(),
            shares_total: 0,
        }
    }
}

impl Default for ConsentMiddleware {
    fn default() -> Self { Self::new() }
}

impl CycleMiddleware for ConsentMiddleware {
    fn name(&self) -> &'static str { "consent" }

    fn enrich_prompt(&self, _perceived: &PerceivedInput) -> Vec<String> {
        let grants = self.consent.active_grant_count();
        if grants > 0 {
            vec![format!("Consent: {grants} active sharing grants")]
        } else {
            Vec::new()
        }
    }

    fn post_deliver(&mut self, _cycle: &CycleResult) {
        self.shares_total += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consent_middleware_name() {
        let mw = ConsentMiddleware::new();
        assert_eq!(mw.name(), "consent");
    }
}
