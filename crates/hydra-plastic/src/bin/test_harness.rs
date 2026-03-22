//! Combined test harness for the Growth Layer (Phase 12).
//!
//! Tests all 5 crates: genome, cartography, antifragile, generative, plastic.
//! Demonstrates the growth function: all metrics increase from one operation.

use hydra_antifragile::{AntifragileStore, ObstacleClass};
use hydra_cartography::{CartographyAtlas, SystemClass, SystemProfile};
use hydra_generative::{GenerativeEngine, SynthesisOutcome};
use hydra_genome::{ApproachSignature, GenomeStore};
use hydra_plastic::{EnvironmentProfile, ExecutionMode, PlasticityTensor};

fn main() {
    println!("=== Phase 12: Growth Layer Combined Harness ===\n");

    let pass = test_genome()
        && test_cartography()
        && test_antifragile()
        && test_generative()
        && test_plastic()
        && test_growth_integration();

    println!();
    if pass {
        println!("ALL GROWTH LAYER TESTS PASSED");
    } else {
        println!("SOME TESTS FAILED");
        std::process::exit(1);
    }
}

fn test_genome() -> bool {
    println!("[genome] Testing capability genetics...");
    let mut store = GenomeStore::new();
    let approach = ApproachSignature::new(
        "api_call",
        vec!["authenticate".into(), "send_request".into()],
        vec!["curl".into()],
    );
    let id = match store.add_from_operation("deploy rest api service", approach, 0.8) {
        Ok(id) => id,
        Err(e) => {
            println!("  FAIL: add_from_operation: {}", e);
            return false;
        }
    };
    if store.total_ever() != 1 {
        println!("  FAIL: total_ever should be 1");
        return false;
    }
    if let Err(e) = store.record_use(&id, true) {
        println!("  FAIL: record_use: {}", e);
        return false;
    }
    let results = store.query("deploy rest api service");
    if results.is_empty() {
        println!("  FAIL: query returned no results");
        return false;
    }
    println!(
        "  PASS: genome store works (total_ever={})",
        store.total_ever()
    );
    true
}

fn test_cartography() -> bool {
    println!("[cartography] Testing digital topology...");
    let mut atlas = CartographyAtlas::new();
    let mut p1 = SystemProfile::new("stripe-api", SystemClass::RestApi);
    p1.add_hint("json");
    p1.add_hint("oauth2");
    p1.add_approach("use-bearer-token");
    if let Err(e) = atlas.add(p1) {
        println!("  FAIL: add profile: {}", e);
        return false;
    }
    let mut p2 = SystemProfile::new("github-api", SystemClass::RestApi);
    p2.add_hint("json");
    if let Err(e) = atlas.add(p2) {
        println!("  FAIL: add profile: {}", e);
        return false;
    }
    match atlas.transfer_knowledge("github-api") {
        Ok(count) => println!("  Transferred {} approaches", count),
        Err(e) => {
            println!("  FAIL: transfer_knowledge: {}", e);
            return false;
        }
    }
    println!(
        "  PASS: atlas works (total_ever={}, classes=RestApi:{})",
        atlas.total_ever(),
        atlas.by_class(&SystemClass::RestApi).len()
    );
    true
}

fn test_antifragile() -> bool {
    println!("[antifragile] Testing obstacle resistance...");
    let mut store = AntifragileStore::new();
    if let Err(e) = store.record_encounter(&ObstacleClass::RateLimit, true, Some("backoff")) {
        println!("  FAIL: record_encounter: {}", e);
        return false;
    }
    if let Err(e) = store.record_encounter(&ObstacleClass::RateLimit, true, Some("backoff")) {
        println!("  FAIL: record_encounter: {}", e);
        return false;
    }
    let resistance = store.resistance(&ObstacleClass::RateLimit);
    if resistance <= 0.0 || resistance > 1.0 {
        println!("  FAIL: resistance out of bounds: {}", resistance);
        return false;
    }
    println!(
        "  PASS: antifragile works (resistance={:.3}, encounters={})",
        resistance,
        store.total_encounters()
    );
    true
}

