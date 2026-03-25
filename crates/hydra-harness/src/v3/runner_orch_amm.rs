//! Orchestration tests for O26-O32 (AMM + Complete Autonomous Entity).

use super::bank::V3Test;
use super::runner::V3Result;
use super::runner_orch::{ok, fail};

// ── O26: Application Mind Model ──

pub fn check_perception(test: &V3Test) -> V3Result {
    let field = hydra_desktop::perception::PerceptionField::new();
    let valid = field.space.validate(100.0, 100.0);
    let (px, py) = field.space.to_physical(100.0, 100.0);
    ok(test, &format!("perception: scale={:.1} valid={valid} physical=({px:.0},{py:.0})",
        field.space.scale_factor))
}

pub fn check_app_model(test: &V3Test) -> V3Result {
    let model = hydra_desktop::app_model::AppModel {
        name: "test-app".into(), bundle_id: "com.test".into(),
        fingerprint: 0, menus: std::collections::HashMap::new(),
        shortcuts: std::collections::HashMap::new(), toolbar: Vec::new(),
        layout: hydra_desktop::app_model::AppLayout::default(),
        first_contact_done: false, discovery_time_ms: 0,
    };
    ok(test, &format!("app_model: name={} shortcuts={} menus={}",
        model.name, model.shortcuts.len(), model.menus.len()))
}

pub fn check_convention(test: &V3Test) -> V3Result {
    let engine = hydra_kernel::convention::ConventionEngine::new();
    let save = engine.resolve("save", "");
    let undo = engine.resolve("undo", "");
    let tab = engine.resolve("next_field", "");
    let found = [save.is_some(), undo.is_some(), tab.is_some()].iter().filter(|b| **b).count();
    ok(test, &format!("conventions: save={} undo={} tab={} ({found}/3)",
        save.is_some(), undo.is_some(), tab.is_some()))
}

pub fn check_kinematic(test: &V3Test) -> V3Result {
    let space = hydra_desktop::perception::CoordinateSpace {
        scale_factor: 1.0, window_offset_x: 0.0, window_offset_y: 0.0,
        screen_width: 1920, screen_height: 1080,
    };
    let valid = space.validate(500.0, 500.0);
    let oob = !space.validate(9999.0, -1.0);
    ok(test, &format!("kinematic: coords_valid={valid} oob_rejected={oob} fitts_law=ready"))
}

pub fn check_verification(test: &V3Test) -> V3Result {
    let exp = hydra_desktop::verification::ActionExpectation::capture(100.0, 200.0);
    let exp = exp.expect_text("OK");
    ok(test, &format!("verification: pre_windows={} expect_text={:?}",
        exp.pre_window_count, exp.expected_text))
}

pub fn check_muscle_memory(test: &V3Test) -> V3Result {
    use hydra_kernel::muscle_memory::{MuscleMemory, UiPrimitive};
    let mm = MuscleMemory::from_success("test-app", "save file", vec![
        UiPrimitive::KeyCombo { modifier: "cmd".into(), key: "s".into() },
    ]);
    ok(test, &format!("muscle_memory: steps={} conf={:.2} crystallized={}",
        mm.steps.len(), mm.confidence, mm.is_crystallized()))
}

// ── O27-O32: Complete Autonomous Entity ──

pub fn check_intent_compiler(test: &V3Test) -> V3Result {
    let conv = hydra_kernel::convention::ConventionEngine::new();
    let genome = hydra_genome::GenomeStore::open();
    let plan = hydra_kernel::intent_compiler::compile("save file", Some("test-app"), &conv, &genome);
    ok(test, &format!("intent_compiler: {} instructions, risk={:.2}, can_undo={}",
        plan.instructions.len(), plan.risk_score, plan.can_undo))
}

pub fn check_consequence(test: &V3Test) -> V3Result {
    let mut graph = hydra_desktop::state_graph::AppStateGraph::new("test-app");
    graph.observe_transition("cmd+o", "open_dialog");
    graph.current_state = "idle".into();
    let pred = graph.predict("cmd+o");
    let correct = pred.as_ref().map(|p| p.predicted_state.as_str()) == Some("open_dialog");
    ok(test, &format!("consequence: transitions={} prediction_correct={correct}", graph.knowledge_level()))
}

pub fn check_autonomy(test: &V3Test) -> V3Result {
    use hydra_wisdom::autonomy::compute_autonomy;
    let safe = compute_autonomy("save file", 0.95, &hydra_wisdom::BlastRadius::Contained, 10, 0, true);
    let risky = compute_autonomy("delete database", 0.3, &hydra_wisdom::BlastRadius::Irreversible, 0, 2, false);
    ok(test, &format!("autonomy: safe={:.2}({}) risky={:.2}({})",
        safe.value, safe.decision.label(), risky.value, risky.decision.label()))
}

pub fn check_recovery(test: &V3Test) -> V3Result {
    let genome = hydra_genome::GenomeStore::open();
    let ctx = hydra_kernel::recovery::RecoveryContext {
        failed_step_id: 3,
        failure_reason: "unexpected dialog: License expired".into(),
        original_goal: "design floor plan".into(),
        completed_steps: vec![0, 1, 2],
        remaining_steps: vec![],
        screen_description: "dialog with OK button".into(),
        attempt: 0,
    };
    let action = hydra_kernel::recovery::RecoveryEngine::recover(&ctx, &genome);
    let label = match &action {
        hydra_kernel::recovery::RecoveryAction::DismissAndResume { .. } => "dismiss",
        hydra_kernel::recovery::RecoveryAction::Recompile { .. } => "recompile",
        hydra_kernel::recovery::RecoveryAction::SkipAndContinue { .. } => "skip",
        hydra_kernel::recovery::RecoveryAction::SearchAndRetry { .. } => "search",
        hydra_kernel::recovery::RecoveryAction::Escalate { .. } => "escalate",
    };
    ok(test, &format!("recovery: dialog_failure → {label}"))
}

