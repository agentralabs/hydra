use std::collections::HashMap;

use hydra_compiler::normalizer::{
    InferredType, NormalizedAction, NormalizedParam, RawAction, VariableInfo,
};
use hydra_compiler::{
    ActionCompiler, ActionNode, CollectionExpr, CompiledExecutor, ConditionExpr, ExecutionRouter,
    NormalizedSequence, ParamExpr, PatternDetector, RoutingDecision, SequenceNormalizer,
};

// === End-to-end: detect → normalize → compile → execute → route ===

#[test]
fn test_full_pipeline_detect_to_execute() {
    // 1. Detect pattern
    let detector = PatternDetector::with_defaults();
    let actions = vec!["git add".into(), "git commit".into()];
    let tools = vec!["git_add".into(), "git_commit".into()];

    for _ in 0..5 {
        detector.record("git_add→git_commit", &actions, &tools, true);
    }

    let patterns = detector.detect();
    assert_eq!(patterns.len(), 1);
    assert!(patterns[0].compilable);

    // 2. Normalize from raw instances
    let instances = vec![
        vec![
            RawAction {
                tool: "git_add".into(),
                params: HashMap::from([("path".into(), serde_json::json!("."))]),
            },
            RawAction {
                tool: "git_commit".into(),
                params: HashMap::from([("message".into(), serde_json::json!("fix: bug A"))]),
            },
        ],
        vec![
            RawAction {
                tool: "git_add".into(),
                params: HashMap::from([("path".into(), serde_json::json!("."))]),
            },
            RawAction {
                tool: "git_commit".into(),
                params: HashMap::from([("message".into(), serde_json::json!("feat: new thing"))]),
            },
        ],
        vec![
            RawAction {
                tool: "git_add".into(),
                params: HashMap::from([("path".into(), serde_json::json!("."))]),
            },
            RawAction {
                tool: "git_commit".into(),
                params: HashMap::from([("message".into(), serde_json::json!("chore: cleanup"))]),
            },
        ],
    ];

    let normalized = SequenceNormalizer::normalize(&instances).unwrap();
    assert_eq!(normalized.signature, "git_add→git_commit");
    assert_eq!(normalized.variables.len(), 1); // message varies

    // 3. Compile
    let compiled = ActionCompiler::compile(&normalized, 5, 1.0);
    assert_eq!(compiled.signature, "git_add→git_commit");
    assert_eq!(compiled.ast.action_count(), 2);

    // 4. Execute with zero tokens
    let mut executor = CompiledExecutor::with_variables(HashMap::from([(
        normalized.variables.keys().next().unwrap().clone(),
        serde_json::json!("fix: the real bug"),
    )]));
    let result = executor.execute(&compiled);
    assert!(result.success);
    assert_eq!(result.tokens_used, 0);
    assert_eq!(result.steps_executed, 2);

    // 5. Route prefers compiled
    let router = ExecutionRouter::new();
    router.register(compiled);
    assert!(matches!(
        router.route("git_add→git_commit"),
        RoutingDecision::Compiled { .. }
    ));
}

#[test]
fn test_router_execute_with_variables() {
    let normalized = NormalizedSequence {
        actions: vec![NormalizedAction {
            tool: "deploy".into(),
            params: HashMap::from([(
                "env".into(),
                NormalizedParam::Variable {
                    name: "target_env".into(),
                },
            )]),
        }],
        variables: HashMap::from([(
            "target_env".into(),
            VariableInfo {
                name: "target_env".into(),
                sample_values: vec![serde_json::json!("staging"), serde_json::json!("prod")],
                inferred_type: InferredType::String,
            },
        )]),
        signature: "deploy".into(),
    };

    let compiled = ActionCompiler::compile(&normalized, 4, 1.0);
    let router = ExecutionRouter::new();
    router.register(compiled);

    let result = router
        .execute_compiled(
            "deploy",
            HashMap::from([("target_env".into(), serde_json::json!("prod"))]),
        )
        .unwrap();

    assert!(result.success);
    assert_eq!(result.tokens_used, 0);
    assert_eq!(result.results[0].params["env"], serde_json::json!("prod"));
}

