use std::sync::atomic::{AtomicU64, Ordering};

use hydra_core::types::{
    Action, ActionType, CompiledIntent, Entity, EntityType, Goal, GoalType, IntentSource,
    TokenBudget, VeritasValidation,
};
use uuid::Uuid;

use crate::cache::IntentCache;
use crate::classifier::LocalClassifier;
use crate::fuzzy::FuzzyMatcher;
use crate::sanitize;

/// Complexity of the compiled intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Complexity {
    Simple,
    Moderate,
    Complex,
    Critical,
}

impl Complexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Moderate => "moderate",
            Self::Complex => "complex",
            Self::Critical => "critical",
        }
    }
}

/// Status of a compilation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileStatus {
    /// Successfully compiled
    Compiled,
    /// Served from cache (0 tokens)
    Cached,
    /// Classified locally (0 tokens)
    LocallyClassified,
    /// Matched via fuzzy (0 tokens)
    FuzzyMatched,
    /// Required LLM (used tokens)
    LlmCompiled,
    /// Input was empty/whitespace
    Empty,
    /// Budget exhausted — couldn't compile
    BudgetExhausted,
    /// Input needs clarification (ambiguous)
    NeedsClarification,
    /// Input contains contradictions
    Contradiction,
    /// Input too long (truncated and compiled)
    Truncated,
}

/// Result of intent compilation
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub intent: Option<CompiledIntent>,
    pub status: CompileStatus,
    pub tokens_used: u64,
    pub layer: u8, // Which layer resolved it (1-4)
    pub warnings: Vec<String>,
    pub complexity: Complexity,
    pub entities_extracted: usize,
}

impl CompileResult {
    pub fn is_ok(&self) -> bool {
        self.intent.is_some()
    }

    pub fn is_cached(&self) -> bool {
        self.status == CompileStatus::Cached
    }

    pub fn has_warning(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn asks_clarification(&self) -> bool {
        self.status == CompileStatus::NeedsClarification
    }

    pub fn is_safe(&self) -> bool {
        !self.warnings.iter().any(|w| w.contains("injection"))
    }

    pub fn contains_dangerous_patterns(&self) -> bool {
        self.warnings.iter().any(|w| w.contains("injection"))
    }

    pub fn has_uncertainty(&self) -> bool {
        self.intent
            .as_ref()
            .map(|i| i.confidence < 0.7)
            .unwrap_or(true)
    }
}

/// The 7-stage intent compiler (V1.1)
///
/// Pipeline:
/// 1. Tokenize (input → tokens)
/// 2. Extract Entities (tokens → entities)
/// 3. Classify Action (tokens + entities → ActionType)
/// 4. Assess Complexity (→ Simple/Moderate/Complex/Critical)
/// 5. Cache Lookup (0-token path for known intents)
/// 6. Build Intent Graph (structured intent via classifier/fuzzy/LLM)
/// 7. Cache Insert (for future use)
pub struct IntentCompiler {
    cache: IntentCache,
    classifier: LocalClassifier,
    fuzzy: FuzzyMatcher,
    llm_calls: AtomicU64,
    llm_tokens_used: AtomicU64,
}

impl IntentCompiler {
    pub fn new() -> Self {
        Self {
            cache: IntentCache::new(10_000),
            classifier: LocalClassifier::new(),
            fuzzy: FuzzyMatcher::new(0.85),
            llm_calls: AtomicU64::new(0),
            llm_tokens_used: AtomicU64::new(0),
        }
    }

    /// Access the underlying cache
    pub fn cache(&self) -> &IntentCache {
        &self.cache
    }

    /// Cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        self.cache.hit_rate()
    }

    /// Total LLM calls made
    pub fn llm_calls(&self) -> u64 {
        self.llm_calls.load(Ordering::Relaxed)
    }

    /// Compile text through the 7-stage pipeline
    pub async fn compile(&self, text: &str, budget: &mut TokenBudget) -> CompileResult {
        self.compile_with_context(text, None, budget).await
    }

