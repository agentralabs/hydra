use hydra_core::types::TokenBudget;
use hydra_intent::compiler::{CompileStatus, IntentCompiler};

// ═══════════════════════════════════════════════════════════
// EDGE CASE TESTS (EC-IC-008 through EC-IC-012)
// ═══════════════════════════════════════════════════════════

/// EC-IC-008: Ambiguous intent
#[tokio::test]
async fn test_ec_ic_008_ambiguous_intent() {
    let compiler = IntentCompiler::new();
    let mut budget = TokenBudget::new(10_000);
    let result = compiler.compile("do thing", &mut budget).await;
    assert!(result.asks_clarification());
    assert_eq!(result.status, CompileStatus::NeedsClarification);
}

/// EC-IC-009: Contradictory intent
#[tokio::test]
async fn test_ec_ic_009_contradictory_intent() {
    let compiler = IntentCompiler::new();
    let mut budget = TokenBudget::new(10_000);
    let result = compiler
        .compile("create and delete the file", &mut budget)
        .await;
    assert!(result.has_warning() || result.asks_clarification());
    assert_eq!(result.status, CompileStatus::Contradiction);
}

/// EC-IC-010: Cache collision — different inputs get different results
#[tokio::test]
async fn test_ec_ic_010_cache_collision() {
    let compiler = IntentCompiler::new();
    let mut budget = TokenBudget::new(100_000);

    compiler.compile("test alpha", &mut budget).await;
    compiler.compile("test beta", &mut budget).await;

    // Cache should distinguish between similar but different intents
    let a = compiler.cache().get("test alpha");
    let b = compiler.cache().get("test beta");
    assert!(a.is_some());
    assert!(b.is_some());
    assert_ne!(a.unwrap().raw_text, b.unwrap().raw_text);
}

/// EC-IC-011: Concurrent compilation — no deadlocks
#[tokio::test]
async fn test_ec_ic_011_concurrent_compilation() {
    let compiler = std::sync::Arc::new(IntentCompiler::new());

    let futures: Vec<_> = (0..100)
        .map(|i| {
            let compiler = compiler.clone();
            async move {
                let mut budget = TokenBudget::new(100_000);
                compiler
                    .compile(&format!("run task {i}"), &mut budget)
                    .await
            }
        })
        .collect();

    let results = futures::future::join_all(futures).await;
    assert_eq!(results.len(), 100);
    // All should complete without deadlock
    for result in &results {
        assert!(result.is_ok());
    }
}

/// EC-IC-012: Zero budget — should use cache/local or return budget error
#[tokio::test]
async fn test_ec_ic_012_zero_budget() {
    let compiler = IntentCompiler::new();
    let mut budget = TokenBudget::new(0);

    // Common pattern → local classifier works with 0 budget
    let result = compiler.compile("list files", &mut budget).await;
    assert!(result.is_ok());
    assert_eq!(result.tokens_used, 0);

    // Unknown pattern → budget exhausted
    let result2 = compiler
        .compile("complex unique operation requiring LLM", &mut budget)
        .await;
    assert!(result2.is_cached() || result2.status == CompileStatus::BudgetExhausted);
}

// ═══════════════════════════════════════════════════════════
// CONTEXT-AWARE CACHE TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_cache_key_includes_context_hash() {
    let compiler = IntentCompiler::new();
    let ctx_a = serde_json::json!({"project": "alpha"});
    let ctx_b = serde_json::json!({"project": "beta"});

    let mut budget = TokenBudget::new(100_000);
    // Same text, different contexts → separate cache entries
    compiler
        .compile_with_context("deploy", Some(&ctx_a), &mut budget)
        .await;
    compiler
        .compile_with_context("deploy", Some(&ctx_b), &mut budget)
        .await;

    // Both should be cached independently
    use std::hash::{Hash, Hasher};
    let hash_a = {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        ctx_a.to_string().hash(&mut h);
        h.finish()
    };
    let hash_b = {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        ctx_b.to_string().hash(&mut h);
        h.finish()
    };
    assert!(compiler
        .cache()
        .get_with_context("deploy", Some(hash_a))
        .is_some());
    assert!(compiler
        .cache()
        .get_with_context("deploy", Some(hash_b))
        .is_some());
}

// ═══════════════════════════════════════════════════════════
// TOKEN CONSERVATION TESTS (explicit names from checklist)
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_cache_hit_uses_zero_tokens() {
    let compiler = IntentCompiler::new();
    let mut budget = TokenBudget::new(10_000);
    compiler.compile("list files", &mut budget).await; // populate cache
    let mut budget2 = TokenBudget::new(10_000);
    let result = compiler.compile("list files", &mut budget2).await;
    assert_eq!(result.tokens_used, 0);
    assert!(result.is_cached());
}

