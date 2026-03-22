//! SignalFabric — the top-level orchestrator tying all signal components together.
//!
//! The fabric owns the gate, queues, subscription registry, receipt log,
//! and audit trail. It provides `emit()`, `dispatch()`, and `status()`.

use crate::audit::{AuditAction, AuditEntry, SignalAuditTrail};
use crate::dispatch::{dispatch_cycle, DispatchResult};
use crate::gate::SignalGate;
use crate::queue::SignalQueues;
use crate::receipt::{DeliveryOutcome, DeliveryReceipt, DeliveryReceiptLog};
use crate::router::{fabric_route, signal_topic, FabricRoute};
use crate::subscription::{SubscriberId, SubscriptionRegistry};
use hydra_animus::Signal;

/// Status snapshot of the signal fabric.
#[derive(Debug)]
pub struct FabricStatus {
    /// Total signals currently queued.
    pub queued: usize,
    /// Total delivery receipts recorded.
    pub receipts: usize,
    /// Total audit trail entries.
    pub audit_entries: usize,
    /// Total active subscriptions.
    pub subscriptions: usize,
    /// Whether any queue is under backpressure.
    pub backpressure: bool,
}

/// The signal fabric — Hydra's live signal routing layer.
///
/// All inter-module signals flow through the fabric. The fabric enforces
/// constitutional compliance, routes by tier priority, and records
/// delivery receipts and audit trails for every signal.
pub struct SignalFabric {
    gate: SignalGate,
    queues: SignalQueues,
    subscriptions: SubscriptionRegistry,
    receipt_log: DeliveryReceiptLog,
    audit_trail: SignalAuditTrail,
}

impl SignalFabric {
    /// Create a new signal fabric with all components initialized.
    pub fn new() -> Self {
        Self {
            gate: SignalGate::new(),
            queues: SignalQueues::new(),
            subscriptions: SubscriptionRegistry::new(),
            receipt_log: DeliveryReceiptLog::new(),
            audit_trail: SignalAuditTrail::new(),
        }
    }

    /// Emit a signal into the fabric.
    ///
    /// The signal passes through the gate, is routed, and either queued
    /// for later dispatch or handled immediately. Orphan signals are
    /// silently rejected (Ok but not routed). Constitutional signals
    /// bypass queues entirely.
    pub fn emit(&mut self, signal: Signal) -> Result<(), crate::errors::SignalError> {
        let signal_id = signal.id.as_str().to_string();

        // Gate check
        if let Err(e) = self.gate.check(&signal) {
            self.audit_trail.record(AuditEntry::new(
                &signal_id,
                AuditAction::GateRejected {
                    reason: e.to_string(),
                },
            ));
            self.receipt_log.record(DeliveryReceipt::new(
                &signal_id,
                DeliveryOutcome::Rejected {
                    reason: e.to_string(),
                },
                "gate",
            ));
            // Orphan signals are rejected but not errored
            return Ok(());
        }

        self.audit_trail
            .record(AuditEntry::new(&signal_id, AuditAction::GatePassed));

        // Route
        let route = fabric_route(&signal);
        self.audit_trail.record(AuditEntry::new(
            &signal_id,
            AuditAction::Routed {
                route: format!("{:?}", route),
            },
        ));

        match route {
            FabricRoute::ConstitutionImmediate => {
                // Constitutional signals get immediate delivery receipts
                self.receipt_log.record(DeliveryReceipt::new(
                    &signal_id,
                    DeliveryOutcome::Delivered,
                    "constitution-handler",
                ));
                self.audit_trail.record(AuditEntry::new(
                    &signal_id,
                    AuditAction::Dispatched {
                        handler: "constitution-handler".to_string(),
                    },
                ));
            }
            FabricRoute::AdversaryImmediate => {
                self.receipt_log.record(DeliveryReceipt::new(
                    &signal_id,
                    DeliveryOutcome::Delivered,
                    "adversary-handler",
                ));
                self.audit_trail.record(AuditEntry::new(
                    &signal_id,
                    AuditAction::Dispatched {
                        handler: "adversary-handler".to_string(),
                    },
                ));
            }
            FabricRoute::QueueForDispatch { .. } => {
                let tier_name = signal_topic(&signal).to_string();
                self.queues.enqueue(signal)?;
                self.audit_trail.record(AuditEntry::new(
                    &signal_id,
                    AuditAction::Enqueued { tier: tier_name },
                ));
            }
            FabricRoute::Drop { reason } => {
                self.receipt_log.record(DeliveryReceipt::new(
                    &signal_id,
                    DeliveryOutcome::Dropped {
                        reason: reason.clone(),
                    },
                    "fabric",
                ));
                self.audit_trail
                    .record(AuditEntry::new(&signal_id, AuditAction::Dropped { reason }));
            }
        }

        Ok(())
    }

