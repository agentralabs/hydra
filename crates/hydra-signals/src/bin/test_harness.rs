//! Test harness for hydra-signals — runs all scenario tests.
//!
//! This binary exercises the signal fabric end-to-end and reports
//! pass/fail for each scenario.

use hydra_animus::{
    graph::PrimeGraph,
    semiring::signal::{Signal, SignalId, SignalTier, SignalWeight},
};
use hydra_signals::{
    audit::{AuditAction, AuditEntry, SignalAuditTrail},
    fabric::SignalFabric,
    gate::SignalGate,
    queue::{SignalQueues, TierQueue},
    receipt::{DeliveryOutcome, DeliveryReceipt, DeliveryReceiptLog},
    router::{fabric_route, signal_topic, FabricRoute},
    subscription::{SubscriberId, SubscriptionRegistry},
    weight::compute_signal_weight,
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

fn make_orphan(tier: SignalTier) -> Signal {
    let mut s = make_signal(tier);
    s.causal_chain.clear();
    s
}

struct Harness {
    passed: usize,
    failed: usize,
}

impl Harness {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
        }
    }

    fn run(&mut self, name: &str, f: impl FnOnce() -> bool) {
        let ok = f();
        if ok {
            println!("  PASS: {}", name);
            self.passed += 1;
        } else {
            println!("  FAIL: {}", name);
            self.failed += 1;
        }
    }

    fn report(&self) {
        println!(
            "\n{} passed, {} failed, {} total",
            self.passed,
            self.failed,
            self.passed + self.failed
        );
        if self.failed > 0 {
            std::process::exit(1);
        }
    }
}

