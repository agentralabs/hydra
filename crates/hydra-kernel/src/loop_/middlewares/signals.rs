//! Signals middleware — hydra-signals emission per-request.
//!
//! Emits cycle signals to the signal fabric for cross-system awareness.

use hydra_animus::{PrimeGraph, Signal, SignalId, SignalTier, SignalWeight};
use hydra_signals::SignalFabric;

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::CycleResult;

pub struct SignalsMiddleware {
    fabric: SignalFabric,
}

impl SignalsMiddleware {
    pub fn new() -> Self {
        Self {
            fabric: SignalFabric::new(),
        }
    }
}

impl CycleMiddleware for SignalsMiddleware {
    fn name(&self) -> &'static str {
        "signals"
    }

    fn post_deliver(&mut self, _cycle: &CycleResult) {
        // Create a cycle-complete signal rooted at constitutional identity
        let weight = match SignalWeight::new(0.5) {
            Ok(w) => w,
            Err(_) => return,
        };
        let signal = Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            weight,
            SignalTier::Companion,
            0,
        );

        if let Err(e) = self.fabric.emit(signal) {
            eprintln!("hydra: signals post_deliver: {e}");
        }

        // Dispatch any queued signals
        let result = self.fabric.dispatch();
        if result.dispatched > 0 {
            eprintln!(
                "hydra: signals dispatched {} (unrouted={})",
                result.dispatched, result.unrouted
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signals_middleware_name() {
        let mw = SignalsMiddleware::new();
        assert_eq!(mw.name(), "signals");
    }
}

impl Default for SignalsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
