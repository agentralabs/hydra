#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ast::{ActionNode, CollectionExpr, ConditionExpr, ParamExpr};
    use crate::compiler::CompiledAction;
    use crate::executor::{CompiledExecutor, ExecutionResult};

    fn make_compiled(ast: ActionNode) -> CompiledAction {
        CompiledAction {
            id: "test-1".into(),
            signature: "test".into(),
            ast,
            required_variables: vec![],
            compiled_at: chrono::Utc::now().to_rfc3339(),
            source_occurrences: 5,
            source_success_rate: 1.0,
        }
    }

    #[test]
    fn test_execute_single_action() {
        let compiled = make_compiled(ActionNode::Action {
            tool: "test_tool".into(),
            params: HashMap::from([("key".into(), ParamExpr::Literal(serde_json::json!("value")))]),
        });

        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);

        assert!(result.success);
        assert_eq!(result.tokens_used, 0);
        assert_eq!(result.steps_executed, 1);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_zero_tokens() {
        let compiled = make_compiled(ActionNode::Sequence(vec![
            ActionNode::Action {
                tool: "a".into(),
                params: HashMap::new(),
            },
            ActionNode::Action {
                tool: "b".into(),
                params: HashMap::new(),
            },
            ActionNode::Action {
                tool: "c".into(),
                params: HashMap::new(),
            },
        ]));

        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert_eq!(result.tokens_used, 0);
        assert_eq!(result.steps_executed, 3);
    }

    #[test]
    fn test_variable_resolution() {
        let compiled = make_compiled(ActionNode::Action {
            tool: "commit".into(),
            params: HashMap::from([("msg".into(), ParamExpr::Variable("user_msg".into()))]),
        });

        let mut executor = CompiledExecutor::with_variables(HashMap::from([(
            "user_msg".into(),
            serde_json::json!("fix: the bug"),
        )]));
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(
            result.results[0].params["msg"],
            serde_json::json!("fix: the bug")
        );
    }

    #[test]
    fn test_foreach_execution() {
        let compiled = make_compiled(ActionNode::ForEach {
            variable: "file".into(),
            collection: CollectionExpr::Literal(vec![
                serde_json::json!("a.rs"),
                serde_json::json!("b.rs"),
            ]),
            body: Box::new(ActionNode::Action {
                tool: "lint".into(),
                params: HashMap::from([("path".into(), ParamExpr::Variable("file".into()))]),
            }),
        });

        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.steps_executed, 2);
    }

    #[test]
    fn test_default_executor() {
        let executor = CompiledExecutor::default();
        let compiled = make_compiled(ActionNode::Action {
            tool: "t".into(),
            params: HashMap::new(),
        });
        let mut executor = executor;
        let result = executor.execute(&compiled);
        assert!(result.success);
    }

    #[test]
    fn test_store_result() {
        let compiled = make_compiled(ActionNode::StoreResult {
            key: "step1".into(),
            action: Box::new(ActionNode::Action {
                tool: "fetch".into(),
                params: HashMap::new(),
            }),
        });
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.steps_executed, 1);
    }

    #[test]
    fn test_if_condition_exists_true() {
        let compiled = make_compiled(ActionNode::Sequence(vec![
            ActionNode::StoreResult {
                key: "data".into(),
                action: Box::new(ActionNode::Action { tool: "fetch".into(), params: HashMap::new() }),
            },
            ActionNode::If {
                condition: ConditionExpr::Exists("data".into()),
                then: Box::new(ActionNode::Action { tool: "process".into(), params: HashMap::new() }),
                else_: None,
            },
        ]));
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.steps_executed, 2); // fetch + process
    }

    #[test]
    fn test_if_condition_exists_false() {
        let compiled = make_compiled(ActionNode::If {
            condition: ConditionExpr::Exists("nonexistent".into()),
            then: Box::new(ActionNode::Action { tool: "skip".into(), params: HashMap::new() }),
            else_: Some(Box::new(ActionNode::Action { tool: "fallback".into(), params: HashMap::new() })),
        });
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.results[0].tool, "fallback");
    }

    #[test]
    fn test_condition_not() {
        let compiled = make_compiled(ActionNode::If {
            condition: ConditionExpr::Not(Box::new(ConditionExpr::Exists("nope".into()))),
            then: Box::new(ActionNode::Action { tool: "yes".into(), params: HashMap::new() }),
            else_: None,
        });
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert_eq!(result.steps_executed, 1);
        assert_eq!(result.results[0].tool, "yes");
    }

    #[test]
    fn test_condition_and() {
        let mut executor = CompiledExecutor::with_variables(HashMap::from([
            ("a".into(), serde_json::json!("exists")),
            ("b".into(), serde_json::json!("also")),
        ]));
        let compiled = make_compiled(ActionNode::If {
            condition: ConditionExpr::And(vec![
                ConditionExpr::Exists("a".into()),
                ConditionExpr::Exists("b".into()),
            ]),
            then: Box::new(ActionNode::Action { tool: "both".into(), params: HashMap::new() }),
            else_: None,
        });
        let result = executor.execute(&compiled);
        assert_eq!(result.steps_executed, 1);
    }

    #[test]
    fn test_condition_or() {
        let mut executor = CompiledExecutor::with_variables(HashMap::from([
            ("a".into(), serde_json::json!("yes")),
        ]));
        let compiled = make_compiled(ActionNode::If {
            condition: ConditionExpr::Or(vec![
                ConditionExpr::Exists("a".into()),
                ConditionExpr::Exists("z".into()),
            ]),
            then: Box::new(ActionNode::Action { tool: "found".into(), params: HashMap::new() }),
            else_: None,
        });
        let result = executor.execute(&compiled);
        assert_eq!(result.steps_executed, 1);
    }

    #[test]
    fn test_previous_result_param() {
        let compiled = make_compiled(ActionNode::Sequence(vec![
            ActionNode::StoreResult {
                key: "step1".into(),
                action: Box::new(ActionNode::Action { tool: "fetch".into(), params: HashMap::new() }),
            },
            ActionNode::Action {
                tool: "use".into(),
                params: HashMap::from([("data".into(), ParamExpr::PreviousResult("step1".into()))]),
            },
        ]));
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.steps_executed, 2);
    }

    #[test]
    fn test_execution_result_serde() {
        let result = ExecutionResult {
            compiled_id: "id".into(),
            signature: "sig".into(),
            success: true,
            tokens_used: 0,
            duration_ms: 5,
            steps_executed: 1,
            results: vec![],
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: ExecutionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tokens_used, 0);
    }

    #[test]
    fn test_foreach_empty_collection() {
        let compiled = make_compiled(ActionNode::ForEach {
            variable: "item".into(),
            collection: CollectionExpr::Literal(vec![]),
            body: Box::new(ActionNode::Action { tool: "process".into(), params: HashMap::new() }),
        });
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.steps_executed, 0);
    }
}
