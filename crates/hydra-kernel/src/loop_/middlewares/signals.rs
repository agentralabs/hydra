//! Signals middleware — hydra-signals emission per-request.
//!
//! Emits cycle signals to the signal fabric for cross-system awareness.
//! Captures dispatch results into enrichments so the TUI can surface them.

use hydra_animus::{PrimeGraph, Signal, SignalId, SignalTier, SignalWeight};
use hydra_signals::SignalFabric;

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct SignalsMiddleware {
    fabric: SignalFabric,
    last_dispatched: usize,
    last_unrouted: usize,
    total_dispatched: usize,
}

impl SignalsMiddleware {
    pub fn new() -> Self {
        Self {
            fabric: SignalFabric::new(),
            last_dispatched: 0,
            last_unrouted: 0,
            total_dispatched: 0,
        }
    }
}

impl CycleMiddleware for SignalsMiddleware {
    fn name(&self) -> &'static str {
        "signals"
    }

    fn enrich_prompt(&self, _perceived: &PerceivedInput) -> Vec<String> {
        if self.total_dispatched > 0 {
            vec![format!(
                "Signal fabric: {} total dispatched, {} last cycle ({} unrouted)",
                self.total_dispatched, self.last_dispatched, self.last_unrouted
            )]
        } else {
            Vec::new()
        }
    }

    fn post_deliver(&mut self, cycle: &CycleResult) {
        // Create a cycle-complete signal with context from the actual cycle
        let weight = match SignalWeight::new(0.5) {
            Ok(w) => w,
            Err(_) => return,
        };

        let tier = if cycle.success {
            SignalTier::Companion
        } else {
            SignalTier::Adversarial
        };

        let signal = Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            weight,
            tier,
            cycle.duration_ms.min(255) as u8,
        );

        if let Err(e) = self.fabric.emit(signal) {
            eprintln!("hydra: signals post_deliver: {e}");
        }

        // Dispatch queued signals and capture results
        let result = self.fabric.dispatch();
        self.last_dispatched = result.dispatched;
        self.last_unrouted = result.unrouted;
        self.total_dispatched += result.dispatched;
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

    #[test]
    fn signals_starts_with_zero_dispatched() {
        let mw = SignalsMiddleware::new();
        assert_eq!(mw.total_dispatched, 0);
    }
}

impl Default for SignalsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