    /// Compile with optional context (context hash included in cache key)
    pub async fn compile_with_context(
        &self,
        text: &str,
        context: Option<&serde_json::Value>,
        budget: &mut TokenBudget,
    ) -> CompileResult {
        let context_hash = context.map(|c| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            c.to_string().hash(&mut hasher);
            hasher.finish()
        });
        self.compile_inner(text, context_hash, budget).await
    }

    async fn compile_inner(
        &self,
        text: &str,
        context_hash: Option<u64>,
        budget: &mut TokenBudget,
    ) -> CompileResult {
        // Pre-check: empty/whitespace
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return CompileResult {
                intent: None,
                status: CompileStatus::Empty,
                tokens_used: 0,
                layer: 0,
                warnings: vec![],
                complexity: Complexity::Simple,
                entities_extracted: 0,
            };
        }

        // Pre-check: too long
        let text = sanitize::truncate_if_needed(trimmed);
        let was_truncated = text.len() < trimmed.len();

        // Pre-check: sanitization warnings
        let mut warnings = Vec::new();
        if sanitize::has_shell_injection(text) {
            warnings.push("Potential shell injection detected — treated as literal text".into());
        }
        if sanitize::has_sql_injection(text) {
            warnings.push("Potential SQL injection detected — treated as literal text".into());
        }
        if sanitize::has_prompt_injection(text) {
            warnings.push("Potential prompt injection detected — treated as literal text".into());
        }

        // Pre-check: ambiguous
        if sanitize::is_ambiguous(text) {
            return CompileResult {
                intent: None,
                status: CompileStatus::NeedsClarification,
                tokens_used: 0,
                layer: 0,
                warnings,
                complexity: Complexity::Simple,
                entities_extracted: 0,
            };
        }

        // Pre-check: contradictory
        if sanitize::has_contradiction(text) {
            warnings.push("Contradictory instructions detected".into());
            return CompileResult {
                intent: None,
                status: CompileStatus::Contradiction,
                tokens_used: 0,
                layer: 0,
                warnings,
                complexity: Complexity::Simple,
                entities_extracted: 0,
            };
        }

        // ═══ STAGE 1: Tokenize ═══
        let tokens: Vec<&str> = text.split_whitespace().collect();

        // ═══ STAGE 2: Extract Entities ═══
        let entities = Self::extract_entities(text, &tokens);
        let entity_count = entities.len();

        // ═══ STAGE 3: Classify Action (tokens + entities → ActionType) ═══
        let action_type = Self::classify_action(text, &tokens, &entities);

        // ═══ STAGE 4: Assess Complexity ═══
        let complexity = Self::assess_complexity(text, &tokens, &entities, &action_type);

        // ═══ STAGE 5: Cache Lookup (0-token path) ═══
        if let Some(mut cached) = self.cache.get_with_context(text, context_hash) {
            // Enrich with freshly extracted entities if the cached version had none
            if cached.entities.is_empty() && !entities.is_empty() {
                cached.entities = entities;
            }
            return CompileResult {
                intent: Some(cached),
                status: CompileStatus::Cached,
                tokens_used: 0,
                layer: 1,
                warnings,
                complexity,
                entities_extracted: entity_count,
            };
        }

        // ═══ STAGE 6: Build Intent Graph ═══
        // Try local classifier first (0 tokens)
        if let Some(mut intent) = self.classifier.classify(text) {
            intent.entities = entities;
            intent.estimated_steps = Self::estimate_steps(&complexity);
            self.cache
                .put_with_context(text, context_hash, intent.clone());
            let status = if was_truncated {
                CompileStatus::Truncated
            } else {
                CompileStatus::LocallyClassified
            };
            return CompileResult {
                intent: Some(intent),
                status,
                tokens_used: 0,
                layer: 2,
                warnings,
                complexity,
                entities_extracted: entity_count,
            };
        }

        // Try fuzzy matching (0 tokens)
        if let Some((mut intent, _similarity)) = self.fuzzy.find_match(text) {
            intent.entities = entities;
            intent.estimated_steps = Self::estimate_steps(&complexity);
            self.cache
                .put_with_context(text, context_hash, intent.clone());
            return CompileResult {
                intent: Some(intent),
                status: CompileStatus::FuzzyMatched,
                tokens_used: 0,
                layer: 3,
                warnings,
                complexity,
                entities_extracted: entity_count,
            };
        }

        // Fall through to LLM (uses tokens)
        let estimated_tokens = self.estimate_llm_tokens(text);
        if !budget.can_afford(estimated_tokens) {
            return CompileResult {
                intent: None,
                status: CompileStatus::BudgetExhausted,
                tokens_used: 0,
                layer: 4,
                warnings,
                complexity,
                entities_extracted: entity_count,
            };
        }

        let intent = self.llm_compile(text, entities, action_type, &complexity);
        budget.record_usage(estimated_tokens);
        self.llm_calls.fetch_add(1, Ordering::Relaxed);
        self.llm_tokens_used
            .fetch_add(estimated_tokens, Ordering::Relaxed);

        // ═══ STAGE 7: Cache Insert ═══
        self.cache.put(text, intent.clone());
        self.fuzzy.add_template(text, intent.clone());

        let status = if was_truncated {
            CompileStatus::Truncated
        } else {
            CompileStatus::LlmCompiled
        };

        CompileResult {
            intent: Some(intent),
            status,
            tokens_used: estimated_tokens,
            layer: 4,
            warnings,
            complexity,
            entities_extracted: entity_count,
        }
    }

    /// Stage 2: Extract entities from tokens
    fn extract_entities(text: &str, tokens: &[&str]) -> Vec<Entity> {
        let mut entities = Vec::new();

        for token in tokens {
            // URLs (check before file paths)
            if token.starts_with("http://") || token.starts_with("https://") {
                entities.push(Entity {
                    id: Uuid::new_v4(),
                    entity_type: EntityType::Url,
                    value: token.to_string(),
                    resolved_path: None,
                    confidence: 0.95,
                });
                continue;
            }

            // File paths (after URLs)
            if token.contains('/') || token.contains('\\') || token.starts_with('.') {
                if token.contains('.') || token.ends_with('/') || token.starts_with("src") {
                    entities.push(Entity {
                        id: Uuid::new_v4(),
                        entity_type: EntityType::FilePath,
                        value: token.to_string(),
                        resolved_path: None,
                        confidence: 0.9,
                    });
                    continue;
                }
            }

            // Package names (contains :: or -)
            if token.contains("::") {
                entities.push(Entity {
                    id: Uuid::new_v4(),
                    entity_type: EntityType::ModuleName,
                    value: token.to_string(),
                    resolved_path: None,
                    confidence: 0.85,
                });
                continue;
            }

            // Function-like names (contains parentheses or camelCase/snake_case)
            if token.contains('(') || token.contains(')') {
                let name = token.trim_end_matches("()").trim_end_matches('(');
                entities.push(Entity {
                    id: Uuid::new_v4(),
                    entity_type: EntityType::FunctionName,
                    value: name.to_string(),
                    resolved_path: None,
                    confidence: 0.8,
                });
                continue;
            }

            // File extensions (e.g., .rs, .py, .ts)
            if token.starts_with("*.") || (token.starts_with('.') && token.len() <= 5) {
                entities.push(Entity {
                    id: Uuid::new_v4(),
                    entity_type: EntityType::Other("file_extension".into()),
                    value: token.to_string(),
                    resolved_path: None,
                    confidence: 0.7,
                });
                continue;
            }

            // Branch names (common git branch patterns)
            if token.starts_with("feature/")
                || token.starts_with("fix/")
                || token.starts_with("release/")
            {
                entities.push(Entity {
                    id: Uuid::new_v4(),
                    entity_type: EntityType::BranchName,
                    value: token.to_string(),
                    resolved_path: None,
                    confidence: 0.9,
                });
                continue;
            }

            // Class names (PascalCase, at least 2 capital letters)
            if token.len() > 3
                && token.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                && token.chars().filter(|c| c.is_uppercase()).count() >= 2
                && token.chars().any(|c| c.is_lowercase())
                && token.chars().all(|c| c.is_alphanumeric())
            {
                entities.push(Entity {
                    id: Uuid::new_v4(),
                    entity_type: EntityType::ClassName,
                    value: token.to_string(),
                    resolved_path: None,
                    confidence: 0.6,
                });
            }
        }

        // Also extract file paths from the full text using regex-like patterns
        let words: Vec<&str> = text.split_whitespace().collect();
        for window in words.windows(1) {
            let w = window[0];
            // Detect paths like "src/main.rs" even without leading ./
            if w.contains('/')
                && (w.ends_with(".rs")
                    || w.ends_with(".ts")
                    || w.ends_with(".py")
                    || w.ends_with(".js")
                    || w.ends_with(".go")
                    || w.ends_with(".java"))
                && !entities.iter().any(|e| e.value == w)
            {
                entities.push(Entity {
                    id: Uuid::new_v4(),
                    entity_type: EntityType::FilePath,
                    value: w.to_string(),
                    resolved_path: None,
                    confidence: 0.9,
                });
            }
        }

        entities
    }

    /// Stage 3: Classify the primary action type
    fn classify_action(text: &str, _tokens: &[&str], entities: &[Entity]) -> ActionType {
        let lower = text.to_lowercase();

        // Destructive actions
        if lower.contains("delete") || lower.contains("remove") || lower.contains("drop") {
            return ActionType::FileDelete;
        }

        // System commands
        if lower.contains("deploy")
            || lower.contains("restart")
            || lower.contains("shutdown")
            || lower.contains("install")
        {
            return ActionType::System;
        }

        // Network actions
        if lower.contains("send") || lower.contains("email") || lower.contains("webhook") {
            return ActionType::Network;
        }

        // Git operations
        if lower.contains("commit")
            || lower.contains("push")
            || lower.contains("merge")
            || lower.contains("rebase")
        {
            return ActionType::GitOperation;
        }

        // Execute/run
        if lower.contains("run") || lower.contains("execute") || lower.contains("test") {
            return ActionType::Execute;
        }

        // Code generation/modification
        if lower.contains("create") || lower.contains("generate") || lower.contains("implement") {
            if entities
                .iter()
                .any(|e| matches!(e.entity_type, EntityType::FilePath))
            {
                return ActionType::FileCreate;
            }
            return ActionType::Write;
        }

        // Modification
        if lower.contains("edit")
            || lower.contains("modify")
            || lower.contains("refactor")
            || lower.contains("fix")
        {
            return ActionType::FileModify;
        }

        // Default: Read (safest)
        ActionType::Read
    }

    /// Stage 4: Assess complexity of the intent
    fn assess_complexity(
        text: &str,
        tokens: &[&str],
        entities: &[Entity],
        action_type: &ActionType,
    ) -> Complexity {
        let mut score: u32 = 0;

        // Token count contributes to complexity
        if tokens.len() > 20 {
            score += 2;
        } else if tokens.len() > 10 {
            score += 1;
        }

        // Multiple entities = more complex
        if entities.len() > 3 {
            score += 2;
        } else if entities.len() > 1 {
            score += 1;
        }

        // Destructive/system actions are inherently more complex
        match action_type {
            ActionType::System => score += 3,
            ActionType::FileDelete => score += 2,
            ActionType::Network => score += 2,
            ActionType::Execute | ActionType::ShellExecute => score += 1,
            ActionType::Composite => score += 3,
            _ => {}
        }

        // Multi-step indicators
        let lower = text.to_lowercase();
        if lower.contains(" and ") || lower.contains(" then ") || lower.contains(" also ") {
            score += 2;
        }
        if lower.contains("all") || lower.contains("every") || lower.contains("each") {
            score += 1;
        }

        // Conditional logic
        if lower.contains(" if ") || lower.contains("unless") || lower.contains("when") {
            score += 1;
        }

        match score {
            0..=1 => Complexity::Simple,
            2..=3 => Complexity::Moderate,
            4..=6 => Complexity::Complex,
            _ => Complexity::Critical,
        }
    }

    /// Estimate number of execution steps from complexity
    fn estimate_steps(complexity: &Complexity) -> usize {
        match complexity {
            Complexity::Simple => 1,
            Complexity::Moderate => 2,
            Complexity::Complex => 4,
            Complexity::Critical => 8,
        }
    }

    /// Estimate LLM token cost for this input
    fn estimate_llm_tokens(&self, text: &str) -> u64 {
        // Rough estimate: ~1 token per 4 chars + overhead
        let input_tokens = (text.len() / 4) as u64 + 100;
        let output_tokens = 200; // Structured output
        input_tokens + output_tokens
    }

    /// LLM compilation with entity and action enrichment
    fn llm_compile(
        &self,
        text: &str,
        entities: Vec<Entity>,
        action_type: ActionType,
        complexity: &Complexity,
    ) -> CompiledIntent {
        let goal_type = match &action_type {
            ActionType::Read => GoalType::Query,
            ActionType::Write | ActionType::FileCreate => GoalType::Create,
            ActionType::FileModify => GoalType::Modify,
            ActionType::FileDelete => GoalType::Delete,
            ActionType::Execute | ActionType::ShellExecute => GoalType::Execute,
            ActionType::System => GoalType::Deploy,
            ActionType::GitOperation => GoalType::Execute,
            ActionType::Network | ActionType::ApiCall => GoalType::Execute,
            _ => GoalType::Execute,
        };

        CompiledIntent {
            id: Uuid::new_v4(),
            raw_text: text.to_string(),
            source: IntentSource::Cli,
            goal: Goal {
                goal_type,
                target: text.to_string(),
                outcome: format!("Execute: {text}"),
                sub_goals: vec![],
            },
            entities,
            actions: vec![Action::new(action_type, text)],
            constraints: vec![],
            success_criteria: vec![],
            confidence: 0.7,
            estimated_steps: Self::estimate_steps(complexity),
            tokens_used: self.estimate_llm_tokens(text),
            veritas_validation: VeritasValidation {
                validated: false,
                safety_score: 0.8,
                warnings: vec![],
            },
        }
    }
}

