//! Combined test harness for Phase 7: hydra-axiom, hydra-belief, hydra-prediction.
//!
//! Runs ~25 scenarios covering all three crates end-to-end.

use hydra_axiom::{
    synthesize, AxiomMorphism, AxiomPrimitive, DeploymentFunctor, FinanceFunctor, FunctorRegistry,
};
use hydra_belief::{
    revise, verify_inclusion, verify_success, Belief, BeliefCategory, BeliefPosition, BeliefStore,
    RevisionPolicy,
};
use hydra_prediction::{
    compute_divergence, ActualOutcome, DivergenceDetector, IntentPrediction, PredictionBasis,
    PredictionStage, PredictionStager, ShadowOutcome,
};

fn main() {
    let mut passed = 0;
    let mut failed = 0;

    macro_rules! scenario {
        ($name:expr, $body:expr) => {{
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
                Ok(()) => {
                    println!("  PASS: {}", $name);
                    passed += 1;
                }
                Err(_) => {
                    println!("  FAIL: {}", $name);
                    failed += 1;
                }
            }
        }};
    }

    println!("=== Phase 7 Combined Test Harness ===\n");

    // --- AXIOM SCENARIOS ---
    println!("[hydra-axiom]");

    scenario!("primitive similarity — same variant", {
        let a = AxiomPrimitive::Risk;
        assert!((a.similarity(&a) - 0.9).abs() < f64::EPSILON);
    });

    scenario!("primitive similarity — related variants", {
        let a = AxiomPrimitive::Risk;
        let b = AxiomPrimitive::Uncertainty;
        assert!((a.similarity(&b) - 0.8).abs() < f64::EPSILON);
    });

    scenario!("primitive similarity — cross-close", {
        let a = AxiomPrimitive::Risk;
        let b = AxiomPrimitive::AdversarialModel;
        assert!((a.similarity(&b) - 0.5).abs() < f64::EPSILON);
    });

    scenario!("primitive similarity — unrelated", {
        let a = AxiomPrimitive::Optimization;
        let b = AxiomPrimitive::TrustRelation;
        assert!((a.similarity(&b)).abs() < f64::EPSILON);
    });

    scenario!("morphism compositionality", {
        assert!(AxiomMorphism::Causes.is_compositional());
        assert!(!AxiomMorphism::CoordinatesWith.is_compositional());
    });

    scenario!("functor registration", {
        let mut reg = FunctorRegistry::new();
        reg.register(Box::new(FinanceFunctor)).unwrap();
        assert_eq!(reg.domain_count(), 1);
    });

    scenario!("functor duplicate rejected", {
        let mut reg = FunctorRegistry::new();
        reg.register(Box::new(FinanceFunctor)).unwrap();
        assert!(reg.register(Box::new(FinanceFunctor)).is_err());
    });

    scenario!("functor concept mapping", {
        let mut reg = FunctorRegistry::new();
        reg.register(Box::new(FinanceFunctor)).unwrap();
        let m = reg.map_concept("finance", "volatility").unwrap();
        assert_eq!(m.axiom_primitive, AxiomPrimitive::Uncertainty);
    });

    scenario!("cross-domain pattern detection", {
        let mut reg = FunctorRegistry::new();
        reg.register(Box::new(FinanceFunctor)).unwrap();
        reg.register(Box::new(DeploymentFunctor)).unwrap();
        let patterns = reg.find_cross_domain_patterns();
        assert!(!patterns.is_empty());
    });

    scenario!("synthesis — empty rejected", {
        assert!(synthesize("empty", vec![], vec![]).is_err());
    });

    scenario!("synthesis — single component", {
        let cap = synthesize("monitor", vec![AxiomPrimitive::Risk], vec![]).unwrap();
        assert!(cap.confidence >= 0.3);
    });

    scenario!("synthesis — multi component", {
        let cap = synthesize(
            "risk-optimizer",
            vec![AxiomPrimitive::Risk, AxiomPrimitive::Optimization],
            vec![(0, 1, AxiomMorphism::OptimizesFor)],
        )
        .unwrap();
        assert_eq!(cap.components.len(), 2);
    });

    // --- BELIEF SCENARIOS ---
    println!("\n[hydra-belief]");

    scenario!("protected belief cannot decrease", {
        let mut b = Belief::capability("coding", 0.8);
        b.apply_delta(-0.5);
        assert!((b.confidence - 0.8).abs() < f64::EPSILON);
    });

    scenario!("protected belief can increase", {
        let mut b = Belief::capability("coding", 0.8);
        b.apply_delta(0.1);
        assert!((b.confidence - 0.9).abs() < f64::EPSILON);
    });

    scenario!("standard belief decreases", {
        let mut b = Belief::world("rain", 0.7);
        b.apply_delta(-0.3);
        assert!((b.confidence - 0.4).abs() < f64::EPSILON);
    });

    scenario!("immutable belief unchanged", {
        let mut b = Belief::new(
            "axiom",
            1.0,
            BeliefCategory::World,
            RevisionPolicy::Immutable,
        );
        b.apply_delta(-0.5);
        assert!((b.confidence - 1.0).abs() < f64::EPSILON);
    });

    scenario!("belief revision installs belief", {
        let mut store = BeliefStore::new();
        let b = Belief::world("sky is blue", 0.9);
        let result = revise(&mut store, b).unwrap();
        verify_success(&store, &result.belief_id).unwrap();
    });

    scenario!("belief revision resolves contradictions", {
        let mut store = BeliefStore::new();
        store
            .insert(Belief::world("deployment risk is high", 0.8))
            .unwrap();
        let result = revise(&mut store, Belief::world("deployment risk is low", 0.9)).unwrap();
        assert!(result.contradictions_resolved >= 1);
    });

    scenario!("capability survives revision", {
        let mut store = BeliefStore::new();
        let cap = Belief::capability("coding skill high", 0.9);
        let cap_id = cap.id.clone();
        store.insert(cap).unwrap();
        revise(&mut store, Belief::world("coding skill low", 0.2)).unwrap();
        assert!(store.get(&cap_id).unwrap().confidence >= 0.9 - f64::EPSILON);
    });

    scenario!("geodesic step moves toward target", {
        let a = BeliefPosition::new(vec![0.0, 0.0]);
        let b = BeliefPosition::new(vec![10.0, 10.0]);
        let stepped = a.geodesic_step(&b);
        assert!(stepped.distance(&b) < a.distance(&b));
    });

    scenario!("AGM inclusion postulate", {
        let mut store = BeliefStore::new();
        let b1 = Belief::world("fact one", 0.5);
        let id1 = b1.id.clone();
        store.insert(b1).unwrap();
        revise(&mut store, Belief::world("fact two", 0.6)).unwrap();
        verify_inclusion(&[id1], &store).unwrap();
    });

    // --- PREDICTION SCENARIOS ---
    println!("\n[hydra-prediction]");

    scenario!("prediction stage slot limit", {
        let mut stage = PredictionStage::new();
        for i in 0..10 {
            stage.update(IntentPrediction::new(
                format!("p{i}"),
                0.3 + (i as f64) * 0.05,
                PredictionBasis::TemporalPattern,
            ));
        }
        assert_eq!(stage.top().len(), 3);
    });

    scenario!("low confidence rejected", {
        let mut stage = PredictionStage::new();
        stage.update(IntentPrediction::new(
            "low",
            0.1,
            PredictionBasis::SessionPattern,
        ));
        assert!(stage.top().is_empty());
    });

    scenario!("divergence — matching outcomes", {
        let shadow = ShadowOutcome {
            description: "t".into(),
            confidence: 0.8,
            state_changes: vec![("k".into(), "v".into())],
        };
        let actual = ActualOutcome {
            description: "t".into(),
            state_changes: vec![("k".into(), "v".into())],
        };
        let div = compute_divergence(&shadow, &actual);
        assert!(div.score.abs() < f64::EPSILON);
    });

    scenario!("divergence — triggers belief revision", {
        let detector = DivergenceDetector::new();
        let shadow = ShadowOutcome {
            description: "fast".into(),
            confidence: 0.8,
            state_changes: vec![("speed".into(), "fast".into())],
        };
        let actual = ActualOutcome {
            description: "slow".into(),
            state_changes: vec![("speed".into(), "slow".into())],
        };
        let mut store = BeliefStore::new();
        assert!(detector
            .evaluate(&shadow, &actual, &mut store, "speed")
            .is_err());
        assert!(!store.is_empty());
    });

    scenario!("stager records and predicts", {
        let mut stager = PredictionStager::new();
        for _ in 0..5 {
            stager.record_intent("check build", None);
        }
        stager.run_cycle();
        assert!(stager.stage().best().is_some());
    });

    // --- SUMMARY ---
    println!("\n=== Results: {passed} passed, {failed} failed ===");
    if failed > 0 {
        std::process::exit(1);
    }
}
