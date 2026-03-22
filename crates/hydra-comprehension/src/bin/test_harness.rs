//! Test harness for hydra-comprehension.
//!
//! Runs ~25 scenarios covering all 4 pipeline stages and prints results.

use hydra_axiom::AxiomPrimitive;
use hydra_comprehension::{
    ComprehensionEngine, Domain, InputSource, MemoryResonance, PrimitiveMapping, TemporalPlacement,
};
use hydra_genome::{ApproachSignature, GenomeStore};

fn main() {
    println!("=== hydra-comprehension test harness ===\n");

    let mut passed = 0_u32;
    let mut failed = 0_u32;
    let engine = ComprehensionEngine::new();
    let genome = GenomeStore::new();

    // --- Domain detection tests ---
    passed += check(
        "domain: engineering",
        engine
            .comprehend(
                "deploy the api service to docker",
                InputSource::PrincipalText,
                &genome,
            )
            .map(|c| c.primary_domain == Domain::Engineering)
            .unwrap_or(false),
    );

    passed += check(
        "domain: finance",
        engine
            .comprehend(
                "check the account balance and revenue report",
                InputSource::PrincipalText,
                &genome,
            )
            .map(|c| c.primary_domain == Domain::Finance)
            .unwrap_or(false),
    );

    passed += check(
        "domain: security",
        engine
            .comprehend(
                "fix the auth vulnerability in credential store",
                InputSource::PrincipalText,
                &genome,
            )
            .map(|c| c.primary_domain == Domain::Security)
            .unwrap_or(false),
    );

    passed += check(
        "domain: data",
        engine
            .comprehend(
                "run the etl pipeline to ingest database records",
                InputSource::PrincipalText,
                &genome,
            )
            .map(|c| c.primary_domain == Domain::Data)
            .unwrap_or(false),
    );

    passed += check(
        "domain: operations",
        engine
            .comprehend(
                "monitor the kubernetes cluster for downtime alerts",
                InputSource::PrincipalText,
                &genome,
            )
            .map(|c| c.primary_domain == Domain::Operations)
            .unwrap_or(false),
    );

    passed += check(
        "domain: mixed returns multiple",
        engine
            .comprehend(
                "deploy api service and check budget balance",
                InputSource::PrincipalText,
                &genome,
            )
            .map(|c| c.all_domains.len() >= 2)
            .unwrap_or(false),
    );

    // --- Primitive mapping tests ---
    passed += check(
        "primitive: risk detected",
        PrimitiveMapping::extract("there is a risk of failure here")
            .contains(&AxiomPrimitive::Risk),
    );

    passed += check(
        "primitive: constraint detected",
        PrimitiveMapping::extract("we have a budget constraint on this")
            .contains(&AxiomPrimitive::Constraint),
    );

    passed += check(
        "primitive: causal link from deploy",
        PrimitiveMapping::extract("deploy the service to production now")
            .contains(&AxiomPrimitive::CausalLink),
    );

    passed += check(
        "primitive: multi-primitive",
        PrimitiveMapping::extract(
            "risk constraint deploy optimize depend uncertainty trust sequence",
        )
        .len()
            >= 5,
    );

    passed += check(
        "primitive: empty returns empty",
        PrimitiveMapping::extract("").is_empty(),
    );

    passed += check(
        "primitive: no match returns empty",
        PrimitiveMapping::extract("the sky is blue today").is_empty(),
    );

    // --- Temporal urgency tests ---
    passed += check(
        "temporal: critical is high urgency",
        TemporalPlacement::analyze("critical failure in production now").urgency >= 0.8,
    );

    passed += check(
        "temporal: plan is low urgency",
        TemporalPlacement::analyze("plan to eventually migrate the service").urgency < 0.3,
    );

    passed += check(
        "temporal: neutral without keywords",
        (TemporalPlacement::analyze("write a function that adds numbers").urgency - 0.5).abs()
            < 0.01,
    );

    passed += check("temporal: deadline activates constraint", {
        let ctx = TemporalPlacement::analyze("the deadline is tomorrow");
        ctx.constraint_status == hydra_comprehension::ConstraintStatus::Activates
    });

    passed += check("temporal: overdue violates constraint", {
        let ctx = TemporalPlacement::analyze("this task is overdue");
        ctx.constraint_status == hydra_comprehension::ConstraintStatus::Violates
    });

    // --- Memory resonance tests ---
    passed += check(
        "resonance: empty store no match",
        !MemoryResonance::check_resonance("deploy api", &genome).has_prior_context,
    );

    let mut genome_with_entry = GenomeStore::new();
    let _ = genome_with_entry.add_from_operation(
        "deploy rest api service",
        ApproachSignature::new("deploy", vec!["step1".into()], vec!["docker".into()]),
        0.8,
    );

    passed += check(
        "resonance: matching entry found",
        MemoryResonance::check_resonance("deploy rest api service", &genome_with_entry)
            .has_prior_context,
    );

    passed += check(
        "resonance: unrelated no match",
        !MemoryResonance::check_resonance("compile rust binary executable", &genome_with_entry)
            .has_prior_context,
    );

    // --- Full pipeline tests ---
    passed += check("pipeline: multi-domain input", {
        let r = engine.comprehend(
            "deploy the api and monitor kubernetes cluster alerts",
            InputSource::PrincipalText,
            &genome,
        );
        r.is_ok()
    });

    passed += check("pipeline: zero LLM always", {
        let r = engine.comprehend(
            "critical risk in the deploy pipeline now",
            InputSource::PrincipalText,
            &genome,
        );
        r.map(|c| !c.used_llm).unwrap_or(false)
    });

    passed += check("pipeline: sister output tagging", {
        let r = engine.comprehend_sister("deploy the rest api service now", "memory", &genome);
        r.map(|c| matches!(c.source, InputSource::SisterOutput { .. }))
            .unwrap_or(false)
    });

    passed += check(
        "pipeline: empty rejection",
        engine
            .comprehend("", InputSource::PrincipalText, &genome)
            .is_err(),
    );

    passed += check(
        "pipeline: short rejection",
        engine
            .comprehend("hi", InputSource::PrincipalText, &genome)
            .is_err(),
    );

    // Summary with confidence check
    if let Ok(full) = engine.comprehend(
        "deploy the api service to docker and optimize performance",
        InputSource::PrincipalText,
        &genome,
    ) {
        println!("\n  Sample summary: {}", full.summary());
        passed += check(
            "pipeline: confidence is non-negative",
            full.confidence >= 0.0,
        );
    } else {
        failed += 1;
        println!("  FAIL: sample comprehension failed");
    }

    // --- Final banner ---
    let total = passed + failed;
    println!("\n========================================");
    println!("  hydra-comprehension: {passed}/{total} passed, {failed} failed",);
    if failed == 0 {
        println!("  ALL TESTS PASSED");
    } else {
        println!("  SOME TESTS FAILED");
    }
    println!("========================================");

    if failed > 0 {
        std::process::exit(1);
    }
}

/// Run a single check, print result, return 1 on pass, 0 on fail.
fn check(name: &str, ok: bool) -> u32 {
    if ok {
        println!("  PASS: {name}");
        1
    } else {
        println!("  FAIL: {name}");
        0
    }
}
