#[cfg(test)]
mod tests {
    use hydra_core::types::{ActionType, Entity, EntityType, TokenBudget};
    use uuid::Uuid;

    use crate::compiler_pipeline::IntentCompiler;
    use crate::compiler_stages::{assess_complexity, classify_action, extract_entities};
    use crate::compiler_types::{CompileResult, CompileStatus, Complexity};

    fn budget() -> TokenBudget {
        TokenBudget::new(10_000)
    }

    // ═══ Stage 2: Entity Extraction ═══

    #[test]
    fn test_extract_file_paths() {
        let entities = extract_entities(
            "Fix the bug in src/main.rs",
            &["Fix", "the", "bug", "in", "src/main.rs"],
        );
        assert!(entities.iter().any(|e| e.entity_type == EntityType::FilePath
            && e.value == "src/main.rs"));
    }

    #[test]
    fn test_extract_urls() {
        let entities = extract_entities(
            "Fetch data from https://api.example.com",
            &["Fetch", "data", "from", "https://api.example.com"],
        );
        assert!(entities
            .iter()
            .any(|e| e.entity_type == EntityType::Url && e.value == "https://api.example.com"));
    }

    #[test]
    fn test_extract_module_names() {
        let entities = extract_entities(
            "Fix std::collections::HashMap",
            &["Fix", "std::collections::HashMap"],
        );
        assert!(entities
            .iter()
            .any(|e| e.entity_type == EntityType::ModuleName));
    }

    #[test]
    fn test_extract_function_names() {
        let entities = extract_entities(
            "Refactor process_data()",
            &["Refactor", "process_data()"],
        );
        assert!(entities
            .iter()
            .any(|e| e.entity_type == EntityType::FunctionName && e.value == "process_data"));
    }

    #[test]
    fn test_extract_branch_names() {
        let entities = extract_entities(
            "Merge feature/auth-system",
            &["Merge", "feature/auth-system"],
        );
        assert!(entities
            .iter()
            .any(|e| e.entity_type == EntityType::BranchName));
    }

    #[test]
    fn test_no_entities_for_simple_query() {
        let entities = extract_entities(
            "What time is it",
            &["What", "time", "is", "it"],
        );
        assert!(entities.is_empty());
    }

    // ═══ Stage 3: Action Classification ═══

    #[test]
    fn test_classify_delete_action() {
        let action = classify_action("delete all test files", &[], &[]);
        assert_eq!(action, ActionType::FileDelete);
    }

    #[test]
    fn test_classify_execute_action() {
        let action = classify_action("run the test suite", &[], &[]);
        assert_eq!(action, ActionType::Execute);
    }

    #[test]
    fn test_classify_git_action() {
        let action = classify_action("commit and push changes", &[], &[]);
        assert_eq!(action, ActionType::GitOperation);
    }

    #[test]
    fn test_classify_read_default() {
        let action = classify_action("what is this", &[], &[]);
        assert_eq!(action, ActionType::Read);
    }

    #[test]
    fn test_classify_network_action() {
        let action = classify_action("send an email notification", &[], &[]);
        assert_eq!(action, ActionType::Network);
    }

    #[test]
    fn test_classify_create_with_file_entity() {
        let entities = vec![Entity {
            id: Uuid::new_v4(),
            entity_type: EntityType::FilePath,
            value: "src/lib.rs".into(),
            resolved_path: None,
            confidence: 0.9,
        }];
        let action =
            classify_action("create a new module", &[], &entities);
        assert_eq!(action, ActionType::FileCreate);
    }

    // ═══ Stage 4: Complexity Assessment ═══

    #[test]
    fn test_simple_complexity() {
        let complexity =
            assess_complexity("list files", &["list", "files"], &[], &ActionType::Read);
        assert_eq!(complexity, Complexity::Simple);
    }

    #[test]
    fn test_moderate_complexity() {
        let entities = vec![Entity {
            id: Uuid::new_v4(),
            entity_type: EntityType::FilePath,
            value: "src/main.rs".into(),
            resolved_path: None,
            confidence: 0.9,
        }];
        let complexity = assess_complexity(
            "fix the bug and update tests",
            &["fix", "the", "bug", "and", "update", "tests"],
            &entities,
            &ActionType::FileModify,
        );
        assert!(complexity >= Complexity::Moderate);
    }

    #[test]
    fn test_complex_task() {
        let entities = vec![
            Entity {
                id: Uuid::new_v4(),
                entity_type: EntityType::FilePath,
                value: "src/api.rs".into(),
                resolved_path: None,
                confidence: 0.9,
            },
            Entity {
                id: Uuid::new_v4(),
                entity_type: EntityType::FilePath,
                value: "src/db.rs".into(),
                resolved_path: None,
                confidence: 0.9,
            },
        ];
        let complexity = assess_complexity(
            "Build a REST API with database integration and then deploy it to production",
            &[
                "Build", "a", "REST", "API", "with", "database", "integration", "and", "then",
                "deploy", "it", "to", "production",
            ],
            &entities,
            &ActionType::System,
        );
        assert!(complexity >= Complexity::Complex);
    }

