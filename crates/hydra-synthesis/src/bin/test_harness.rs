//! Test harness for hydra-synthesis — validates pattern discovery.

use hydra_axiom::AxiomPrimitive;
use hydra_comprehension::*;
use hydra_reasoning::conclusion::{ReasoningConclusion, ReasoningMode};
use hydra_reasoning::ReasoningResult;
use hydra_synthesis::pattern::StructuralPattern;
use hydra_synthesis::{find_cross_domain_matches, SynthesisEngine};

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

    println!("=== hydra-synthesis test harness ===\n");

    // 1. Structural pattern similarity
    println!("[1] Structural pattern similarity");
    let a = StructuralPattern::from_primitives(
        "engineering",
        &[AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
        "eng pattern",
    );
    let b = StructuralPattern::from_primitives(
        "finance",
        &[AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
        "fin pattern",
    );
    check!(
        "same primitives = 1.0",
        (a.similarity(&b) - 1.0).abs() < f64::EPSILON
    );

    // 2. Cross-domain match found
    println!("\n[2] Cross-domain match");
    let patterns = vec![a, b];
    let matches = find_cross_domain_matches(&patterns);
    check!("one match found", matches.len() == 1);
    check!(
        "match similarity is 1.0",
        (matches[0].similarity - 1.0).abs() < f64::EPSILON
    );

    // 3. Pattern ingested
    println!("\n[3] Pattern ingestion");
    let mut engine = SynthesisEngine::new();
    let input = make_input(
        Domain::Engineering,
        vec![AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
    );
    let result = make_result();
    let ingest_result = engine.ingest(&input, &result);
    check!("ingest succeeds", ingest_result.is_ok());
    check!("library size is 1", engine.library_size() == 1);

    // 4. Cross-domain insight
    println!("\n[4] Cross-domain insight");
    let input2 = make_input(
        Domain::Finance,
        vec![AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
    );
    let _ = engine.ingest(&input2, &result);
    let insights = engine.synthesize(&input2, &result);
    check!("synthesize succeeds", insights.is_ok());
    if let Ok(ref ins) = insights {
        check!("at least one insight", !ins.is_empty());
        if let Some(first) = ins.first() {
            check!("insight has narrative", !first.narrative.is_empty());
            check!("insight has transfer hint", !first.transfer_hint.is_empty());
        }
    }

    // 5. Summary format
    println!("\n[5] Summary format");
    let summary = engine.summary();
    check!("contains 'synthesis:'", summary.contains("synthesis:"));
    check!("contains 'library='", summary.contains("library="));
    check!("contains 'domains='", summary.contains("domains="));

    println!(
        "\n=== Synthesis Results: {} passed, {} failed ===",
        passed, failed
    );
    if failed > 0 {
        std::process::exit(1);
    }
}

fn make_input(domain: Domain, primitives: Vec<AxiomPrimitive>) -> ComprehendedInput {
    ComprehendedInput {
        raw: "test synthesis input text here".into(),
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

fn make_result() -> ReasoningResult {
    let c = ReasoningConclusion::new(
        ReasoningMode::Deductive,
        "test conclusion",
        0.8,
        vec![],
        false,
    );
    ReasoningResult {
        conclusions: vec![c.clone()],
        synthesis_confidence: 0.8,
        used_llm: false,
        active_modes: 1,
        primary: Some(c),
        mode_summary: vec![("deductive".to_string(), true)],
    }
}
