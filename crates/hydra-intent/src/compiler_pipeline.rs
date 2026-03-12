use std::sync::atomic::{AtomicU64, Ordering};

use hydra_core::types::{
    Action, ActionType, CompiledIntent, Goal, GoalType, IntentSource, TokenBudget,
    VeritasValidation,
};
use uuid::Uuid;

use crate::cache::IntentCache;
use crate::classifier::LocalClassifier;
use crate::compiler_stages::{assess_complexity, classify_action, estimate_steps, extract_entities};
use crate::compiler_types::{CompileResult, CompileStatus, Complexity};
use crate::fuzzy::FuzzyMatcher;
use crate::sanitize;

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
        let entities = extract_entities(text, &tokens);
        let entity_count = entities.len();

        // ═══ STAGE 3: Classify Action (tokens + entities → ActionType) ═══
        let action_type = classify_action(text, &tokens, &entities);

        // ═══ STAGE 4: Assess Complexity ═══
        let complexity = assess_complexity(text, &tokens, &entities, &action_type);

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
            intent.estimated_steps = estimate_steps(&complexity);
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
            intent.estimated_steps = estimate_steps(&complexity);
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
        entities: Vec<hydra_core::types::Entity>,
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
            estimated_steps: estimate_steps(complexity),
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
