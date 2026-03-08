use std::sync::atomic::{AtomicU64, Ordering};

use hydra_core::types::{
    Action, ActionType, CompiledIntent, Goal, GoalType, IntentSource, TokenBudget,
    VeritasValidation,
};
use uuid::Uuid;

use crate::cache::IntentCache;
use crate::classifier::LocalClassifier;
use crate::fuzzy::FuzzyMatcher;
use crate::sanitize;

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
}

/// The 4-layer token-conservative intent compiler
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

    /// Compile text through the 4-layer escalation
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
            };
        }

        // ═══ LAYER 1: Cache lookup (0 tokens) ═══
        if let Some(cached) = self.cache.get_with_context(text, context_hash) {
            return CompileResult {
                intent: Some(cached),
                status: CompileStatus::Cached,
                tokens_used: 0,
                layer: 1,
                warnings,
            };
        }

        // ═══ LAYER 2: Local classifier (0 tokens) ═══
        if let Some(intent) = self.classifier.classify(text) {
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
            };
        }

        // ═══ LAYER 3: Fuzzy matching (0 tokens) ═══
        if let Some((intent, _similarity)) = self.fuzzy.find_match(text) {
            self.cache
                .put_with_context(text, context_hash, intent.clone());
            return CompileResult {
                intent: Some(intent),
                status: CompileStatus::FuzzyMatched,
                tokens_used: 0,
                layer: 3,
                warnings,
            };
        }

        // ═══ LAYER 4: LLM compilation (uses tokens) ═══
        let estimated_tokens = self.estimate_llm_tokens(text);
        if !budget.can_afford(estimated_tokens) {
            return CompileResult {
                intent: None,
                status: CompileStatus::BudgetExhausted,
                tokens_used: 0,
                layer: 4,
                warnings,
            };
        }

        // Simulate LLM compilation (in real implementation, calls an LLM)
        let intent = self.llm_compile(text);
        budget.record_usage(estimated_tokens);
        self.llm_calls.fetch_add(1, Ordering::Relaxed);
        self.llm_tokens_used
            .fetch_add(estimated_tokens, Ordering::Relaxed);

        // Cache the result for future (amortize cost)
        self.cache.put(text, intent.clone());
        // Also add to fuzzy templates
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
        }
    }

    /// Estimate LLM token cost for this input
    fn estimate_llm_tokens(&self, text: &str) -> u64 {
        // Rough estimate: ~1 token per 4 chars + overhead
        let input_tokens = (text.len() / 4) as u64 + 100;
        let output_tokens = 200; // Structured output
        input_tokens + output_tokens
    }

    /// Simulate LLM compilation (placeholder for real LLM call)
    fn llm_compile(&self, text: &str) -> CompiledIntent {
        CompiledIntent {
            id: Uuid::new_v4(),
            raw_text: text.to_string(),
            source: IntentSource::Cli,
            goal: Goal {
                goal_type: GoalType::Execute,
                target: text.to_string(),
                outcome: format!("Execute: {text}"),
                sub_goals: vec![],
            },
            entities: vec![],
            actions: vec![Action::new(ActionType::Execute, text)],
            constraints: vec![],
            success_criteria: vec![],
            confidence: 0.7, // LLM is less confident than exact match
            estimated_steps: 1,
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