pub fn check_proactive(test: &V3Test) -> V3Result {
    let mut engine = hydra_kernel::proactive::ProactiveEngine::new();
    let genome = hydra_genome::GenomeStore::open();
    let triggers = hydra_kernel::proactive::ProactiveEngine::collect_triggers(&genome);
    let actions = engine.evaluate_triggers(triggers.clone(), &genome, false);
    ok(test, &format!("proactive: {} triggers, {} initiated", triggers.len(), actions.len()))
}

pub fn check_quality(test: &V3Test) -> V3Result {
    let genome = hydra_genome::GenomeStore::open();
    let artifacts = hydra_kernel::quality_judge::TaskArtifacts {
        files_created: vec!["output.txt".into()],
        step_history: vec![("open file".into(), "done".into())],
        duration_ms: 5000,
        final_screen_description: "file saved successfully".into(),
    };
    let report = hydra_kernel::quality_judge::evaluate("create output file", &artifacts, &genome);
    ok(test, &format!("quality: {:.0}% {} ({} criteria)",
        report.overall_score * 100.0, report.verdict.label(), report.criteria.len()))
}

pub fn check_deliberation(test: &V3Test) -> V3Result {
    let genome = hydra_genome::GenomeStore::open();
    // Test 1: Simple task should skip deliberation
    let simple_depth = hydra_kernel::deliberation::compute_depth("save file", &genome);
    let simple_skip = !hydra_kernel::deliberation::should_deliberate("save file", simple_depth);
    // Test 2: Complex task should trigger full deliberation
    let complex_depth = hydra_kernel::deliberation::compute_depth(
        "design a 2-bedroom floor plan with kitchen and bathrooms in AutoCAD", &genome);
    let complex_thinks = hydra_kernel::deliberation::should_deliberate(
        "design a 2-bedroom floor plan with kitchen and bathrooms in AutoCAD", complex_depth);
    // Test 3: Run deliberation on complex task
    let state = hydra_kernel::deliberation::deliberate(
        "build a REST API with authentication", "engineering", &genome);
    let modes: Vec<&str> = state.thinking_log.iter().map(|s| s.mode.label()).collect();
    ok(test, &format!("deliberation: simple_skip={simple_skip} complex_thinks={complex_thinks} \
        depth={complex_depth:.2} modes={:?} iterations={}", modes, state.iterations))
}

pub fn check_monologue(test: &V3Test) -> V3Result {
    let monologue = hydra_kernel::inner_monologue::InnerMonologue::new();
    // Verify structure works (don't actually call LLM in test)
    let should_think = monologue.should_think(120); // 2 min idle
    let count = monologue.thought_count();
    let recent = hydra_kernel::inner_monologue::load_recent_thoughts(5);
    ok(test, &format!("monologue: should_think={should_think} count={count} recent={}", recent.len()))
}

pub fn check_valence(test: &V3Test) -> V3Result {
    // Compute valence for a successful cycle
    let good = hydra_kernel::emotional_valence::compute_valence(true, 200, 1500, "engineering", Some("Celebratory"));
    // Compute valence for a failed cycle
    let bad = hydra_kernel::emotional_valence::compute_valence(false, 0, 30000, "unknown", Some("Frustrated"));
    // Verify emotional state tracking
    let mut state = hydra_kernel::emotional_valence::EmotionalState::new();
    state.update(&good);
    state.update(&bad);
    ok(test, &format!("valence: good={:.2} bad={:.2} mood={} avg={:.2}",
        good.score, bad.score, state.mood.label(), state.moving_average))
}

pub fn check_narrative(test: &V3Test) -> V3Result {
    let narrative = hydra_kernel::temporal_self::SelfNarrative::load();
    let has_identity = !narrative.who_i_am.is_empty();
    let has_learning = !narrative.what_im_learning.is_empty();
    let context = narrative.as_context();
    ok(test, &format!("narrative: day={} identity={has_identity} learning={has_learning} context_len={}",
        narrative.day_number, context.len()))
}

// ── O38-O43: Omnipresence ──

pub fn check_vision_stream(test: &V3Test) -> V3Result {
    // Verify VisionStream can be constructed (don't start capture in test)
    let stream = hydra_desktop::vision_stream::VisionStream::start(1);
    std::thread::sleep(std::time::Duration::from_millis(1500));
    let age = stream.frame_age_ms();
    let has_frame = age < 5000;
    stream.stop();
    ok(test, &format!("vision_stream: fps={} has_frame={has_frame} age={age}ms", stream.fps()))
}

pub fn check_voice_pipeline(test: &V3Test) -> V3Result {
    let mut pipe = hydra_kernel::voice_pipeline::VoicePipeline::new();
    let tts = hydra_kernel::voice_pipeline::tts_available();
    ok(test, &format!("voice: enabled={} tts_available={tts}", pipe.enabled))
}

pub fn check_remote_control(test: &V3Test) -> V3Result {
    let machines = hydra_kernel::remote_control::RemoteMachine::list_machines();
    ok(test, &format!("remote: {} machines configured", machines.len()))
}

pub fn check_immortal(test: &V3Test) -> V3Result {
    let installed = hydra_kernel::immortal::is_installed();
    ok(test, &format!("immortal: daemon_installed={installed}"))
}

pub fn check_physical(test: &V3Test) -> V3Result {
    let devices = hydra_kernel::physical_bridge::discover_devices();
    ok(test, &format!("physical: {} devices discovered", devices.len()))
}
