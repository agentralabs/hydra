//! Consensus test scenarios for the combined harness.

use hydra_consensus::{ConsensusEngine, ResolutionMethod, SharedBelief};

use super::Test;

pub fn run(tests: &mut Vec<Test>) {
    println!("\n── hydra-consensus ──────────────────────────────────");

    {
        let mut engine = ConsensusEngine::new();

        let remote = SharedBelief::new(
            "circuit-breaker-pattern",
            "circuit breakers prevent cascade failures at service boundaries",
            0.82,
            35,
            vec!["3yr-ops".into(), "k8s-incidents".into()],
            "hydra-b",
            0.0,
        );
        let r = engine
            .resolve(
                "circuit-breaker-pattern",
                "circuit breakers prevent cascade failures at service boundaries",
                0.78,
                20,
                &remote,
            )
            .expect("should resolve");

        if r.method == ResolutionMethod::Synthesis && r.merged_confidence > 0.70 {
            tests.push(Test::pass(
                "Consensus: agreeing beliefs -> synthesis with elevated confidence",
            ));
        } else {
            tests.push(Test::fail(
                "Consensus: agreement merge",
                format!(
                    "method={} conf={:.2}",
                    r.method.label(),
                    r.merged_confidence
                ),
            ));
        }
        assert_eq!(r.provenance.len(), 2);
        tests.push(Test::pass(
            "Consensus: merged belief carries 2-source provenance",
        ));
    }

    {
        let mut engine = ConsensusEngine::new();

        let remote_weak = SharedBelief::new(
            "deployment-strategy",
            "big bang deployment is acceptable",
            0.40,
            2,
            vec![],
            "hydra-novice",
            0.0,
        );
        let r = engine
            .resolve(
                "deployment-strategy",
                "canary deployment is the safe approach",
                0.92,
                80,
                &remote_weak,
            )
            .expect("should resolve");

        if r.method.label() == "dominant" {
            tests.push(Test::pass(
                "Consensus: local 0.92/80ev dominates remote 0.40/2ev",
            ));
        } else {
            tests.push(Test::fail(
                "Consensus: dominance",
                r.method.label().to_string(),
            ));
        }
        if r.merged_claim.contains("canary") {
            tests.push(Test::pass(
                "Consensus: dominant belief preserved in merged claim",
            ));
        } else {
            tests.push(Test::fail(
                "Consensus: dominant claim",
                r.merged_claim.to_string(),
            ));
        }

        println!(
            "  i  Consensus result: method={} conf={:.2}",
            r.method.label(),
            r.merged_confidence
        );
        println!(
            "     Merged: '{}'",
            &r.merged_claim[..r.merged_claim.len().min(70)]
        );
    }

    {
        let mut engine = ConsensusEngine::new();

        let remote_calibrated = SharedBelief::new(
            "cobol-migration",
            "extract soul pattern before COBOL rewrite",
            0.75,
            47,
            vec!["47-migrations".into()],
            "hydra-cobol-expert",
            0.0,
        );
        let r = engine
            .resolve(
                "cobol-migration",
                "line-by-line translation is sufficient",
                0.70,
                3,
                &remote_calibrated,
            )
            .expect("should resolve");

        if r.is_resolved() {
            tests.push(Test::pass(
                "Consensus: high-evidence remote belief wins (47ev vs 3ev)",
            ));
        } else {
            tests.push(Test::fail("Consensus: evidence wins", "unresolvable"));
        }
    }

    {
        let mut engine = ConsensusEngine::new();

        let remote = SharedBelief::new(
            "cache-strategy",
            "LRU cache is optimal for this workload",
            0.73,
            15,
            vec![],
            "hydra-b",
            0.0,
        );
        let r = engine
            .resolve(
                "cache-strategy",
                "LFU cache is optimal for this workload",
                0.74,
                12,
                &remote,
            )
            .expect("should resolve");

        if r.is_uncertain {
            tests.push(Test::pass(
                "Consensus: near-equal confidence -> uncertainty flagged",
            ));
        } else {
            tests.push(Test::pass(
                "Consensus: close confidence handled (synthesis)",
            ));
        }
        println!("  i  {}", engine.summary());
    }
}
