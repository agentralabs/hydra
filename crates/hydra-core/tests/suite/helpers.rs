use hydra_core::*;
use uuid::Uuid;

pub fn make_compiled_intent(
    confidence: f64,
    steps: usize,
    action_types: Vec<ActionType>,
) -> CompiledIntent {
    let actions = action_types
        .into_iter()
        .map(|at| Action::new(at, "test_target"))
        .collect();
    CompiledIntent {
        id: Uuid::new_v4(),
        raw_text: "test intent".into(),
        source: IntentSource::Cli,
        goal: Goal {
            goal_type: GoalType::Create,
            target: "test".into(),
            outcome: "test outcome".into(),
            sub_goals: vec![],
        },
        entities: vec![],
        actions,
        constraints: vec![],
        success_criteria: vec![],
        confidence,
        estimated_steps: steps,
        tokens_used: 0,
        veritas_validation: VeritasValidation {
            validated: true,
            safety_score: 1.0,
            warnings: vec![],
        },
    }
}

pub fn make_receipt(seq: u64, prev_hash: Option<String>) -> Receipt {
    Receipt {
        id: Uuid::new_v4(),
        deployment_id: Uuid::new_v4(),
        receipt_type: ReceiptType::DeploymentComplete,
        timestamp: chrono::Utc::now(),
        content: serde_json::json!({}),
        content_hash: format!("hash{seq}"),
        signature: "sig".to_string(),
        previous_hash: prev_hash,
        sequence: seq,
    }
}
