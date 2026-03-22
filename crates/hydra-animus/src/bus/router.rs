//! Signal routing by tier and weight.

use crate::semiring::signal::{Signal, SignalTier};

/// Routing decision for a signal.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingDecision {
    /// Route to constitutional handler immediately.
    ConstitutionImmediate,
    /// Route to adversary handler immediately.
    AdversaryImmediate,
    /// Route to standard handler with the given priority.
    Standard { priority: u8 },
    /// Drop the signal (weight below floor).
    Drop { reason: String },
}

/// Determine how a signal should be routed.
pub fn route(signal: &Signal) -> RoutingDecision {
    if signal.tier == SignalTier::Constitution {
        return RoutingDecision::ConstitutionImmediate;
    }

    if signal.tier == SignalTier::Adversarial {
        return RoutingDecision::AdversaryImmediate;
    }

    if signal.is_orphan() {
        return RoutingDecision::Drop {
            reason: format!("orphan signal: chain incomplete for '{}'", signal.id),
        };
    }

    RoutingDecision::Standard {
        priority: signal.tier.clone() as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        graph::PrimeGraph,
        semiring::signal::{SignalId, SignalWeight},
    };

    fn signal(tier: SignalTier) -> Signal {
        Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            tier,
            3,
        )
    }

    #[test]
    fn constitution_signal_routes_immediately() {
        assert_eq!(
            route(&signal(SignalTier::Constitution)),
            RoutingDecision::ConstitutionImmediate
        );
    }

    #[test]
    fn adversarial_signal_routes_immediately() {
        assert_eq!(
            route(&signal(SignalTier::Adversarial)),
            RoutingDecision::AdversaryImmediate
        );
    }

    #[test]
    fn fleet_signal_routes_standard() {
        assert!(matches!(
            route(&signal(SignalTier::Fleet)),
            RoutingDecision::Standard { .. }
        ));
    }

    #[test]
    fn orphan_signal_is_dropped() {
        let mut s = signal(SignalTier::Fleet);
        s.causal_chain.clear();
        assert!(matches!(route(&s), RoutingDecision::Drop { .. }));
    }
}