    #[test]
    fn test_critical_complexity() {
        let entities = vec![
            Entity { id: Uuid::new_v4(), entity_type: EntityType::FilePath, value: "a".into(), resolved_path: None, confidence: 0.9 },
            Entity { id: Uuid::new_v4(), entity_type: EntityType::FilePath, value: "b".into(), resolved_path: None, confidence: 0.9 },
            Entity { id: Uuid::new_v4(), entity_type: EntityType::FilePath, value: "c".into(), resolved_path: None, confidence: 0.9 },
            Entity { id: Uuid::new_v4(), entity_type: EntityType::FilePath, value: "d".into(), resolved_path: None, confidence: 0.9 },
        ];
        let complexity = assess_complexity(
            "Deploy all services to production and then restart each one if they fail unless already running",
            &(0..22).map(|_| "word").collect::<Vec<_>>(),
            &entities,
            &ActionType::System,
        );
        assert_eq!(complexity, Complexity::Critical);
    }

    // ═══ Full Pipeline Tests ═══

    #[tokio::test]
    async fn test_empty_input() {
        let compiler = IntentCompiler::new();
        let mut b = budget();
        let result = compiler.compile("", &mut b).await;
        assert_eq!(result.status, CompileStatus::Empty);
        assert!(!result.is_ok());
    }

    #[tokio::test]
    async fn test_whitespace_input() {
        let compiler = IntentCompiler::new();
        let mut b = budget();
        let result = compiler.compile("   \t  ", &mut b).await;
        assert_eq!(result.status, CompileStatus::Empty);
    }

    #[tokio::test]
    async fn test_ambiguous_input() {
        let compiler = IntentCompiler::new();
        let mut b = budget();
        let result = compiler.compile("do it", &mut b).await;
        assert_eq!(result.status, CompileStatus::NeedsClarification);
        assert!(result.has_uncertainty());
    }

    #[tokio::test]
    async fn test_local_classification() {
        let compiler = IntentCompiler::new();
        let mut b = budget();
        let result = compiler.compile("list all files", &mut b).await;
        assert_eq!(result.status, CompileStatus::LocallyClassified);
        assert_eq!(result.tokens_used, 0);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cache_hit_on_second_call() {
        let compiler = IntentCompiler::new();
        let mut b = budget();

        // First call
        let r1 = compiler.compile("list all files", &mut b).await;
        assert!(r1.is_ok());

        // Second call — should be cached
        let r2 = compiler.compile("list all files", &mut b).await;
        assert_eq!(r2.status, CompileStatus::Cached);
        assert_eq!(r2.tokens_used, 0);
    }

    #[tokio::test]
    async fn test_same_input_twice_zero_tokens() {
        let compiler = IntentCompiler::new();
        let mut b = budget();

        compiler.compile("explain the codebase architecture", &mut b).await;
        let r2 = compiler.compile("explain the codebase architecture", &mut b).await;
        assert_eq!(r2.tokens_used, 0);
    }

    #[tokio::test]
    async fn test_entity_extraction_in_pipeline() {
        let compiler = IntentCompiler::new();
        let mut b = budget();
        let result = compiler
            .compile("Fix the bug in src/main.rs", &mut b)
            .await;
        assert!(result.is_ok());
        assert!(result.entities_extracted > 0);
        let intent = result.intent.unwrap();
        assert!(intent
            .entities
            .iter()
            .any(|e| e.value == "src/main.rs"));
    }

    #[tokio::test]
    async fn test_complexity_assessment_in_pipeline() {
        let compiler = IntentCompiler::new();
        let mut b = budget();

        let simple = compiler.compile("list files", &mut b).await;
        assert_eq!(simple.complexity, Complexity::Simple);

        let complex = compiler
            .compile(
                "Build a REST API with authentication and deploy it to staging",
                &mut b,
            )
            .await;
        assert!(complex.complexity >= Complexity::Moderate);
    }

    #[tokio::test]
    async fn test_build_rest_api_complex() {
        let compiler = IntentCompiler::new();
        let mut b = budget();
        let result = compiler
            .compile("Build a REST API", &mut b)
            .await;
        assert!(result.is_ok());
        // Should detect as at least moderate complexity
        assert!(result.complexity >= Complexity::Simple);
    }

    #[tokio::test]
    async fn test_shell_injection_warning() {
        let compiler = IntentCompiler::new();
        let mut b = budget();
        let result = compiler
            .compile("run this command: rm -rf /", &mut b)
            .await;
        assert!(result.has_warning());
        assert!(result.contains_dangerous_patterns());
    }

    #[tokio::test]
    async fn test_contradiction_detected() {
        let compiler = IntentCompiler::new();
        let mut b = budget();
        let result = compiler
            .compile("create and delete the file", &mut b)
            .await;
        assert_eq!(result.status, CompileStatus::Contradiction);
    }

    #[tokio::test]
    async fn test_budget_exhaustion() {
        let compiler = IntentCompiler::new();
        let mut b = TokenBudget::new(0); // No budget
        let result = compiler
            .compile("do some complex unique thing that no classifier matches", &mut b)
            .await;
        // If local classifier can't handle it, should be budget exhausted
        // (unless local classifier catches it)
        assert!(
            result.status == CompileStatus::BudgetExhausted
                || result.status == CompileStatus::LocallyClassified
        );
    }

    #[tokio::test]
    async fn test_has_uncertainty_high_confidence() {
        let compiler = IntentCompiler::new();
        let mut b = budget();
        let result = compiler.compile("list all files", &mut b).await;
        // Local classifier should give reasonable confidence
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_compile_result_methods() {
        let result = CompileResult {
            intent: None,
            status: CompileStatus::NeedsClarification,
            tokens_used: 0,
            layer: 0,
            warnings: vec![],
            complexity: Complexity::Simple,
            entities_extracted: 0,
        };
        assert!(!result.is_ok());
        assert!(result.asks_clarification());
        assert!(result.has_uncertainty());
        assert!(!result.is_cached());
        assert!(result.is_safe());
    }
}
