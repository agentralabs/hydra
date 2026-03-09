//! Live integration test for the T21-T35 Advanced Capability Testing Sprint.
//! Spawns actual sisters, runs the cognitive loop with real messages, and captures output.
//!
//! Run with: cargo test -p hydra-native --test live_sprint_test -- --nocapture

use std::sync::Arc;
use tokio::sync::mpsc;

use hydra_native::cognitive::{
    CognitiveLoopConfig, CognitiveUpdate, DecideEngine, InventionEngine,
    run_cognitive_loop,
};
use hydra_native::sisters::cognitive::init_sisters;

/// Run a single message through the cognitive loop and collect all CognitiveUpdate events.
async fn run_message(
    text: &str,
    sisters: &Arc<hydra_native::sisters::cognitive::Sisters>,
    inv: &Arc<InventionEngine>,
    history: &[(String, String)],
) -> Vec<CognitiveUpdate> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let config = CognitiveLoopConfig {
        text: text.to_string(),
        anthropic_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
        openai_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        google_key: String::new(),
        model: "claude-sonnet-4-20250514".to_string(),
        user_name: "TestUser".to_string(),
        task_id: format!("test-{}", uuid::Uuid::new_v4()),
        history: history.to_vec(),
        anthropic_oauth_token: std::env::var("ANTHROPIC_OAUTH_TOKEN").ok(),
    };

    let sisters_handle = Some(sisters.clone());
    let decide = Arc::new(DecideEngine::new());
    let inventions = Some(inv.clone());

    // Run cognitive loop
    run_cognitive_loop(
        config,
        sisters_handle,
        tx,
        decide,
        None, // undo_stack
        inventions,
        None, // proactive_notifier
        None, // spawner
        None, // approval_manager
        None, // db
        None, // federation
    ).await;

    // Collect all updates
    let mut updates = Vec::new();
    while let Ok(update) = rx.try_recv() {
        updates.push(update);
    }
    updates
}

/// Extract the final Message content from a list of CognitiveUpdate events.
fn extract_messages(updates: &[CognitiveUpdate]) -> Vec<String> {
    updates.iter().filter_map(|u| {
        if let CognitiveUpdate::Message { content, role, .. } = u {
            if role == "hydra" {
                Some(content.clone())
            } else {
                None
            }
        } else {
            None
        }
    }).collect()
}

/// Extract phases from updates.
fn extract_phases(updates: &[CognitiveUpdate]) -> Vec<String> {
    updates.iter().filter_map(|u| {
        if let CognitiveUpdate::Phase(phase) = u {
            Some(phase.clone())
        } else {
            None
        }
    }).collect()
}

fn print_test_result(test_id: &str, description: &str, messages: &[String], phases: &[String]) {
    println!("\n{}", "=".repeat(60));
    println!("TEST {}: {}", test_id, description);
    println!("{}", "=".repeat(60));
    println!("Phases: {:?}", phases);
    for (i, msg) in messages.iter().enumerate() {
        println!("Response #{}: {}", i + 1, &msg[..msg.len().min(500)]);
    }
    if messages.is_empty() {
        println!("Response: <NO MESSAGE>");
    }
    println!();
}

#[tokio::test]
async fn test_block5_identity_receipts() {
    println!("\n\n========================================");
    println!("BLOCK 5: IDENTITY & RECEIPTS");
    println!("========================================\n");

    // Spawn sisters (real MCP connections)
    let sisters = init_sisters().await;
    let inv = Arc::new(InventionEngine::new());

    // T21: First do an action, then ask about it
    // Step 1: Run /test to create an action
    let updates = run_message("/test", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T21a", "/test (create action)", &messages, &phases);

    // Step 2: Ask what Hydra just did
    let history = vec![("user".into(), "/test".into()), ("hydra".into(), messages.first().cloned().unwrap_or_default())];
    let updates = run_message("what did you just do?", &sisters, &inv, &history).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T21b", "\"what did you just do?\"", &messages, &phases);

    // T22: Prove what you did
    let updates = run_message("prove what you did in the last hour", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T22", "\"prove what you did in the last hour\"", &messages, &phases);

    // T23: Trust level
    let updates = run_message("what's my trust level?", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T23", "\"what's my trust level?\"", &messages, &phases);

    // T35: Receipts
    let updates = run_message("show my receipts", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T35", "\"show my receipts\"", &messages, &phases);
}

#[tokio::test]
async fn test_block6_planning_time() {
    println!("\n\n========================================");
    println!("BLOCK 6: PLANNING & TIME");
    println!("========================================\n");

    let sisters = init_sisters().await;
    let inv = Arc::new(InventionEngine::new());

    // T24: Create a goal
    let updates = run_message("create a goal: deploy v2.0 by Friday", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T24", "\"create a goal: deploy v2.0 by Friday\"", &messages, &phases);

    // T25: What are my goals
    let updates = run_message("what are my goals?", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T25", "\"what are my goals?\"", &messages, &phases);

    // T26: Any deadlines
    let updates = run_message("any deadlines?", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T26", "\"any deadlines?\"", &messages, &phases);
}

#[tokio::test]
async fn test_block7_beliefs() {
    println!("\n\n========================================");
    println!("BLOCK 7: BELIEFS");
    println!("========================================\n");

    let sisters = init_sisters().await;
    let inv = Arc::new(InventionEngine::new());

    // T28: Store a belief
    let updates = run_message("we're using PostgreSQL and Express for this project", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T28a", "\"we're using PostgreSQL and Express for this project\"", &messages, &phases);

    // T28b: Recall the belief
    let updates = run_message("what stack are we using?", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T28b", "\"what stack are we using?\"", &messages, &phases);

    // T29: Correct the belief
    let updates = run_message("actually, we switched to FastAPI instead of Express", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T29a", "\"actually, we switched to FastAPI instead of Express\"", &messages, &phases);

    // T29b: Verify correction
    let updates = run_message("what's our backend framework?", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T29b", "\"what's our backend framework?\"", &messages, &phases);
}

#[tokio::test]
async fn test_block8_evolve() {
    println!("\n\n========================================");
    println!("BLOCK 8: EVOLVE (CRYSTALLIZATION)");
    println!("========================================\n");

    let sisters = init_sisters().await;
    let inv = Arc::new(InventionEngine::new());

    // T30: Run /test three times
    for i in 1..=3 {
        let updates = run_message("/test", &sisters, &inv, &[]).await;
        let messages = extract_messages(&updates);
        let phases = extract_phases(&updates);
        print_test_result(&format!("T30.{}", i), &format!("/test run #{}", i), &messages, &phases);

        // Check crystallization
        let has_crystal = updates.iter().any(|u| matches!(u, CognitiveUpdate::SkillCrystallized { .. }));
        println!("  Crystallized? {}", has_crystal);
        println!("  Total skills: {}", inv.skill_count());
        println!("  Total patterns: {}", inv.pattern_count());
    }

    // T31: Try "run tests" to see if crystallized skill fires
    let updates = run_message("run tests", &sisters, &inv, &[]).await;
    let messages = extract_messages(&updates);
    let phases = extract_phases(&updates);
    print_test_result("T31", "\"run tests\" (expect crystallized)", &messages, &phases);
}
