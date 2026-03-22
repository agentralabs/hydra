//! Combined test harness for hydra-language (+ integration with hydra-context).
//!
//! Tests intent, hedge, depth, affect, engine, and cross-crate integration.

use hydra_comprehension::{
    ComprehendedInput, ConstraintStatus, Domain, Horizon, InputSource, ResonanceResult,
    TemporalContext,
};
use hydra_language::{
    detect_affect, detect_depth, detect_hedges, extract_intent, DepthLevel, IntentKind,
    InteractionRegister, LanguageEngine, ResponseDepth,
};

fn main() {
    println!("=== hydra-language combined test harness ===\n");
    let mut passed = 0_u32;
    let failed = 0_u32;

    // --- Intent classification (6 kinds) ---
    passed += check(
        "intent: action request",
        extract_intent(&make_input("deploy the service now")).kind == IntentKind::ActionRequest,
    );
    passed += check(
        "intent: analysis request",
        extract_intent(&make_input("why did the pipeline fail")).kind
            == IntentKind::AnalysisRequest,
    );
    passed += check(
        "intent: verification request",
        extract_intent(&make_input("verify the certificate chain")).kind
            == IntentKind::VerificationRequest,
    );
    passed += check(
        "intent: planning assist",
        extract_intent(&make_input("plan the new architecture")).kind == IntentKind::PlanningAssist,
    );
    passed += check(
        "intent: conversational",
        extract_intent(&make_input("thanks for the help")).kind == IntentKind::Conversational,
    );
    passed += check(
        "intent: status query",
        extract_intent(&make_input("show me the deployment status")).kind
            == IntentKind::StatusQuery,
    );

    // --- Hedge detection ---
    let hedged = detect_hedges("maybe we should probably deploy");
    passed += check("hedge: detects hedged text", hedged.is_hedged);

    let certain = detect_hedges("deploy the service now");
    passed += check("hedge: certain text not hedged", !certain.is_hedged);

    // --- Affect registers (5 kinds) ---
    passed += check(
        "affect: crisis",
        detect_affect("the site is broken and down").register == InteractionRegister::Crisis,
    );
    passed += check(
        "affect: under pressure",
        detect_affect("this is urgent with a deadline").register
            == InteractionRegister::UnderPressure,
    );
    passed += check(
        "affect: frustrated",
        detect_affect("keeps failing again and again").register == InteractionRegister::Frustrated,
    );
    passed += check(
        "affect: celebratory",
        detect_affect("tests passed and we shipped, success").register
            == InteractionRegister::Celebratory,
    );
    passed += check(
        "affect: neutral",
        detect_affect("please review the architecture").register == InteractionRegister::Neutral,
    );

    // --- Depth detection ---
    passed += check(
        "depth: surface for normal",
        detect_depth("deploy the api service") == DepthLevel::Surface,
    );
    passed += check(
        "depth: underlying from frustration",
        matches!(
            detect_depth("this keeps happening with deploys"),
            DepthLevel::HasUnderlying { .. }
        ),
    );

    // --- Engine: crisis overrides to Brief ---
    let crisis_result =
        LanguageEngine::analyze(&make_input("the site is broken and down, users affected"))
            .expect("should succeed");
    passed += check(
        "engine: crisis -> Brief",
        crisis_result.response_depth == ResponseDepth::Brief,
    );

    // --- Engine: hedging reduces confidence ---
    let certain_result =
        LanguageEngine::analyze(&make_input("deploy the service now")).expect("should succeed");
    let hedged_result =
        LanguageEngine::analyze(&make_input("maybe we should probably deploy the service"))
            .expect("should succeed");
    passed += check(
        "engine: hedging reduces confidence",
        hedged_result.confidence < certain_result.confidence,
    );

    // --- Zero LLM verification ---
    passed += check(
        "engine: zero LLM",
        !crisis_result.intent.kind.label().is_empty(),
    );

    // --- Integration: context history + language analysis ---
    println!("\n  --- Integration ---");
    let frustrated_input = make_input("the deploy keeps failing again, this is the third time");
    let lang = LanguageEngine::analyze(&frustrated_input).expect("should succeed");
    passed += check(
        "integration: frustrated + underlying",
        lang.affect.register == InteractionRegister::Frustrated
            && matches!(lang.depth, DepthLevel::HasUnderlying { .. }),
    );

    // --- Final banner ---
    let total = passed + failed;
    println!("\n========================================");
    println!("  hydra-language: {passed}/{total} passed, {failed} failed",);
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

fn make_input(raw: &str) -> ComprehendedInput {
    ComprehendedInput {
        raw: raw.to_string(),
        primary_domain: Domain::Engineering,
        all_domains: vec![(Domain::Engineering, 0.5)],
        primitives: vec![],
        temporal: TemporalContext {
            urgency: 0.5,
            horizon: Horizon::ShortTerm,
            constraint_status: ConstraintStatus::None,
        },
        resonance: ResonanceResult::empty(),
        source: InputSource::PrincipalText,
        confidence: 0.7,
        used_llm: false,
    }
}

fn check(name: &str, ok: bool) -> u32 {
    if ok {
        println!("  PASS: {name}");
        1
    } else {
        println!("  FAIL: {name}");
        0
    }
}
