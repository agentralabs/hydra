//! Signal routing — maps animus routing decisions to fabric routes.
//!
//! Uses `hydra_animus::bus::router::route` internally and translates
//! the result into fabric-level routing decisions.

use hydra_animus::bus::router::route as animus_route;
use hydra_animus::bus::router::RoutingDecision;
use hydra_animus::{Signal, SignalTier};

/// Fabric-level routing decision for a signal.
#[derive(Debug, Clone, PartialEq)]
pub enum FabricRoute {
    /// Route immediately to constitutional handler — bypass all queues.
    ConstitutionImmediate,
    /// Route immediately to adversarial handler.
    AdversaryImmediate,
    /// Route to a standard tier queue for later dispatch.
    QueueForDispatch {
        /// Which tier queue to place the signal in.
        tier: SignalTier,
    },
    /// Drop the signal entirely.
    Drop {
        /// The reason for dropping.
        reason: String,
    },
}

/// Determine the fabric route for a signal.
///
/// Translates the animus-level routing decision into a fabric-level route
/// that determines whether the signal is queued, handled immediately, or dropped.
pub fn fabric_route(signal: &Signal) -> FabricRoute {
    match animus_route(signal) {
        RoutingDecision::ConstitutionImmediate => FabricRoute::ConstitutionImmediate,
        RoutingDecision::AdversaryImmediate => FabricRoute::AdversaryImmediate,
        RoutingDecision::Standard { priority: _ } => FabricRoute::QueueForDispatch {
            tier: signal.tier.clone(),
        },
        RoutingDecision::Drop { reason } => FabricRoute::Drop { reason },
    }
}

/// Derive the topic string for a signal based on its tier.
///
/// Topic strings follow the pattern "signal.<tier_name>".
pub fn signal_topic(signal: &Signal) -> &'static str {
    match signal.tier {
        SignalTier::Constitution => "signal.constitution",
        SignalTier::Adversarial => "signal.adversarial",
        SignalTier::BeliefRevision => "signal.belief_revision",
        SignalTier::Fleet => "signal.fleet",
        SignalTier::Companion => "signal.companion",
        SignalTier::Prediction => "signal.prediction",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_animus::{
        graph::PrimeGraph,
        semiring::signal::{SignalId, SignalWeight},
    };

    fn make_signal(tier: SignalTier) -> Signal {
        Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            tier,
            3,
        )
    }

    #[test]
    fn constitution_routes_immediately() {
        assert_eq!(
            fabric_route(&make_signal(SignalTier::Constitution)),
            FabricRoute::ConstitutionImmediate
        );
    }

    #[test]
    fn adversarial_routes_immediately() {
        assert_eq!(
            fabric_route(&make_signal(SignalTier::Adversarial)),
            FabricRoute::AdversaryImmediate
        );
    }

    #[test]
    fn fleet_routes_to_queue() {
        let route = fabric_route(&make_signal(SignalTier::Fleet));
        assert!(matches!(
            route,
            FabricRoute::QueueForDispatch {
                tier: SignalTier::Fleet
            }
        ));
    }

    #[test]
    fn orphan_signal_dropped() {
        let mut signal = make_signal(SignalTier::Fleet);
        signal.causal_chain.clear();
        assert!(matches!(fabric_route(&signal), FabricRoute::Drop { .. }));
    }

    #[test]
    fn topic_strings_correct() {
        assert_eq!(
            signal_topic(&make_signal(SignalTier::Fleet)),
            "signal.fleet"
        );
        assert_eq!(
            signal_topic(&make_signal(SignalTier::Constitution)),
            "signal.constitution"
        );
        assert_eq!(
            signal_topic(&make_signal(SignalTier::Prediction)),
            "signal.prediction"
        );
    }
}
