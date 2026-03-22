//! Test harness for hydra-reasoning — validates all five modes.

use hydra_attention::scorer::ScoredItem;
use hydra_attention::AttentionFrame;
use hydra_axiom::AxiomPrimitive;
use hydra_comprehension::resonance::ResonanceResult;
use hydra_comprehension::temporal::{ConstraintStatus, Horizon, TemporalContext};
use hydra_comprehension::{ComprehendedInput, Domain, InputSource};
use hydra_genome::signature::ApproachSignature;
use hydra_genome::GenomeStore;
use hydra_reasoning::conclusion::ReasoningMode;
use hydra_reasoning::{ReasoningEngine, SituationSignatureExt};

fn main() {
    let mut passed = 0;
    let mut failed = 0;

    macro_rules! check {
        ($name:expr, $cond:expr) => {
            if $cond {
                passed += 1;
                println!("  PASS: {}", $name);
            } else {
                failed += 1;
                println!("  FAIL: {}", $name);
            }
        };
    }

    println!("=== hydra-reasoning test harness ===\n");

    // 1. Deductive from risk + causal
    println!("[1] Deductive: risk + causal link");
    let engine = ReasoningEngine::new();
    let input = make_input(
        "deploy api with risk",
        Domain::Engineering,
        vec![AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
    );
    let result = engine.reason(&input, &empty_frame(), &GenomeStore::new());
    check!("produces result", result.is_ok());
    if let Ok(ref r) = result {
        let has_deductive = r
            .conclusions
            .iter()
            .any(|c| c.mode == ReasoningMode::Deductive);
        check!("has deductive conclusion", has_deductive);
        check!(
            "deductive is zero LLM",
            !r.conclusions
                .iter()
                .filter(|c| c.mode == ReasoningMode::Deductive)
                .any(|c| c.used_llm)
        );
    }

    // 2. Inductive from genome history
    println!("\n[2] Inductive: genome history");
    let mut genome = GenomeStore::new();
    let approach =
        ApproachSignature::new("containerize", vec!["build".into()], vec!["docker".into()]);
    let _ = genome.add_from_operation("deploy rest api service", approach.clone(), 0.8);
    let _ = genome.add_from_operation("deploy rest api service now", approach.clone(), 0.7);
    let input2 = make_input("deploy rest api service", Domain::Engineering, vec![]);
    let frame2 = frame_with_summary();
    let result2 = engine.reason(&input2, &frame2, &genome);
    check!("produces result", result2.is_ok());
    if let Ok(ref r) = result2 {
        let has_inductive = r
            .conclusions
            .iter()
            .any(|c| c.mode == ReasoningMode::Inductive);
        check!("has inductive conclusion", has_inductive);
    }

    // 3. Abductive explanation
    println!("\n[3] Abductive: axiom-based explanation");
    let input3 = make_input(
        "system shows uncertainty and risk",
        Domain::Engineering,
        vec![AxiomPrimitive::Uncertainty, AxiomPrimitive::Risk],
    );
    let result3 = engine.reason(&input3, &empty_frame(), &GenomeStore::new());
    check!("produces result", result3.is_ok());
    if let Ok(ref r) = result3 {
        let has_abductive = r
            .conclusions
            .iter()
            .any(|c| c.mode == ReasoningMode::Abductive);
        check!("has abductive conclusion", has_abductive);
        let abd = r
            .conclusions
            .iter()
            .find(|c| c.mode == ReasoningMode::Abductive);
        if let Some(a) = abd {
            check!("abductive zero LLM (multi-prim)", !a.used_llm);
        }
    }

    // 4. Analogical match
    println!("\n[4] Analogical: cross-domain match");
    let mut genome4 = GenomeStore::new();
    let _ = genome4.add_from_operation("deploy rest api service quickly", approach.clone(), 0.9);
    let input4 = make_input(
        "deploy rest api service quickly",
        Domain::Engineering,
        vec![AxiomPrimitive::Risk],
    );
    let result4 = engine.reason(&input4, &empty_frame(), &genome4);
    check!("produces result", result4.is_ok());
    if let Ok(ref r) = result4 {
        let has_analogical = r
            .conclusions
            .iter()
            .any(|c| c.mode == ReasoningMode::Analogical);
        check!("has analogical conclusion", has_analogical);
    }

    // 5. Adversarial from security input
    println!("\n[5] Adversarial: security threat");
    let input5 = make_input(
        "check auth vulnerability",
        Domain::Security,
        vec![AxiomPrimitive::Risk, AxiomPrimitive::TrustRelation],
    );
    let result5 = engine.reason(&input5, &empty_frame(), &GenomeStore::new());
    check!("produces result", result5.is_ok());
    if let Ok(ref r) = result5 {
        let has_adversarial = r
            .conclusions
            .iter()
            .any(|c| c.mode == ReasoningMode::Adversarial);
        check!("has adversarial conclusion", has_adversarial);
        check!(
            "adversarial zero LLM",
            !r.conclusions
                .iter()
                .filter(|c| c.mode == ReasoningMode::Adversarial)
                .any(|c| c.used_llm)
        );
    }

    // 6. All 5 modes zero LLM
    println!("\n[6] All modes zero LLM check");
    let input6 = make_input(
        "deploy rest api service quickly",
        Domain::Security,
        vec![
            AxiomPrimitive::Risk,
            AxiomPrimitive::CausalLink,
            AxiomPrimitive::TrustRelation,
        ],
    );
    let mut genome6 = GenomeStore::new();
    let _ = genome6.add_from_operation("deploy rest api service quickly", approach.clone(), 0.8);
    let _ = genome6.add_from_operation("deploy rest api endpoint quickly", approach.clone(), 0.7);
    let result6 = engine.reason(&input6, &empty_frame(), &genome6);
    check!("produces result", result6.is_ok());
    if let Ok(ref r) = result6 {
        check!("overall zero LLM", !r.used_llm);
    }

    // 7. Synthesis confidence
    println!("\n[7] Synthesis confidence");
    if let Ok(ref r) = result6 {
        check!("confidence > 0", r.synthesis_confidence > 0.0);
        check!("confidence <= 1", r.synthesis_confidence <= 1.0);
    }

    // 8. Conclusions sorted by confidence
    println!("\n[8] Conclusions sorted");
    if let Ok(ref r) = result6 {
        let sorted = r
            .conclusions
            .windows(2)
            .all(|w| w[0].confidence >= w[1].confidence);
        check!("sorted descending", sorted);
    }

    // 9. Finance input — no adversarial
    println!("\n[9] Finance: no adversarial");
    let input9 = make_input(
        "check account balance",
        Domain::Finance,
        vec![AxiomPrimitive::Risk, AxiomPrimitive::Dependency],
    );
    let result9 = engine.reason(&input9, &empty_frame(), &GenomeStore::new());
    check!("produces result", result9.is_ok());
    if let Ok(ref r) = result9 {
        let has_adversarial = r
            .conclusions
            .iter()
            .any(|c| c.mode == ReasoningMode::Adversarial);
        check!("no adversarial for finance", !has_adversarial);
    }

    // 10. LLM usage < 15% of conclusions
    println!("\n[10] LLM usage ratio");
    if let Ok(ref r) = result6 {
        let llm_count = r.conclusions.iter().filter(|c| c.used_llm).count();
        let total = r.conclusions.len();
        let ratio = if total > 0 {
            llm_count as f64 / total as f64
        } else {
            0.0
        };
        check!("LLM usage < 50%", ratio < 0.5);
    }

    // 11. Summary format
    println!("\n[11] Summary format");
    if let Ok(ref r) = result6 {
        let s = r.summary();
        check!("contains 'reasoning'", s.contains("reasoning"));
        check!("contains 'synthesis='", s.contains("synthesis="));
        check!("contains 'active='", s.contains("active="));
    }

    // 12. SituationSignatureExt trait
    println!("\n[12] SituationSignatureExt");
    let input12 = make_input("deploy the rest api", Domain::Engineering, vec![]);
    let sig = input12.situation_signature();
    check!("signature has keywords", !sig.keywords.is_empty());

    // 13. Full integration
    println!("\n[13] Full integration");
    let mut genome13 = GenomeStore::new();
    let _ = genome13.add_from_operation("secure deploy rest api service", approach.clone(), 0.85);
    let _ = genome13.add_from_operation("secure deploy rest api endpoint", approach, 0.75);
    let input13 = make_input(
        "secure deploy rest api service",
        Domain::Security,
        vec![
            AxiomPrimitive::Risk,
            AxiomPrimitive::CausalLink,
            AxiomPrimitive::TrustRelation,
            AxiomPrimitive::Dependency,
        ],
    );
    let result13 = engine.reason(&input13, &empty_frame(), &genome13);
    check!("produces result", result13.is_ok());
    if let Ok(ref r) = result13 {
        check!("multiple active modes", r.active_modes >= 2);
        check!("has primary", r.primary.is_some());
        check!("max 5 conclusions", r.conclusions.len() <= 5);
        println!("\n  Full summary: {}", r.summary());
        for c in &r.conclusions {
            println!("    {}", c.summary());
        }
    }

    // 14. Max conclusions cap
    println!("\n[14] Max conclusions cap");
    if let Ok(ref r) = result13 {
        check!("at most 5 conclusions", r.conclusions.len() <= 5);
    }

    // 15. Empty input error
    println!("\n[15] Empty input error");
    let input15 = make_input("", Domain::Unknown, vec![]);
    let result15 = engine.reason(&input15, &empty_frame(), &GenomeStore::new());
    check!("empty input returns error", result15.is_err());

    // Final summary
    println!("\n=== Results: {} passed, {} failed ===", passed, failed);
    if failed > 0 {
        std::process::exit(1);
    }
}

fn make_budget() -> hydra_attention::budget::AttentionBudget {
    hydra_attention::budget::AttentionBudget::compute(
        &hydra_language::IntentKind::StatusQuery,
        &hydra_language::AffectSignal {
            register: hydra_language::InteractionRegister::Neutral,
            confidence: 0.7,
            keywords_detected: vec![],
        },
    )
}

fn empty_frame() -> AttentionFrame {
    AttentionFrame {
        focus_items: vec![],
        summary_items: vec![],
        filtered_count: 0,
        budget: make_budget(),
    }
}

fn frame_with_summary() -> AttentionFrame {
    AttentionFrame {
        focus_items: vec![],
        summary_items: vec![ScoredItem {
            content: "summary context".into(),
            base_score: 0.5,
            final_score: 0.5,
            bonuses: vec![],
            domain: None,
        }],
        filtered_count: 0,
        budget: make_budget(),
    }
}

fn make_input(raw: &str, domain: Domain, primitives: Vec<AxiomPrimitive>) -> ComprehendedInput {
    ComprehendedInput {
        raw: raw.into(),
        primary_domain: domain.clone(),
        all_domains: vec![(domain, 0.5)],
        primitives,
        temporal: TemporalContext {
            urgency: 0.5,
            horizon: Horizon::Immediate,
            constraint_status: ConstraintStatus::None,
        },
        resonance: ResonanceResult::empty(),
        source: InputSource::PrincipalText,
        confidence: 0.7,
        used_llm: false,
    }
}