#[test]
fn test_conditional_ast_execution() {
    let ast = ActionNode::If {
        condition: ConditionExpr::Exists("feature_flag".into()),
        then: Box::new(ActionNode::Action {
            tool: "deploy_v2".into(),
            params: HashMap::new(),
        }),
        else_: Some(Box::new(ActionNode::Action {
            tool: "deploy_v1".into(),
            params: HashMap::new(),
        })),
    };

    let compiled = hydra_compiler::compiler::CompiledAction {
        id: "cond-1".into(),
        signature: "conditional_deploy".into(),
        ast,
        required_variables: vec!["feature_flag".into()],
        compiled_at: chrono::Utc::now().to_rfc3339(),
        source_occurrences: 3,
        source_success_rate: 1.0,
    };

    // With feature_flag → deploy_v2
    let mut executor = CompiledExecutor::with_variables(HashMap::from([(
        "feature_flag".into(),
        serde_json::json!(true),
    )]));
    let result = executor.execute(&compiled);
    assert!(result.success);
    assert_eq!(result.results[0].tool, "deploy_v2");

    // Without feature_flag → deploy_v1
    let mut executor = CompiledExecutor::new();
    let result = executor.execute(&compiled);
    assert!(result.success);
    assert_eq!(result.results[0].tool, "deploy_v1");
}

#[test]
fn test_foreach_with_collection() {
    let ast = ActionNode::ForEach {
        variable: "file".into(),
        collection: CollectionExpr::Literal(vec![
            serde_json::json!("a.rs"),
            serde_json::json!("b.rs"),
            serde_json::json!("c.rs"),
        ]),
        body: Box::new(ActionNode::Action {
            tool: "lint".into(),
            params: HashMap::from([("path".into(), ParamExpr::Variable("file".into()))]),
        }),
    };

    let compiled = hydra_compiler::compiler::CompiledAction {
        id: "loop-1".into(),
        signature: "lint_all".into(),
        ast,
        required_variables: vec![],
        compiled_at: chrono::Utc::now().to_rfc3339(),
        source_occurrences: 3,
        source_success_rate: 1.0,
    };

    let mut executor = CompiledExecutor::new();
    let result = executor.execute(&compiled);
    assert!(result.success);
    assert_eq!(result.tokens_used, 0);
    assert_eq!(result.steps_executed, 3);
    assert_eq!(result.results[0].params["path"], serde_json::json!("a.rs"));
    assert_eq!(result.results[2].params["path"], serde_json::json!("c.rs"));
}

#[test]
fn test_store_and_reference_result() {
    let ast = ActionNode::Sequence(vec![
        ActionNode::StoreResult {
            key: "build_output".into(),
            action: Box::new(ActionNode::Action {
                tool: "build".into(),
                params: HashMap::new(),
            }),
        },
        ActionNode::Action {
            tool: "deploy".into(),
            params: HashMap::from([(
                "artifact".into(),
                ParamExpr::PreviousResult("build_output".into()),
            )]),
        },
    ]);

    let compiled = hydra_compiler::compiler::CompiledAction {
        id: "chain-1".into(),
        signature: "build→deploy".into(),
        ast,
        required_variables: vec![],
        compiled_at: chrono::Utc::now().to_rfc3339(),
        source_occurrences: 5,
        source_success_rate: 1.0,
    };

    let mut executor = CompiledExecutor::new();
    let result = executor.execute(&compiled);
    assert!(result.success);
    assert_eq!(result.steps_executed, 2);
    // The deploy step should have the build_output as artifact param
    assert!(result.results[1].params.contains_key("artifact"));
}

#[test]
fn test_router_stats_tracking() {
    let router = ExecutionRouter::new();

    let normalized = NormalizedSequence {
        actions: vec![NormalizedAction {
            tool: "test".into(),
            params: HashMap::new(),
        }],
        variables: HashMap::new(),
        signature: "run_tests".into(),
    };

    router.register(ActionCompiler::compile(&normalized, 5, 1.0));

    // 3 compiled hits, 2 LLM fallbacks
    router.route("run_tests");
    router.route("run_tests");
    router.route("run_tests");
    router.route("unknown_a");
    router.route("unknown_b");

    let stats = router.stats();
    assert_eq!(stats.compiled_hits, 3);
    assert_eq!(stats.llm_fallbacks, 2);
    assert_eq!(stats.compiled_actions, 1);
}
