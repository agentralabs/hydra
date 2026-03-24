//! Signals middleware — signal fabric + fleet + swarm monitoring.
//!
//! Emits cycle signals to the signal fabric for cross-system awareness.
//! Shares FleetRegistry with Swarm via Arc<Mutex<>> for unified state.

use std::sync::{Arc, Mutex};
use hydra_animus::{PrimeGraph, Signal, SignalId, SignalTier, SignalWeight};
use hydra_fleet::FleetRegistry;
use hydra_signals::SignalFabric;
use hydra_swarm::Swarm;

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct SignalsMiddleware {
    fabric: SignalFabric,
    swarm: Swarm,
    last_dispatched: usize,
    last_unrouted: usize,
    total_dispatched: usize,
}

impl SignalsMiddleware {
    pub fn new() -> Self {
        let fleet = Arc::new(Mutex::new(FleetRegistry::new()));
        Self {
            fabric: SignalFabric::new(),
            swarm: Swarm::shared(fleet),
            last_dispatched: 0,
            last_unrouted: 0,
            total_dispatched: 0,
        }
    }
}

impl CycleMiddleware for SignalsMiddleware {
    fn name(&self) -> &'static str { "signals" }

    fn enrich_prompt(&self, _perceived: &PerceivedInput) -> Vec<String> {
        let mut items = Vec::new();
        if self.total_dispatched > 0 {
            items.push(format!(
                "Signal fabric: {} total dispatched, {} last cycle ({} unrouted)",
                self.total_dispatched, self.last_dispatched, self.last_unrouted
            ));
        }
        let agent_count = self.swarm.agent_count();
        if agent_count > 0 {
            let health = self.swarm.health();
            items.push(format!("Fleet: {agent_count} agents, swarm {:?}", health.level));
        }
        items
    }

    fn post_deliver(&mut self, cycle: &CycleResult) {
        let weight = match SignalWeight::new(0.5) {
            Ok(w) => w,
            Err(_) => return,
        };
        let tier = if cycle.success { SignalTier::Companion } else { SignalTier::Adversarial };
        let signal = Signal::new(
            PrimeGraph::new(), SignalId::identity(), weight, tier,
            cycle.duration_ms.min(255) as u8,
        );
        if let Err(e) = self.fabric.emit(signal) {
            eprintln!("hydra: signals post_deliver: {e}");
        }
        let result = self.fabric.dispatch();
        self.last_dispatched = result.dispatched;
        self.last_unrouted = result.unrouted;
        self.total_dispatched += result.dispatched;
        let _ = self.swarm.lyapunov_delta();
    }
}

impl Default for SignalsMiddleware {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signals_middleware_name() {
        let mw = SignalsMiddleware::new();
        assert_eq!(mw.name(), "signals");
    }

    #[test]
    fn signals_starts_with_zero_dispatched() {
        let mw = SignalsMiddleware::new();
        assert_eq!(mw.total_dispatched, 0);
    }

    #[test]
    fn swarm_health_accessible() {
        let mw = SignalsMiddleware::new();
        let health = mw.swarm.health();
        assert_eq!(health.total_agents, 0);
    }
}
