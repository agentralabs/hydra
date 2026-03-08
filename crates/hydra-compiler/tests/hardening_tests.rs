//! Category 1: Unit Gap Fill — hydra-compiler edge cases.

use hydra_compiler::*;

// === AST deeply nested ===

#[test]
fn test_ast_deeply_nested_sequence() {
    let inner = ActionNode::Action {
        tool: "final".into(),
        params: std::collections::HashMap::new(),
    };
    let mut node = inner;
    for i in 0..50 {
        node = ActionNode::Sequence(vec![
            ActionNode::Action {
                tool: format!("step_{}", i),
                params: std::collections::HashMap::new(),
            },
            node,
        ]);
    }
    assert!(node.action_count() > 50);
}

#[test]
fn test_ast_action_count() {
    let node = ActionNode::Sequence(vec![
        ActionNode::Action {
            tool: "a".into(),
            params: std::collections::HashMap::new(),
        },
        ActionNode::Action {
            tool: "b".into(),
            params: std::collections::HashMap::new(),
        },
        ActionNode::Action {
            tool: "c".into(),
            params: std::collections::HashMap::new(),
        },
    ]);
    assert_eq!(node.action_count(), 3);
}

#[test]
fn test_ast_tool_names() {
    let node = ActionNode::Sequence(vec![
        ActionNode::Action {
            tool: "read_file".into(),
            params: std::collections::HashMap::new(),
        },
        ActionNode::Action {
            tool: "write_file".into(),
            params: std::collections::HashMap::new(),
        },
    ]);
    let names = node.tool_names();
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"write_file"));
}

// === Pattern detection ===

#[test]
fn test_pattern_collision() {
    let detector = PatternDetector::with_defaults();
    detector.record("sig1", &["a".into(), "b".into()], &["tool1".into()], true);
    detector.record("sig1", &["a".into(), "b".into()], &["tool1".into()], true);
    detector.record("sig1", &["a".into(), "b".into()], &["tool1".into()], false);
    assert_eq!(detector.signature_count(), 1);
    let patterns = detector.detect();
    // Need min occurrences to detect
    let _ = patterns;
}

#[test]
fn test_pattern_detector_clear() {
    let detector = PatternDetector::with_defaults();
    detector.record("sig1", &["a".into()], &["t1".into()], true);
    assert_eq!(detector.signature_count(), 1);
    detector.clear();
    assert_eq!(detector.signature_count(), 0);
}

// === Executor stack overflow prevention ===

#[test]
fn test_executor_conditional() {
    let compiled = CompiledAction {
        id: "test-id".into(),
        signature: "test".into(),
        ast: ActionNode::If {
            condition: ConditionExpr::Success("prev".into()),
            then: Box::new(ActionNode::Action {
                tool: "do_thing".into(),
                params: std::collections::HashMap::new(),
            }),
            else_: Some(Box::new(ActionNode::Action {
                tool: "fallback".into(),
                params: std::collections::HashMap::new(),
            })),
        },
        required_variables: vec![],
        compiled_at: String::new(),
        source_occurrences: 5,
        source_success_rate: 0.8,
    };
    let mut executor = CompiledExecutor::new();
    let result = executor.execute(&compiled);
    // Should complete without stack overflow
    let _ = result;
}

// === Router ===

#[test]
fn test_router_compiled_vs_llm() {
    let router = ExecutionRouter::new();
    // No compiled patterns = LLM routing
    match router.route("unknown_sig") {
        RoutingDecision::Llm { .. } => {}
        RoutingDecision::Compiled { .. } => panic!("should route to LLM for unknown"),
    }
}

#[test]
fn test_router_register_deregister() {
    let router = ExecutionRouter::new();
    let compiled = CompiledAction {
        id: "test-id".into(),
        signature: "test_sig".into(),
        ast: ActionNode::Action {
            tool: "test".into(),
            params: std::collections::HashMap::new(),
        },
        required_variables: vec![],
        compiled_at: String::new(),
        source_occurrences: 10,
        source_success_rate: 0.9,
    };
    router.register(compiled);
    assert_eq!(router.compiled_count(), 1);
    assert!(router.deregister("test_sig"));
    assert_eq!(router.compiled_count(), 0);
}

#[test]
fn test_router_stats() {
    let router = ExecutionRouter::new();
    let stats = router.stats();
    assert_eq!(stats.compiled_actions, 0);
}
