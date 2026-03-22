//! Phase 37 Test Harness — THE FINAL LAYER 5 CRATE
//! Run: cargo run -p hydra-exchange --bin test_harness

use hydra_exchange::{
    ExchangeEngine, ExchangeError, ExchangeOffer, ExchangeOutcome, ExchangeRequest, OfferKind,
};

struct Test {
    name: &'static str,
    passed: bool,
    notes: Option<String>,
}
impl Test {
    fn pass(name: &'static str) -> Self {
        Self {
            name,
            passed: true,
            notes: None,
        }
    }
    fn fail(name: &'static str, n: impl Into<String>) -> Self {
        Self {
            name,
            passed: false,
            notes: Some(n.into()),
        }
    }
}

fn main() {
    println!("===================================================");
    println!("  Phase 37 — THE FINAL LAYER 5 CRATE");
    println!("  hydra-exchange — Capability Exchange");
    println!("  Offer. Request. Execute. Receipt.");
    println!("===================================================");

    let mut tests = Vec::new();
    let mut engine = ExchangeEngine::new();

    // -- OFFERS --
    println!("\n-- offers -----------------------------------------------");

    let _settle_id = engine.register_offer(ExchangeOffer::new(
        OfferKind::SettlementExecution {
            skill_name: "agentra-settlement".into(),
        },
        "Execute Agentra settlements on behalf of authorized agents. \
         Requires trust score >= 0.70. Constitutional compliance guaranteed.",
        0.70,
        15.0,
        None,
    ));

    engine.register_offer(ExchangeOffer::new(
        OfferKind::RedTeamAnalysis,
        "Pre-execution adversarial analysis. \
         Requires trust score >= 0.65.",
        0.65,
        8.0,
        None,
    ));

    engine.register_offer(ExchangeOffer::new(
        OfferKind::WisdomJudgment {
            domain: "fintech".into(),
        },
        "Calibrated wisdom judgment for fintech domain decisions.",
        0.75,
        12.0,
        Some(50),
    ));

    if engine.offer_count() == 3 {
        tests.push(Test::pass(
            "Offers: 3 capabilities registered (settlement, redteam, wisdom)",
        ));
    } else {
        tests.push(Test::fail(
            "Offers: count",
            format!("{}", engine.offer_count()),
        ));
    }

    if engine.registry.available_offers().len() == 3 {
        tests.push(Test::pass("Offers: all 3 available in registry"));
    } else {
        tests.push(Test::fail(
            "Offers: available",
            format!("{}", engine.registry.available_offers().len()),
        ));
    }

    // -- TRUST-GATED PROCESSING --
    println!("\n-- trust-gated processing -------------------------------");

    {
        let request = ExchangeRequest::new(
            "agentra-agent-alpha",
            OfferKind::RedTeamAnalysis,
            "analyze deployment plan for staging environment",
            0.80,
        );
        let result = engine.process_request(request);
        match result {
            Ok(ref r) if r.outcome == "fulfilled" && r.receipt_id.is_some() => {
                tests.push(Test::pass(
                    "Exchange: trusted agent -> request fulfilled with receipt",
                ));
            }
            Ok(ref r) => {
                tests.push(Test::fail(
                    "Exchange: fulfillment",
                    format!("outcome={}", r.outcome),
                ));
            }
            Err(ref e) => {
                tests.push(Test::fail("Exchange: fulfillment", format!("{}", e)));
            }
        }

        if engine.receipt_count() == 1 {
            tests.push(Test::pass(
                "Exchange: receipt written to registry (immutable)",
            ));
        } else {
            tests.push(Test::fail(
                "Exchange: receipt count",
                format!("{}", engine.receipt_count()),
            ));
        }
    }

    {
        let request = ExchangeRequest::new(
            "unknown-external-system",
            OfferKind::RedTeamAnalysis,
            "analyze our production system",
            0.35,
        );
        let result = engine.process_request(request);
        if let Err(ExchangeError::InsufficientTrust { .. }) = result {
            tests.push(Test::pass(
                "Exchange: low-trust agent -> InsufficientTrust rejection",
            ));
        } else {
            tests.push(Test::fail("Exchange: trust gate", "not rejected"));
        }
    }

    {
        let request = ExchangeRequest::new(
            "agentra-settlement-client",
            OfferKind::SettlementExecution {
                skill_name: "agentra-settlement".into(),
            },
            "execute Q1 settlement batch for Agentra platform",
            0.82,
        );
        match engine.process_request(request) {
            Ok(ref result) if result.outcome == "fulfilled" => {
                tests.push(Test::pass(
                    "Exchange: settlement execution fulfilled for authorized client",
                ));
                println!(
                    "  i  Settlement exchange: value={:.1}, receipt={}",
                    result.value,
                    result.receipt_id.as_deref().unwrap_or("none"),
                );
            }
            Ok(ref result) => {
                tests.push(Test::fail(
                    "Exchange: settlement",
                    result.outcome.to_string(),
                ));
            }
            Err(ref e) => {
                tests.push(Test::fail("Exchange: settlement", format!("{}", e)));
            }
        }
    }

    {
        let request = ExchangeRequest::new(
            "agent",
            OfferKind::GenomeSharing {
                domain: "cobol".into(),
                max_entries: 100,
            },
            "share COBOL migration knowledge",
            0.80,
        );
        let result = engine.process_request(request);
        if let Err(ExchangeError::CapabilityUnavailable { .. }) = result {
            tests.push(Test::pass(
                "Exchange: unlisted capability -> CapabilityUnavailable error",
            ));
        } else {
            tests.push(Test::fail("Exchange: unavailable", "wrong error"));
        }
    }

    // -- RECEIPTS --
    println!("\n-- receipts ---------------------------------------------");

    {
        let receipts = engine
            .registry
            .receipts_for_counterparty("agentra-settlement-client");
        if receipts.len() == 1 && receipts[0].verify_integrity() {
            tests.push(Test::pass(
                "Receipts: settlement exchange receipt — SHA256 integrity verified",
            ));
        } else {
            tests.push(Test::fail(
                "Receipts: integrity",
                format!("count={}", receipts.len()),
            ));
        }

        if engine.successful_exchange_count() == 2 {
            tests.push(Test::pass("Receipts: 2 successful exchanges recorded"));
        } else {
            tests.push(Test::fail(
                "Receipts: success count",
                format!("{}", engine.successful_exchange_count()),
            ));
        }
    }

    // -- OFFER EXHAUSTION --
    println!("\n-- offer exhaustion -------------------------------------");

    {
        let mut limited_engine = ExchangeEngine::new();
        limited_engine.register_offer(ExchangeOffer::new(
            OfferKind::ArtifactSharing {
                artifact_kind: "playbook".into(),
            },
            "Share settlement playbook — limited to 2 exchanges",
            0.60,
            5.0,
            Some(2),
        ));

        for _ in 0..2 {
            let req = ExchangeRequest::new(
                "agent",
                OfferKind::ArtifactSharing {
                    artifact_kind: "playbook".into(),
                },
                "context",
                0.70,
            );
            if let Err(e) = limited_engine.process_request(req) {
                tests.push(Test::fail("Exhaustion: pre-exhaust", format!("{}", e)));
            }
        }

        let req = ExchangeRequest::new(
            "agent",
            OfferKind::ArtifactSharing {
                artifact_kind: "playbook".into(),
            },
            "context",
            0.70,
        );
        let result = limited_engine.process_request(req);
        if result.is_err() {
            tests.push(Test::pass(
                "Exhaustion: limited offer exhausted after 2 uses",
            ));
        } else {
            tests.push(Test::fail(
                "Exhaustion: limit enforcement",
                "no error after max",
            ));
        }
    }

    // -- OUTGOING REQUEST --
    {
        let outgoing = engine.request_capability(
            "hydra-instance-b",
            OfferKind::GenomeSharing {
                domain: "cobol-migration".into(),
                max_entries: 50,
            },
            "Need COBOL migration genome entries for upcoming enterprise project",
            0.78,
        );
        if !outgoing.id.is_empty() && (outgoing.trust_score - 0.78).abs() < f64::EPSILON {
            tests.push(Test::pass(
                "Outgoing: request to external Hydra instance created",
            ));
        } else {
            tests.push(Test::fail("Outgoing: request creation", "missing fields"));
        }
    }

    // -- ERROR requires_human --
    {
        let trust_err = ExchangeError::InsufficientTrust {
            counterparty: "x".into(),
            score: 0.3,
            min: 0.6,
        };
        let escalation_err = ExchangeError::EscalationRequired {
            value: 1000.0,
            max: 500.0,
        };
        let offer_err = ExchangeError::OfferNotFound {
            offer_id: "x".into(),
        };
        if trust_err.requires_human()
            && escalation_err.requires_human()
            && !offer_err.requires_human()
        {
            tests.push(Test::pass(
                "Errors: requires_human() correct for trust, escalation, and offer-not-found",
            ));
        } else {
            tests.push(Test::fail("Errors: requires_human", "classification wrong"));
        }
    }

    // -- OUTCOME LABELS --
    {
        let ful = ExchangeOutcome::Fulfilled {
            description: "done".into(),
        };
        let rej = ExchangeOutcome::Rejected {
            reason: "no".into(),
        };
        let par = ExchangeOutcome::Partial {
            description: "half".into(),
            fraction_delivered: 0.5,
        };
        if ful.is_successful()
            && !rej.is_successful()
            && par.is_successful()
            && ful.label() == "fulfilled"
            && rej.label() == "rejected"
            && par.label() == "partial"
        {
            tests.push(Test::pass(
                "Outcomes: labels and is_successful correct for all three variants",
            ));
        } else {
            tests.push(Test::fail("Outcomes: classification", "mismatch"));
        }
    }

    // -- SUMMARY --
    {
        let s = engine.summary();
        if s.contains("exchange:") && s.contains("offers=") && s.contains("receipts=") {
            tests.push(Test::pass(
                "Summary: format correct for TUI and intelligence brief",
            ));
        } else {
            tests.push(Test::fail("Summary", s));
        }
    }

    // -- RESULTS --
    println!();
    let total = tests.len();
    let passed = tests.iter().filter(|t| t.passed).count();
    let failed = total - passed;

    for t in &tests {
        if t.passed {
            println!("  PASS  {}", t.name);
        } else {
            println!("  FAIL  {}", t.name);
            if let Some(n) = &t.notes {
                println!("           {}", n);
            }
        }
    }

    println!();
    println!("===================================================");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        println!("===================================================");
        std::process::exit(1);
    } else {
        println!();
        println!("  LAYER 5 — COMPLETE");
        println!("  5 crates: settlement, attribution, portfolio, crystallizer, exchange");
        println!("  Trust-gated: 0.35 rejected, 0.80 ok. Receipts: SHA256 immutable.");
        println!("  Phase 37 complete. Layer 5 is closed. Layer 6 begins.");
        println!("===================================================");
    }
}
