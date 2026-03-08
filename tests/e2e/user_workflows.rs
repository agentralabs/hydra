//! Category 3: E2E — user workflow simulations.

use hydra_runtime::*;
use hydra_core::*;

#[test]
fn test_simple_question_flow() {
    // Simulate: user sends message → intent compiled → response
    let intent = Intent::new("What is the capital of France?", IntentSource::Cli);
    assert!(!intent.text.is_empty());

    // Simulate compilation
    let compiled = CompiledIntent {
        id: intent.id,
        raw_text: intent.text.clone(),
        source: IntentSource::Cli,
        goal: Goal { goal_type: GoalType::Query, target: "knowledge".into(), outcome: "answer".into(), sub_goals: vec![] },
        entities: vec![],
        actions: vec![],
        constraints: vec![],
        success_criteria: vec![],
        confidence: 0.95,
        estimated_steps: 1,
        tokens_used: 50,
        veritas_validation: None,
    };
    assert!(compiled.is_high_confidence());
    assert!(!compiled.is_multi_step());
}

#[test]
fn test_code_generation_flow() {
    let intent = Intent::new("Create a Python function to sort a list", IntentSource::Cli);

    let compiled = CompiledIntent {
        id: intent.id,
        raw_text: intent.text,
        source: IntentSource::Cli,
        goal: Goal { goal_type: GoalType::Create, target: "function".into(), outcome: "sort function".into(), sub_goals: vec![] },
        entities: vec![
            Entity { id: uuid::Uuid::new_v4(), entity_type: EntityType::Other("language".into()), value: "Python".into(), resolved_path: None, confidence: 0.9 },
        ],
        actions: vec![
            Action::new(ActionType::FileCreate, "sort.py"),
        ],
        constraints: vec![],
        success_criteria: vec![],
        confidence: 0.85,
        estimated_steps: 3,
        tokens_used: 200,
        veritas_validation: None,
    };
    assert!(compiled.is_multi_step());
    assert!(!compiled.has_destructive_actions());
}

#[test]
fn test_multi_turn_context() {
    // Simulate maintaining context across turns
    let mut state = hydra_native::state::hydra::HydraState::with_defaults();
    state.add_user_message("What is Rust?");
    state.add_hydra_message("Rust is a systems programming language.", None);
    state.add_user_message("How does it compare to C++?");
    state.add_hydra_message("Rust has memory safety guarantees that C++ lacks.", None);

    assert_eq!(state.messages.len(), 4);
    assert_eq!(state.messages[0].role, hydra_native::state::hydra::MessageRole::User);
    assert_eq!(state.messages[1].role, hydra_native::state::hydra::MessageRole::Hydra);
}

#[test]
fn test_run_lifecycle_events() {
    let mut state = hydra_native::state::hydra::HydraState::with_defaults();
    state.handle_run_started("run-1", "test intent");
    assert_eq!(state.runs.len(), 1);

    state.handle_step_started("run-1", "perceive");
    state.handle_step_completed("run-1", "perceive", 100, 50);
    state.handle_step_started("run-1", "think");
    state.handle_step_completed("run-1", "think", 300, 100);
    state.handle_run_completed("run-1", "Task completed successfully");

    assert_eq!(state.runs[0].status, hydra_native::state::hydra::RunStatus::Completed);
}
