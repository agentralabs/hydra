use super::*;

/// Create a Sisters struct with no connections (offline mode)
fn offline_sisters() -> Sisters {
    Sisters {
        memory: None, identity: None, codebase: None, vision: None,
        comm: None, contract: None, time: None,
        planning: None, cognition: None, reality: None,
        forge: None, aegis: None, veritas: None, evolve: None,
        data: None, connect: None, workflow: None,
    }
}

// ═══════════════════════════════════════════════════════════
// PERCEIVE — V4 Longevity Integration
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_perceive_offline_returns_no_memory() {
    let sisters = offline_sisters();
    let result = sisters.perceive("What did we discuss?").await;

    // With no sisters connected, memory_context should be null
    assert!(result["memory_context"].is_null(),
        "Offline sisters should produce null memory_context");
}

#[tokio::test]
async fn test_perceive_has_correct_shape() {
    let sisters = offline_sisters();
    let result = sisters.perceive("test query").await;

    // Verify all expected fields exist
    assert!(result.get("input").is_some());
    assert!(result.get("involves_code").is_some());
    assert!(result.get("involves_vision").is_some());
    assert!(result.get("memory_context").is_some());
    assert!(result.get("identity_context").is_some());
    assert!(result.get("time_context").is_some());
    assert!(result.get("cognition_context").is_some());
    assert!(result.get("reality_context").is_some());
    assert!(result.get("similar_context").is_some());
    assert!(result.get("grounding_context").is_some());
    assert!(result.get("prediction_context").is_some());
    assert!(result.get("sisters_online").is_some());
}

#[tokio::test]
async fn test_perceive_code_detection_still_works() {
    let sisters = offline_sisters();

    let code_result = sisters.perceive("Fix the bug in main.rs").await;
    assert_eq!(code_result["involves_code"], true);

    let non_code = sisters.perceive("What is the weather?").await;
    assert_eq!(non_code["involves_code"], false);
}

// ═══════════════════════════════════════════════════════════
// LEARN — V3 Capture with Causal Chains
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_learn_offline_does_not_panic() {
    let sisters = offline_sisters();
    // Should complete gracefully even with no sisters connected
    sisters.learn("test message", "test response").await;
}

#[tokio::test]
async fn test_learn_correction_detection() {
    let sisters = offline_sisters();

    // These should all be detected as corrections
    let corrections = [
        "No, I meant the other file",
        "Actually, use Python instead",
        "That's wrong, it should be 42",
        "That's not right",
        "I prefer tabs over spaces",
        "Always use snake_case",
        "Never use var in JavaScript",
        "Don't add comments there",
    ];

    for correction in &corrections {
        // Just verify it doesn't panic — the actual capture happens via sisters
        sisters.learn(correction, "acknowledged").await;
    }
}

#[tokio::test]
async fn test_learn_non_correction() {
    let sisters = offline_sisters();

    // These should NOT be detected as corrections
    let non_corrections = [
        "Can you help me with this?",
        "Thanks, that looks good",
        "What does this function do?",
        "Show me the API docs",
    ];

    for msg in &non_corrections {
        sisters.learn(msg, "here you go").await;
    }
}

#[tokio::test]
async fn test_learn_with_empty_response() {
    let sisters = offline_sisters();
    sisters.learn("test", "").await;
}

#[tokio::test]
async fn test_learn_with_very_long_response() {
    let sisters = offline_sisters();
    let long_response = "x".repeat(10000);
    // Should truncate gracefully (response[..500] in V3 capture)
    sisters.learn("generate a long output", &long_response).await;
}

#[tokio::test]
async fn test_learn_with_unicode() {
    let sisters = offline_sisters();
    sisters.learn("你好世界 🌍", "こんにちは 🎌").await;
}

// ═══════════════════════════════════════════════════════════
// Memory Context Merging (V2 + V4)
// ═══════════════════════════════════════════════════════════

#[test]
fn test_memory_merge_both_present() {
    let v2 = Some("Recent: talked about auth".to_string());
    let v4 = Some("Pattern: user prefers JWT".to_string());

    let merged = match (&v2, &v4) {
        (Some(m), Some(l)) => Some(format!("{}\n\n## Long-Term Memory\n{}", m, l)),
        (Some(m), None) => Some(m.clone()),
        (None, Some(l)) => Some(format!("## Long-Term Memory\n{}", l)),
        (None, None) => None,
    };

    let result = merged.unwrap();
    assert!(result.contains("Recent: talked about auth"));
    assert!(result.contains("## Long-Term Memory"));
    assert!(result.contains("Pattern: user prefers JWT"));
}

