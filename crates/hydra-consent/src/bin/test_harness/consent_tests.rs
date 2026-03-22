//! Consent test scenarios for the combined harness.

use hydra_consent::{ConsentEngine, ConsentError, ConsentScope};

use super::Test;

pub fn run(tests: &mut Vec<Test>) {
    println!("\n── hydra-consent ────────────────────────────────────");

    {
        let mut engine = ConsentEngine::new();

        engine.grant(
            "hydra-b",
            ConsentScope::GenomeSharing {
                domain: "engineering".into(),
                max_entries: 50,
            },
            Some(100),
            30,
        );

        let ok = engine.check("hydra-b", "genome:engineering");
        if ok.is_ok() {
            tests.push(Test::pass(
                "Consent: genome:engineering permitted within grant",
            ));
        } else {
            tests.push(Test::fail("Consent: check", "denied when should permit"));
        }

        let denied = engine.check("hydra-b", "wisdom:engineering");
        if denied.is_err() {
            tests.push(Test::pass(
                "Consent: wisdom:engineering denied (not in grant scope)",
            ));
        } else {
            tests.push(Test::fail(
                "Consent: scope boundary",
                "permitted when should deny",
            ));
        }
    }

    {
        let mut engine = ConsentEngine::new();
        engine.grant("hydra-b", ConsentScope::PatternParticipation, None, 30);

        engine
            .record_sharing(
                "hydra-b",
                "pattern",
                "Contributed cascade failure pattern observation. \
             Provenance: hydra-agentra-main.",
                "receipt-sha256-abc123",
            )
            .expect("should record");

        if engine.audit_count() == 1 {
            tests.push(Test::pass("Consent: sharing event recorded in audit log"));
        } else {
            tests.push(Test::fail(
                "Consent: audit count",
                format!("{}", engine.audit_count()),
            ));
        }
    }

    {
        let engine = ConsentEngine::new();
        let r = engine.check("unknown-peer", "genome:engineering");
        if let Err(ConsentError::NoConsent { .. }) = r {
            tests.push(Test::pass(
                "Consent: no consent grant -> NoConsent error (hard stop)",
            ));
        } else {
            tests.push(Test::fail("Consent: no consent error", "wrong error type"));
        }
    }

    {
        let mut engine = ConsentEngine::new();
        engine.grant(
            "hydra-b",
            ConsentScope::WisdomSharing {
                domain: "fintech".into(),
            },
            None,
            30,
        );
        assert!(engine.check("hydra-b", "wisdom:fintech").is_ok());

        engine.revoke_peer("hydra-b", "trust score dropped below threshold");

        let r = engine.check("hydra-b", "wisdom:fintech");
        if r.is_err() {
            tests.push(Test::pass("Consent: revocation takes effect immediately"));
        } else {
            tests.push(Test::fail(
                "Consent: revocation",
                "still permitted after revoke",
            ));
        }
        assert_eq!(engine.active_grant_count(), 0);
        tests.push(Test::pass(
            "Consent: active grant count = 0 after full revocation",
        ));
    }

    {
        let mut engine = ConsentEngine::new();
        engine.grant(
            "hydra-b",
            ConsentScope::GenomeSharing {
                domain: "fintech".into(),
                max_entries: 20,
            },
            Some(3),
            30,
        );
        for _ in 0..3 {
            engine
                .record_sharing("hydra-b", "genome:fintech", "share", "receipt")
                .expect("should record");
        }
        let r = engine.record_sharing("hydra-b", "genome:fintech", "share", "receipt");
        if r.is_err() {
            tests.push(Test::pass("Consent: grant exhausted after 3 uses"));
        } else {
            tests.push(Test::fail(
                "Consent: exhaustion",
                "still permitted after max",
            ));
        }
    }
}