    /// Run one dispatch cycle, draining all queues in priority order.
    ///
    /// Returns the number of dispatched and unrouted signals.
    pub fn dispatch(&mut self) -> DispatchResult {
        dispatch_cycle(&mut self.queues, &self.subscriptions, &mut self.receipt_log)
    }

    /// Subscribe a handler to a signal topic.
    pub fn subscribe(
        &mut self,
        topic: &str,
        subscriber_id: SubscriberId,
        label: &str,
    ) -> Result<(), crate::errors::SignalError> {
        self.subscriptions
            .subscribe(topic, subscriber_id, label)
            .map(|_| ())
    }

    /// Get a snapshot of the fabric's current status.
    pub fn status(&self) -> FabricStatus {
        FabricStatus {
            queued: self.queues.total_count(),
            receipts: self.receipt_log.len(),
            audit_entries: self.audit_trail.len(),
            subscriptions: self.subscriptions.total_subscriptions(),
            backpressure: self.queues.any_backpressure(),
        }
    }

    /// Returns a reference to the receipt log.
    pub fn receipt_log(&self) -> &DeliveryReceiptLog {
        &self.receipt_log
    }

    /// Returns a reference to the audit trail.
    pub fn audit_trail(&self) -> &SignalAuditTrail {
        &self.audit_trail
    }
}

impl Default for SignalFabric {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_animus::{
        graph::PrimeGraph,
        semiring::signal::{SignalId, SignalTier, SignalWeight},
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
    fn emit_valid_fleet_signal() {
        let mut fabric = SignalFabric::new();
        assert!(fabric.emit(make_signal(SignalTier::Fleet)).is_ok());
        assert_eq!(fabric.status().queued, 1);
    }

    #[test]
    fn emit_orphan_signal_silently_rejected() {
        let mut fabric = SignalFabric::new();
        let mut signal = make_signal(SignalTier::Fleet);
        signal.causal_chain.clear();
        // Orphan signals return Ok but are not queued
        assert!(fabric.emit(signal).is_ok());
        assert_eq!(fabric.status().queued, 0);
        assert!(fabric.status().receipts > 0);
    }

    #[test]
    fn constitutional_signal_bypasses_queue() {
        let mut fabric = SignalFabric::new();
        assert!(fabric.emit(make_signal(SignalTier::Constitution)).is_ok());
        // Not queued — handled immediately
        assert_eq!(fabric.status().queued, 0);
        // But a receipt was created
        assert!(fabric.status().receipts > 0);
    }

    #[test]
    fn dispatch_returns_counts() {
        let mut fabric = SignalFabric::new();
        fabric
            .subscribe(
                "signal.fleet",
                SubscriberId::from_value("sub-1"),
                "fleet-handler",
            )
            .unwrap();
        fabric.emit(make_signal(SignalTier::Fleet)).unwrap();
        fabric.emit(make_signal(SignalTier::Companion)).unwrap();

        let result = fabric.dispatch();
        // Fleet was dispatched (has subscriber), companion was unrouted
        assert_eq!(result.dispatched, 1);
        assert_eq!(result.unrouted, 1);
    }
}