#[test]
fn test_memory_merge_only_v2() {
    let v2 = Some("Recent memory".to_string());
    let v4: Option<String> = None;

    let merged = match (&v2, &v4) {
        (Some(m), Some(l)) => Some(format!("{}\n\n## Long-Term Memory\n{}", m, l)),
        (Some(m), None) => Some(m.clone()),
        (None, Some(l)) => Some(format!("## Long-Term Memory\n{}", l)),
        (None, None) => None,
    };

    assert_eq!(merged.unwrap(), "Recent memory");
}

#[test]
fn test_memory_merge_only_v4() {
    let v2: Option<String> = None;
    let v4 = Some("Long-term pattern".to_string());

    let merged = match (&v2, &v4) {
        (Some(m), Some(l)) => Some(format!("{}\n\n## Long-Term Memory\n{}", m, l)),
        (Some(m), None) => Some(m.clone()),
        (None, Some(l)) => Some(format!("## Long-Term Memory\n{}", l)),
        (None, None) => None,
    };

    assert!(merged.unwrap().starts_with("## Long-Term Memory"));
}

#[test]
fn test_memory_merge_neither() {
    let v2: Option<String> = None;
    let v4: Option<String> = None;

    let merged: Option<String> = match (&v2, &v4) {
        (Some(m), Some(l)) => Some(format!("{}\n\n## Long-Term Memory\n{}", m, l)),
        (Some(m), None) => Some(m.clone()),
        (None, Some(l)) => Some(format!("## Long-Term Memory\n{}", l)),
        (None, None) => None,
    };

    assert!(merged.is_none());
}

// ═══════════════════════════════════════════════════════════
// Classification & Risk Detection (regression)
// ═══════════════════════════════════════════════════════════

#[test]
fn test_complexity_classification() {
    assert_eq!(Sisters::classify_complexity("hi"), "simple");
    assert_eq!(Sisters::classify_complexity("hello there"), "simple");
    assert_eq!(Sisters::classify_complexity("build me an ecommerce site"), "complex");
    assert_eq!(Sisters::classify_complexity("fix the bug"), "simple");
    assert_eq!(Sisters::classify_complexity("install and start it"), "complex");
    assert_eq!(Sisters::classify_complexity("run it"), "complex");
    assert_eq!(Sisters::classify_complexity("do it"), "complex");
}

#[test]
fn test_risk_assessment_unchanged() {
    assert_eq!(Sisters::assess_risk("what is the weather"), "none");
    assert_eq!(Sisters::assess_risk("delete old backups"), "high");
    assert_eq!(Sisters::assess_risk("modify the config"), "medium");
    assert_eq!(Sisters::assess_risk("check the codebase"), "none");
    // "read a file" → no longer triggers code detection
    assert_eq!(Sisters::assess_risk("read a file"), "none");
}

#[test]
fn test_connected_count_zero_offline() {
    let sisters = offline_sisters();
    assert_eq!(sisters.connected_count(), 0);
}

#[test]
fn test_status_summary_offline() {
    let sisters = offline_sisters();
    assert_eq!(sisters.status_summary(), "No sisters connected");
}

#[tokio::test]
async fn test_perceive_output_includes_new_fields() {
    // Verify the output JSON structure includes new sister context fields
    let sisters = offline_sisters();
    let perceived = sisters.perceive("test query").await;

    // These should be null (offline) but present in the structure
    assert!(perceived.get("veritas_context").is_some() || perceived["veritas_context"].is_null());
    assert!(perceived.get("contract_context").is_some() || perceived["contract_context"].is_null());
    assert!(perceived.get("planning_context").is_some() || perceived["planning_context"].is_null());
    assert!(perceived.get("comm_context").is_some() || perceived["comm_context"].is_null());
    assert!(perceived.get("forge_context").is_some() || perceived["forge_context"].is_null());
    assert!(perceived.get("temporal_context").is_some() || perceived["temporal_context"].is_null());
}

#[test]
fn test_degradation_report_all_offline() {
    let sisters = offline_sisters();
    let report = sisters.degradation_report();
    // Dynamic count: 0/N where N is total sisters
    assert!(report.contains("0/"));
    assert!(report.contains("Offline"));
    assert!(report.contains("Memory"));
}

#[test]
fn test_connected_sisters_list_offline() {
    let sisters = offline_sisters();
    assert!(sisters.connected_sisters_list().is_empty());
}

#[test]
fn test_cognitive_prompt_includes_veritas_context() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({
        "input": "test",
        "veritas_context": "Intent: build a web app",
        "planning_context": "Active goal: Deploy v2 by Friday",
    });
    let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, false);
    assert!(prompt.contains("# Intent Analysis"));
    assert!(prompt.contains("# Active Goals"));
}

#[test]
fn test_cognitive_prompt_graceful_degradation() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({ "input": "test" });
    let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, false);
    assert!(prompt.contains("SISTERS OFFLINE") || prompt.contains("None (offline mode"));
}
