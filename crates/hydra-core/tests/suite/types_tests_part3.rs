use hydra_core::*;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════
// PROACTIVE UPDATE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_proactive_update_all_6_variants() {
    let updates: Vec<ProactiveUpdate> = vec![
        ProactiveUpdate::Acknowledgment {
            message: "Got it!".into(),
        },
        ProactiveUpdate::Progress {
            percent: 50.0,
            message: "Working...".into(),
            deployment_id: None,
        },
        ProactiveUpdate::Event {
            title: "Sister connected".into(),
            detail: "memory module online".into(),
        },
        ProactiveUpdate::Decision {
            request: DecisionRequest {
                id: Uuid::new_v4(),
                question: "Proceed?".into(),
                options: vec![],
                timeout_seconds: Some(30),
                default: Some(0),
            },
        },
        ProactiveUpdate::Completion {
            summary: CompletionSummary {
                headline: "Done!".into(),
                actions: vec!["Created file".into()],
                changes: vec!["src/main.rs".into()],
                next_steps: vec!["Run tests".into()],
            },
        },
        ProactiveUpdate::Alert {
            level: AlertLevel::Warning,
            message: "Low token budget".into(),
            suggestion: Some("Consider conservation mode".into()),
        },
    ];
    assert_eq!(updates.len(), 6);
}

// ═══════════════════════════════════════════════════════════
// DECISION & COMPLETION TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_decision_option_keyboard_shortcut() {
    let option = DecisionOption {
        label: "Yes".into(),
        description: Some("Approve the action".into()),
        risk_level: Some(RiskLevel::Low),
        keyboard_shortcut: Some("y".into()),
    };
    assert_eq!(option.keyboard_shortcut, Some("y".to_string()));
}

#[test]
fn test_completion_summary() {
    let summary = CompletionSummary {
        headline: "File created successfully".into(),
        actions: vec!["Created src/main.rs".into()],
        changes: vec!["src/main.rs (new)".into()],
        next_steps: vec!["Run cargo build".into(), "Run cargo test".into()],
    };
    assert_eq!(summary.next_steps.len(), 2);
}

// ═══════════════════════════════════════════════════════════
// COGNITIVE STATE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_cognitive_phases() {
    let phases = [
        CognitivePhase::Perceive,
        CognitivePhase::Think,
        CognitivePhase::Decide,
        CognitivePhase::Act,
        CognitivePhase::Learn,
    ];
    assert_eq!(phases.len(), 5);
}

#[test]
fn test_cognitive_state_has_goals_and_beliefs() {
    let state = CognitiveState {
        phase: CognitivePhase::Think,
        intent_id: Some(Uuid::new_v4()),
        context: serde_json::json!({"working_dir": "/tmp"}),
        goals: vec![Goal {
            goal_type: GoalType::Create,
            target: "file".into(),
            outcome: "created".into(),
            sub_goals: vec![],
        }],
        budget: TokenBudget::new(10_000),
        beliefs: vec![Belief {
            key: "language".into(),
            value: serde_json::json!("rust"),
            confidence: 0.95,
            source: "user_preference".into(),
        }],
        checkpoint: None,
    };
    assert_eq!(state.goals.len(), 1);
    assert_eq!(state.beliefs.len(), 1);
    assert_eq!(state.beliefs[0].confidence, 0.95);
}
