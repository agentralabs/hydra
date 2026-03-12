use std::collections::HashMap;

use hydra_compiler::normalizer::{
    InferredType, NormalizedAction, NormalizedParam, RawAction, VariableInfo,
};
use hydra_compiler::{
    ActionCompiler, ActionNode, CompiledExecutor, ConditionExpr, ExecutionRouter,
    NormalizedSequence, PatternDetector, RoutingDecision, SequenceNormalizer,
};

#[test]
fn test_detector_custom_thresholds() {
    let detector = PatternDetector::new(hydra_compiler::detector::DetectorConfig {
        min_occurrences: 10,
        min_success_rate: 0.95,
        max_age_days: 30,
    });

    let actions = vec!["deploy".into()];
    let tools = vec!["deploy_tool".into()];

    // 9 successes — below threshold of 10
    for _ in 0..9 {
        detector.record("deploy_flow", &actions, &tools, true);
    }
    assert!(detector.detect().is_empty());

    // 10th success — meets threshold
    detector.record("deploy_flow", &actions, &tools, true);
    assert_eq!(detector.detect().len(), 1);
}

#[test]
fn test_normalizer_preserves_literals() {
    let instances = vec![
        vec![RawAction {
            tool: "format".into(),
            params: HashMap::from([
                ("style".into(), serde_json::json!("prettier")),
                ("file".into(), serde_json::json!("src/main.rs")),
            ]),
        }],
        vec![RawAction {
            tool: "format".into(),
            params: HashMap::from([
                ("style".into(), serde_json::json!("prettier")),
                ("file".into(), serde_json::json!("src/lib.rs")),
            ]),
        }],
    ];

    let norm = SequenceNormalizer::normalize(&instances).unwrap();
    // style is constant → literal
    assert_eq!(
        norm.actions[0].params["style"],
        NormalizedParam::Literal(serde_json::json!("prettier"))
    );
    // file varies → variable
    assert!(matches!(
        norm.actions[0].params["file"],
        NormalizedParam::Variable { .. }
    ));
}

#[test]
fn test_compiled_action_serialization() {
    let normalized = NormalizedSequence {
        actions: vec![
            NormalizedAction {
                tool: "build".into(),
                params: HashMap::from([(
                    "target".into(),
                    NormalizedParam::Literal(serde_json::json!("release")),
                )]),
            },
            NormalizedAction {
                tool: "deploy".into(),
                params: HashMap::from([(
                    "env".into(),
                    NormalizedParam::Variable { name: "env".into() },
                )]),
            },
        ],
        variables: HashMap::from([(
            "env".into(),
            VariableInfo {
                name: "env".into(),
                sample_values: vec![serde_json::json!("prod")],
                inferred_type: InferredType::String,
            },
        )]),
        signature: "build→deploy".into(),
    };

    let compiled = ActionCompiler::compile(&normalized, 8, 0.95);
    let json = serde_json::to_string(&compiled).unwrap();
    let restored: hydra_compiler::compiler::CompiledAction = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.signature, "build→deploy");
    assert_eq!(restored.source_occurrences, 8);
    assert_eq!(restored.ast.action_count(), 2);
}

#[test]
fn test_condition_logic_and_or_not() {
    let ast = ActionNode::If {
        condition: ConditionExpr::And(vec![
            ConditionExpr::Exists("a".into()),
            ConditionExpr::Not(Box::new(ConditionExpr::Exists("b".into()))),
        ]),
        then: Box::new(ActionNode::Action {
            tool: "yes".into(),
            params: HashMap::new(),
        }),
        else_: Some(Box::new(ActionNode::Action {
            tool: "no".into(),
            params: HashMap::new(),
        })),
    };

    let compiled = hydra_compiler::compiler::CompiledAction {
        id: "logic-1".into(),
        signature: "logic_test".into(),
        ast,
        required_variables: vec![],
        compiled_at: chrono::Utc::now().to_rfc3339(),
        source_occurrences: 3,
        source_success_rate: 1.0,
    };

    // a=true, b absent → And(Exists(a)=true, Not(Exists(b))=true) → then
    let mut executor =
        CompiledExecutor::with_variables(HashMap::from([("a".into(), serde_json::json!(1))]));
    let result = executor.execute(&compiled);
    assert_eq!(result.results[0].tool, "yes");

    // a=true, b=true → And(true, Not(true)=false) → else
    let mut executor = CompiledExecutor::with_variables(HashMap::from([
        ("a".into(), serde_json::json!(1)),
        ("b".into(), serde_json::json!(2)),
    ]));
    let result = executor.execute(&compiled);
    assert_eq!(result.results[0].tool, "no");
}

#[test]
fn test_router_deregister_falls_back() {
    let router = ExecutionRouter::new();
    let normalized = NormalizedSequence {
        actions: vec![NormalizedAction {
            tool: "temp".into(),
            params: HashMap::new(),
        }],
        variables: HashMap::new(),
        signature: "temp_flow".into(),
    };

    router.register(ActionCompiler::compile(&normalized, 3, 1.0));
    assert!(matches!(
        router.route("temp_flow"),
        RoutingDecision::Compiled { .. }
    ));

    router.deregister("temp_flow");
    assert!(matches!(
        router.route("temp_flow"),
        RoutingDecision::Llm { .. }
    ));
}

#[test]
fn test_empty_instances_returns_none() {
    assert!(SequenceNormalizer::normalize(&[]).is_none());
    assert!(SequenceNormalizer::normalize(&[vec![]]).is_none());
}