fn main() {
    println!("hydra-signals test harness\n");
    let mut h = Harness::new();

    // --- Gate tests ---
    println!("[Gate]");
    h.run("valid signal passes gate", || {
        let gate = SignalGate::new();
        gate.check(&make_signal(SignalTier::Fleet)).is_ok()
    });
    h.run("orphan signal rejected at gate", || {
        let gate = SignalGate::new();
        gate.check(&make_orphan(SignalTier::Fleet)).is_err()
    });
    h.run("constitutional signal passes gate", || {
        let gate = SignalGate::new();
        gate.check(&make_signal(SignalTier::Constitution)).is_ok()
    });

    // --- Router tests ---
    println!("\n[Router]");
    h.run("constitution routes immediately", || {
        fabric_route(&make_signal(SignalTier::Constitution)) == FabricRoute::ConstitutionImmediate
    });
    h.run("adversarial routes immediately", || {
        fabric_route(&make_signal(SignalTier::Adversarial)) == FabricRoute::AdversaryImmediate
    });
    h.run("fleet routes to queue", || {
        matches!(
            fabric_route(&make_signal(SignalTier::Fleet)),
            FabricRoute::QueueForDispatch { .. }
        )
    });
    h.run("orphan dropped by router", || {
        matches!(
            fabric_route(&make_orphan(SignalTier::Fleet)),
            FabricRoute::Drop { .. }
        )
    });

    // --- Queue tests ---
    println!("\n[Queues]");
    h.run("constitutional cannot be queued", || {
        let mut q = SignalQueues::new();
        q.enqueue(make_signal(SignalTier::Constitution)).is_err()
    });
    h.run("priority order: adversarial > fleet > prediction", || {
        let mut q = SignalQueues::new();
        q.enqueue(make_signal(SignalTier::Prediction)).unwrap();
        q.enqueue(make_signal(SignalTier::Fleet)).unwrap();
        q.enqueue(make_signal(SignalTier::Adversarial)).unwrap();
        let first = q.pop_highest_priority().unwrap();
        let second = q.pop_highest_priority().unwrap();
        let third = q.pop_highest_priority().unwrap();
        first.tier == SignalTier::Adversarial
            && second.tier == SignalTier::Fleet
            && third.tier == SignalTier::Prediction
    });
    h.run("queue full returns error", || {
        let mut q = TierQueue::new(SignalTier::Fleet, 1);
        q.push(make_signal(SignalTier::Fleet)).unwrap();
        q.push(make_signal(SignalTier::Fleet)).is_err()
    });

    // --- Subscription tests ---
    println!("\n[Subscription]");
    h.run("subscribe and lookup", || {
        let mut reg = SubscriptionRegistry::new();
        let id = SubscriberId::from_value("sub-1");
        reg.subscribe("signal.fleet", id, "fleet-handler").unwrap();
        reg.subscribers_for("signal.fleet").len() == 1
    });
    h.run("unsubscribe removes entry", || {
        let mut reg = SubscriptionRegistry::new();
        let id = SubscriberId::from_value("sub-1");
        reg.subscribe("signal.fleet", id.clone(), "fleet-handler")
            .unwrap();
        reg.unsubscribe("signal.fleet", &id);
        reg.subscribers_for("signal.fleet").is_empty()
    });

    // --- Receipt tests ---
    println!("\n[Receipt]");
    h.run("receipt success detection", || {
        let r = DeliveryReceipt::new("sig-1", DeliveryOutcome::Delivered, "h");
        r.is_success() && r.is_hot()
    });
    h.run("receipt log accumulates", || {
        let mut log = DeliveryReceiptLog::new();
        log.record(DeliveryReceipt::new("s1", DeliveryOutcome::Delivered, "h"));
        log.record(DeliveryReceipt::new(
            "s2",
            DeliveryOutcome::Failed {
                reason: "err".into(),
            },
            "h",
        ));
        log.len() == 2 && log.success_count() == 1 && log.failure_count() == 1
    });

    // --- Audit tests ---
    println!("\n[Audit]");
    h.run("audit trail records entries", || {
        let mut trail = SignalAuditTrail::new();
        trail.record(AuditEntry::new("sig-1", AuditAction::GatePassed));
        trail.record(AuditEntry::new(
            "sig-1",
            AuditAction::Routed {
                route: "queue".into(),
            },
        ));
        trail.len() == 2
    });
    h.run("audit trail filters by signal", || {
        let mut trail = SignalAuditTrail::new();
        trail.record(AuditEntry::new("sig-1", AuditAction::GatePassed));
        trail.record(AuditEntry::new("sig-2", AuditAction::GatePassed));
        trail.entries_for("sig-1").len() == 1 && trail.entries_for("sig-2").len() == 1
    });

    // --- Fabric tests ---
    println!("\n[Fabric]");
    h.run("emit valid fleet signal queues it", || {
        let mut f = SignalFabric::new();
        f.emit(make_signal(SignalTier::Fleet)).unwrap();
        f.status().queued == 1
    });
    h.run("emit orphan signal silently rejected", || {
        let mut f = SignalFabric::new();
        f.emit(make_orphan(SignalTier::Fleet)).unwrap();
        f.status().queued == 0 && f.status().receipts > 0
    });
    h.run("constitutional bypasses queue", || {
        let mut f = SignalFabric::new();
        f.emit(make_signal(SignalTier::Constitution)).unwrap();
        f.status().queued == 0 && f.status().receipts > 0
    });
    h.run("dispatch returns correct counts", || {
        let mut f = SignalFabric::new();
        f.subscribe(
            "signal.fleet",
            SubscriberId::from_value("sub-1"),
            "fleet-handler",
        )
        .unwrap();
        f.emit(make_signal(SignalTier::Fleet)).unwrap();
        f.emit(make_signal(SignalTier::Companion)).unwrap();
        let r = f.dispatch();
        r.dispatched == 1 && r.unrouted == 1
    });

    // --- Weight test ---
    println!("\n[Weight]");
    h.run("constitutional signal has high weight", || {
        let s = Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            SignalTier::Constitution,
            0, // trust tier 0 = constitutional authority
        );
        let w = compute_signal_weight(&s);
        w > 0.8
    });

    // --- Topic test ---
    println!("\n[Topic]");
    h.run("signal_topic returns correct strings", || {
        signal_topic(&make_signal(SignalTier::Fleet)) == "signal.fleet"
            && signal_topic(&make_signal(SignalTier::Constitution)) == "signal.constitution"
    });

    // --- End-to-end ---
    println!("\n[End-to-End]");
    h.run(
        "full lifecycle: emit -> subscribe -> dispatch -> receipt",
        || {
            let mut f = SignalFabric::new();
            f.subscribe(
                "signal.fleet",
                SubscriberId::from_value("sub-1"),
                "fleet-handler",
            )
            .unwrap();
            f.subscribe(
                "signal.belief_revision",
                SubscriberId::from_value("sub-2"),
                "belief-handler",
            )
            .unwrap();

            f.emit(make_signal(SignalTier::Fleet)).unwrap();
            f.emit(make_signal(SignalTier::BeliefRevision)).unwrap();
            f.emit(make_signal(SignalTier::Constitution)).unwrap();
            f.emit(make_orphan(SignalTier::Fleet)).unwrap();

            let status_before = f.status();
            let result = f.dispatch();
            let status_after = f.status();

            // 2 queued (fleet + belief_revision), constitution bypassed, orphan rejected
            status_before.queued == 2
                && result.dispatched == 2
                && result.unrouted == 0
                && status_after.queued == 0
                && status_after.audit_entries > 0
        },
    );

    println!();
    h.report();
}