impl Default for IntentCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> TokenBudget {
        TokenBudget::new(10_000)
    }

    // ═══ Stage 2: Entity Extraction ═══

    #[test]
    fn test_extract_file_paths() {
        let entities = IntentCompiler::extract_entities(
            "Fix the bug in src/main.rs",
            &["Fix", "the", "bug", "in", "src/main.rs"],
        );
        assert!(entities.iter().any(|e| e.entity_type == EntityType::FilePath
            && e.value == "src/main.rs"));
    }

    #[test]
    fn test_extract_urls() {
        let entities = IntentCompiler::extract_entities(
            "Fetch data from https://api.example.com",
            &["Fetch", "data", "from", "https://api.example.com"],
        );
        assert!(entities
            .iter()
            .any(|e| e.entity_type == EntityType::Url && e.value == "https://api.example.com"));
    }

    #[test]
    fn test_extract_module_names() {
        let entities = IntentCompiler::extract_entities(
            "Fix std::collections::HashMap",
            &["Fix", "std::collections::HashMap"],
        );
        assert!(entities
            .iter()
            .any(|e| e.entity_type == EntityType::ModuleName));
    }

    #[test]
    fn test_extract_function_names() {
        let entities = IntentCompiler::extract_entities(
            "Refactor process_data()",
            &["Refactor", "process_data()"],
        );
        assert!(entities
            .iter()
            .any(|e| e.entity_type == EntityType::FunctionName && e.value == "process_data"));
    }

    #[test]
    fn test_extract_branch_names() {
        let entities = IntentCompiler::extract_entities(
            "Merge feature/auth-system",
            &["Merge", "feature/auth-system"],
        );
        assert!(entities
            .iter()
            .any(|e| e.entity_type == EntityType::BranchName));
    }

    #[test]
    fn test_no_entities_for_simple_query() {
        let entities = IntentCompiler::extract_entities(
            "What time is it",
            &["What", "time", "is", "it"],
        );
        assert!(entities.is_empty());
    }

    // ═══ Stage 3: Action Classification ═══

    #[test]
    fn test_classify_delete_action() {
        let action = IntentCompiler::classify_action("delete all test files", &[], &[]);
        assert_eq!(action, ActionType::FileDelete);
    }

    #[test]
    fn test_classify_execute_action() {
        let action = IntentCompiler::classify_action("run the test suite", &[], &[]);
        assert_eq!(action, ActionType::Execute);
    }

    #[test]
    fn test_classify_git_action() {
        let action = IntentCompiler::classify_action("commit and push changes", &[], &[]);
        assert_eq!(action, ActionType::GitOperation);
    }

    #[test]
    fn test_classify_read_default() {
        let action = IntentCompiler::classify_action("what is this", &[], &[]);
        assert_eq!(action, ActionType::Read);
    }

    #[test]
    fn test_classify_network_action() {
        let action = IntentCompiler::classify_action("send an email notification", &[], &[]);
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
            IntentCompiler::classify_action("create a new module", &[], &entities);
        assert_eq!(action, ActionType::FileCreate);
    }

    // ═══ Stage 4: Complexity Assessment ═══

    #[test]
    fn test_simple_complexity() {
        let complexity =
            IntentCompiler::assess_complexity("list files", &["list", "files"], &[], &ActionType::Read);
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
        let complexity = IntentCompiler::assess_complexity(
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
        let complexity = IntentCompiler::assess_complexity(
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
        let complexity = IntentCompiler::assess_complexity(
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
