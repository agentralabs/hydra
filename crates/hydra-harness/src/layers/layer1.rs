//! Tests all Layer 1 capabilities.
//! constitution, genome, trust, belief, temporal, soul,
//! morphic, axiom, antifragile, skills, signals

use crate::TestResult;
use std::time::Instant;

pub fn run() -> Vec<TestResult> {
    let mut results = Vec::new();

    results.push(test_constitution());
    results.extend(test_genome());
    results.extend(test_trust());
    results.extend(test_belief());
    results.extend(test_temporal());
    results.extend(test_soul());
    results.extend(test_morphic());
    results.extend(test_axiom());
    results.extend(test_antifragile());
    results.extend(test_skills());
    results.extend(test_signals());

    let passed = results.iter().filter(|r| r.passed).count();
    println!("  Layer 1: {}/{} passed", passed, results.len());
    results
}

fn test_constitution() -> TestResult {
    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let checker = hydra_constitution::ConstitutionChecker::new();
        assert!(checker.law_count() >= 7, "Must have 7+ constitutional laws");
        checker.law_count()
    }) {
        Ok(count) => {
            println!("  [PASS] hydra-constitution: {} laws verified", count);
            TestResult::pass("hydra-constitution", "all_laws_present",
                start.elapsed().as_millis() as u64)
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-constitution: {}", err);
            TestResult::fail("hydra-constitution", "all_laws_present",
                &err, start.elapsed().as_millis() as u64)
        }
    }
}

fn test_genome() -> Vec<TestResult> {
    let mut results = Vec::new();

    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        let mut store = hydra_genome::GenomeStore::new();
        let entry = hydra_genome::GenomeEntry::from_operation(
            "circuit breaker at service boundaries",
            hydra_genome::ApproachSignature::new(
                "install circuit breaker", vec![], vec![],
            ),
            0.88,
        );
        let _ = store.add(entry);
        assert!(store.len() == 1, "Store should have 1 entry");
        assert!(!store.is_empty());
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-genome: add and query");
            results.push(TestResult::pass("hydra-genome", "add_and_query",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-genome: {}", err);
            results.push(TestResult::fail("hydra-genome", "add_and_query",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_trust() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        let field = hydra_trust::TrustField::new();
        // Verify we can create a trust field and inspect it
        assert!(field.agent_count() == 0, "New field has no agents");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-trust: trust field creation");
            results.push(TestResult::pass("hydra-trust", "trust_field",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-trust: {}", err);
            results.push(TestResult::fail("hydra-trust", "trust_field",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_belief() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        let mut store = hydra_belief::BeliefStore::new();
        let belief = hydra_belief::Belief::world(
            "circuit_breakers_prevent_cascades", 0.88,
        );
        let belief_id = belief.id.clone();
        store.insert(belief).expect("Insert must succeed");
        let got = store.get(&belief_id);
        assert!(got.is_some(), "Belief must be retrievable");
        let conf = got.map(|b| b.confidence).unwrap_or(0.0);
        assert!((conf - 0.88).abs() < 0.01, "Confidence must match");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-belief: store and retrieve");
            results.push(TestResult::pass("hydra-belief", "store_retrieve",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-belief: {}", err);
            results.push(TestResult::fail("hydra-belief", "store_retrieve",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_temporal() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        let t1 = hydra_temporal::Timestamp::now();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let t2 = hydra_temporal::Timestamp::now();
        assert!(t2.as_nanos() > t1.as_nanos(), "Time must advance");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-temporal: causal time");
            results.push(TestResult::pass("hydra-temporal", "causal_time",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-temporal: {}", err);
            results.push(TestResult::fail("hydra-temporal", "causal_time",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_soul() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_soul::{Soul, NodeKind};
        let mut soul = Soul::new();
        for i in 0..10 {
            soul.record_exchange(
                &format!("exchange-{}", i),
                NodeKind::RecurringChoice,
            ).expect("record_exchange must succeed");
        }
        assert!(soul.graph().node_count() == 10, "Graph must have 10 nodes");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-soul: accumulation");
            results.push(TestResult::pass("hydra-soul", "accumulation",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-soul: {}", err);
            results.push(TestResult::fail("hydra-soul", "accumulation",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_morphic() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        let identity = hydra_morphic::MorphicIdentity::genesis();
        assert!(identity.depth() == 0, "Genesis depth must be 0");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-morphic: identity genesis");
            results.push(TestResult::pass("hydra-morphic", "identity_genesis",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-morphic: {}", err);
            results.push(TestResult::fail("hydra-morphic", "identity_genesis",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_axiom() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_axiom::{AxiomPrimitive, AxiomMorphism, synthesize};
        let result = synthesize(
            "test-capability",
            vec![AxiomPrimitive::Risk, AxiomPrimitive::Constraint],
            vec![(0, 1, AxiomMorphism::Constrains)],
        );
        assert!(result.is_ok(), "Axiom synthesis must succeed");
        let cap = result.expect("just asserted");
        assert!(cap.confidence > 0.0, "Must have non-zero confidence");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-axiom: synthesis");
            results.push(TestResult::pass("hydra-axiom", "synthesis",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-axiom: {}", err);
            results.push(TestResult::fail("hydra-axiom", "synthesis",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_antifragile() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        use hydra_antifragile::{AntifragileStore, ObstacleClass};
        let mut store = AntifragileStore::new();
        let before = store.resistance(&ObstacleClass::AuthChallenge);
        store.record_encounter(&ObstacleClass::AuthChallenge, true, None)
            .expect("record_encounter must succeed");
        let after = store.resistance(&ObstacleClass::AuthChallenge);
        assert!(after >= before, "Resistance must not decrease on success");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-antifragile: obstacle resistance");
            results.push(TestResult::pass("hydra-antifragile", "obstacle_resistance",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-antifragile: {}", err);
            results.push(TestResult::fail("hydra-antifragile", "obstacle_resistance",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_skills() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        let skills_dir = std::path::PathBuf::from("skills");
        assert!(skills_dir.exists(), "skills/ directory must exist");
        assert!(skills_dir.join("general").exists(), "general skill must be present");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-skills: general skill present");
            results.push(TestResult::pass("hydra-skills", "skill_present",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-skills: {}", err);
            results.push(TestResult::fail("hydra-skills", "skill_present",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_signals() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        let fabric = hydra_signals::SignalFabric::new();
        let status = fabric.status();
        assert!(status.queued == 0, "New fabric has 0 queued signals");
    }) {
        Ok(()) => {
            println!("  [PASS] hydra-signals: signal fabric");
            results.push(TestResult::pass("hydra-signals", "signal_fabric",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] hydra-signals: {}", err);
            results.push(TestResult::fail("hydra-signals", "signal_fabric",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}