fn test_generative() -> bool {
    println!("[generative] Testing capability synthesis...");
    let engine = GenerativeEngine::new();
    let mut store = GenomeStore::new();
    let result = engine.synthesize_for(
        "optimize resource allocation under time constraints",
        &mut store,
    );
    match result {
        Ok(SynthesisOutcome::Success {
            capability_name,
            confidence,
        }) => {
            println!(
                "  Synthesized: {} (confidence={:.3})",
                capability_name, confidence
            );
        }
        Ok(other) => {
            println!("  FAIL: expected Success, got {:?}", other);
            return false;
        }
        Err(e) => {
            println!("  FAIL: synthesize_for: {}", e);
            return false;
        }
    }
    // Second call should find existing.
    let result2 = engine.synthesize_for(
        "optimize resource allocation under time constraints",
        &mut store,
    );
    match result2 {
        Ok(SynthesisOutcome::ExistingApproach { .. }) => {}
        Ok(other) => {
            println!("  FAIL: expected ExistingApproach, got {:?}", other);
            return false;
        }
        Err(e) => {
            println!("  FAIL: synthesize_for (2nd): {}", e);
            return false;
        }
    }
    println!(
        "  PASS: generative engine works (genome total={})",
        store.total_ever()
    );
    true
}

fn test_plastic() -> bool {
    println!("[plastic] Testing environment adaptation...");
    let mut tensor = PlasticityTensor::new();
    if let Err(e) = tensor.add(EnvironmentProfile::new(
        "local",
        ExecutionMode::NativeBinary,
    )) {
        println!("  FAIL: add: {}", e);
        return false;
    }
    if let Some(env) = tensor.get_mut("local") {
        env.record_encounter(true);
        env.record_encounter(true);
    }
    let env = tensor.get("local").expect("local should exist");
    if env.confidence <= 0.5 {
        println!("  FAIL: confidence should have increased");
        return false;
    }
    println!(
        "  PASS: plasticity works (confidence={:.3}, total_ever={})",
        env.confidence,
        tensor.total_ever()
    );
    true
}

fn test_growth_integration() -> bool {
    println!("[integration] Testing growth function (all metrics increase)...");

    // Snapshot initial state.
    let mut genome = GenomeStore::new();
    let mut atlas = CartographyAtlas::new();
    let mut antifragile = AntifragileStore::new();
    let mut tensor = PlasticityTensor::new();
    let engine = GenerativeEngine::new();

    let g0 = genome.total_ever();
    let a0 = atlas.total_ever();
    let af0 = antifragile.total_encounters();
    let t0 = tensor.total_ever();

    // Simulate one operation touching all stores.
    // 1. Encounter a new system.
    let mut profile = SystemProfile::new("new-api", SystemClass::RestApi);
    profile.add_hint("json");
    let _ = atlas.add(profile);

    // 2. Synthesize a capability.
    let _ = engine.synthesize_for("deploy and optimize rest api service", &mut genome);

    // 3. Encounter an obstacle.
    let _ = antifragile.record_encounter(&ObstacleClass::RateLimit, true, Some("retry"));

    // 4. Record an environment.
    let _ = tensor.add(EnvironmentProfile::new(
        "cloud",
        ExecutionMode::ContainerExec,
    ));

    // Verify all metrics increased.
    let all_grew = genome.total_ever() > g0
        && atlas.total_ever() > a0
        && antifragile.total_encounters() > af0
        && tensor.total_ever() > t0;

    if !all_grew {
        println!("  FAIL: not all metrics increased");
        println!("    genome: {} -> {}", g0, genome.total_ever());
        println!("    atlas: {} -> {}", a0, atlas.total_ever());
        println!(
            "    antifragile: {} -> {}",
            af0,
            antifragile.total_encounters()
        );
        println!("    tensor: {} -> {}", t0, tensor.total_ever());
        return false;
    }

    println!("  PASS: all growth metrics increased from one operation");
    println!("    genome:      {} -> {}", g0, genome.total_ever());
    println!("    atlas:       {} -> {}", a0, atlas.total_ever());
    println!(
        "    antifragile: {} -> {}",
        af0,
        antifragile.total_encounters()
    );
    println!("    tensor:      {} -> {}", t0, tensor.total_ever());
    true
}