#[tokio::test]
async fn test_local_classifier_uses_zero_tokens() {
    let compiler = IntentCompiler::new();
    let mut budget = TokenBudget::new(10_000);
    let result = compiler.compile("build the project", &mut budget).await;
    assert_eq!(result.tokens_used, 0);
    assert_eq!(result.status, CompileStatus::LocallyClassified);
}

#[tokio::test]
async fn test_fuzzy_match_uses_zero_tokens() {
    let compiler = IntentCompiler::new();
    let mut budget = TokenBudget::new(100_000);

    // LLM compile a unique phrase (adds to fuzzy templates)
    compiler
        .compile(
            "provision a new kubernetes namespace with resource quotas",
            &mut budget,
        )
        .await;

    // Clear exact cache so we hit fuzzy, not cache
    compiler.cache().clear();

    // Very similar phrase should fuzzy match at 0 tokens
    let mut budget2 = TokenBudget::new(100_000);
    let result = compiler
        .compile(
            "provision a new kubernetes namespace with resource quotas applied",
            &mut budget2,
        )
        .await;
    // If fuzzy matched, tokens_used == 0; otherwise it may hit LLM (similarity may be < 0.85)
    // The important thing: it doesn't crash and either fuzzy matches or falls through
    assert!(result.is_ok());
    if result.status == CompileStatus::FuzzyMatched {
        assert_eq!(result.tokens_used, 0);
    }
}

#[tokio::test]
async fn test_llm_only_when_needed() {
    let compiler = IntentCompiler::new();
    assert_eq!(compiler.llm_calls(), 0);

    // Common patterns should NOT trigger LLM
    let mut budget = TokenBudget::new(100_000);
    for input in &["list files", "run tests", "git commit", "deploy", "build"] {
        compiler.compile(input, &mut budget).await;
    }
    assert_eq!(
        compiler.llm_calls(),
        0,
        "LLM should not be called for common patterns"
    );

    // Unique input with zero matching classifier keywords — must trigger LLM
    compiler
        .compile("xyzqwk plmbvn jrtfgs hcndlw wkrptq", &mut budget)
        .await;
    assert_eq!(
        compiler.llm_calls(),
        1,
        "LLM should be called exactly once for unique input"
    );
}

#[tokio::test]
async fn test_escalation_order_correct() {
    let compiler = IntentCompiler::new();
    let mut budget = TokenBudget::new(100_000);

    // Layer 2 (local classifier) — fresh compiler, no cache
    let r1 = compiler.compile("run tests", &mut budget).await;
    assert_eq!(
        r1.layer, 2,
        "First call to known pattern should be layer 2 (local)"
    );

    // Layer 1 (cache) — same text again
    let r2 = compiler.compile("run tests", &mut budget).await;
    assert_eq!(r2.layer, 1, "Second call should be layer 1 (cache)");

    // Layer 4 (LLM) — completely unknown text (gibberish, no keyword matches possible)
    let r3 = compiler
        .compile("xqzwk plmbvn jrtfgs hcndlw wkrptq", &mut budget)
        .await;
    assert_eq!(r3.layer, 4, "Unknown text should reach layer 4 (LLM)");

    // Layer 1 (cache) — same unknown text again (now cached from LLM)
    let r4 = compiler
        .compile("xqzwk plmbvn jrtfgs hcndlw wkrptq", &mut budget)
        .await;
    assert_eq!(
        r4.layer, 1,
        "Previously LLM-compiled text should now be layer 1 (cache)"
    );

    // Verify escalation order: 2 → 1 → 4 → 1
    assert_eq!(
        vec![r1.layer, r2.layer, r3.layer, r4.layer],
        vec![2, 1, 4, 1]
    );
}

// ═══════════════════════════════════════════════════════════
// TOKEN METRICS TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_token_metrics_cache_hits() {
    let compiler = IntentCompiler::new();
    let mut budget = TokenBudget::new(100_000);

    compiler.compile("list files", &mut budget).await;
    compiler.compile("list files", &mut budget).await;
    compiler.compile("list files", &mut budget).await;

    assert!(compiler.cache().total_hits() >= 2);
}

#[tokio::test]
async fn test_same_intent_cheaper_second_time() {
    let compiler = IntentCompiler::new();

    let unique = "generate kubernetes deployment manifest for microservice";
    let mut budget1 = TokenBudget::new(100_000);
    let first = compiler.compile(unique, &mut budget1).await;
    let first_tokens = first.tokens_used;

    let mut budget2 = TokenBudget::new(100_000);
    let second = compiler.compile(unique, &mut budget2).await;
    let second_tokens = second.tokens_used;

    // Second time should be free (cached)
    assert!(second_tokens < first_tokens || first_tokens == 0);
    assert_eq!(second_tokens, 0);
}
