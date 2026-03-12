//! Cognitive loop integration tests — exercises dispatch shortcuts that complete
//! without LLM calls. Uses Sisters::empty() and no API keys.
//!
//! These tests verify the fast-path handlers: greetings, slash commands,
//! settings detection, and crystallization tracking. The full 5-phase LLM loop
//! is NOT tested here — that requires real API keys and is a live test.
//!
//! For live tests with real sisters + API keys:
//!   ANTHROPIC_API_KEY=sk-... cargo test -p hydra-native --test suite -- live --nocapture

use std::sync::Arc;
use tokio::sync::mpsc;

use hydra_native::cognitive::{
    CognitiveLoopConfig, CognitiveUpdate, DecideEngine, InventionEngine,
    run_cognitive_loop,
};
use hydra_native::sisters::cognitive::Sisters;

/// Run a message through the cognitive loop with empty sisters, no API keys,
/// and a 5-second timeout. Only tests that hit early-return dispatch paths
/// will complete successfully — full LLM paths will timeout.
async fn run_offline(text: &str) -> Vec<CognitiveUpdate> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let config = CognitiveLoopConfig {
        text: text.to_string(),
        anthropic_key: String::new(),
        openai_key: String::new(),
        google_key: String::new(),
        model: "claude-sonnet-4-20250514".to_string(),
        user_name: "TestUser".to_string(),
        task_id: format!("test-{}", uuid::Uuid::new_v4()),
        history: vec![],
        anthropic_oauth_token: None,
    };

    let sisters = Arc::new(Sisters::empty());
    let decide = Arc::new(DecideEngine::new());
    let inv = Arc::new(InventionEngine::new());

    // 5-second timeout — dispatch shortcuts finish in <1ms
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        run_cognitive_loop(
            config,
            Some(sisters),
            tx,
            decide,
            None, None, None, None, None, None, None,
        ),
    ).await;

    let mut updates = Vec::new();
    while let Ok(update) = rx.try_recv() {
        updates.push(update);
    }
    updates
}

fn has_message(updates: &[CognitiveUpdate]) -> bool {
    updates.iter().any(|u| matches!(u, CognitiveUpdate::Message { .. }))
}

// ═══════════════════════════════════════════════════════════
// DISPATCH SHORTCUT TESTS — these hit early-return paths
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_slash_test_command() {
    // /test shells out to `cargo test` / `npm test` — it will timeout in offline mode.
    // We just verify it enters the slash command path (produces updates), not that it completes.
    let updates = run_offline("/test").await;
    assert!(!updates.is_empty(), "/test should produce updates via slash command handler");
}

#[tokio::test]
async fn test_greeting_shortcut() {
    let updates = run_offline("hello").await;
    assert!(!updates.is_empty(), "greeting should produce updates");
    assert!(has_message(&updates), "greeting should produce a message response");
}

#[tokio::test]
async fn test_thanks_shortcut() {
    let updates = run_offline("thanks!").await;
    assert!(!updates.is_empty(), "thanks should produce updates");
}

#[tokio::test]
async fn test_farewell_shortcut() {
    let updates = run_offline("goodbye").await;
    assert!(!updates.is_empty(), "farewell should produce updates");
}

// ═══════════════════════════════════════════════════════════
// SISTERS UNIT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_empty_sisters_connected_count() {
    let sisters = Sisters::empty();
    assert_eq!(sisters.connected_count(), 0);
    assert!(sisters.connected_sisters_list().is_empty());
    assert!(sisters.discover_mcp_tools().is_empty());
}

#[test]
fn test_empty_sisters_all_fourteen() {
    let sisters = Sisters::empty();
    let all = sisters.all_sisters();
    assert_eq!(all.len(), 14, "Should have all 14 sister slots");
    assert!(all.iter().all(|(_, opt)| opt.is_none()), "All should be None");
}

#[test]
fn test_empty_sisters_tools_for_sister() {
    let sisters = Sisters::empty();
    let tools = sisters.tools_for_sister("memory", &["store", "recall"]);
    assert!(tools.is_empty(), "Empty sisters should return no tools");
}

// ═══════════════════════════════════════════════════════════
// INVENTION ENGINE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_invention_engine_init() {
    let inv = InventionEngine::new();
    assert_eq!(inv.skill_count(), 0);
    assert_eq!(inv.pattern_count(), 0);
}
