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
// SYSTEM PROMPT — Memory Capabilities & Honesty Rules
// ═══════════════════════════════════════════════════════════

#[test]
fn test_cognitive_prompt_includes_self_knowledge() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({
        "input": "hello",
        "involves_code": false,
        "involves_vision": false,
    });

    // Full mode (is_complex = true) should include all identity sections
    let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, true);

    assert!(prompt.contains("# Your Identity & Capabilities (Ground Truth)"),
        "System prompt missing capabilities section");
    assert!(prompt.contains("Execute ANY shell command"),
        "System prompt missing shell execution capability");
    assert!(prompt.contains("NEVER say \"I can't\""),
        "System prompt missing anti-hallucination rule");
    assert!(prompt.contains("A BRAIN: AgenticMemory"),
        "System prompt missing Memory organ");
    assert!(prompt.contains("6-layer hierarchy"),
        "System prompt missing hierarchy reference");
    assert!(prompt.contains("System Mutation"),
        "System prompt missing federation/mutation capability");
}

#[test]
fn test_cognitive_prompt_includes_honesty_rules() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({
        "input": "hello",
        "involves_code": false,
        "involves_vision": false,
    });

    let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, false);

    assert!(prompt.contains("## Memory & Honesty Rules"),
        "System prompt missing Honesty Rules section");
    assert!(prompt.contains("Never fabricate past interactions"),
        "System prompt missing fabrication prohibition");
    assert!(prompt.contains("Only claim to remember things verified through memory retrieval"),
        "System prompt missing verification requirement");
}

#[test]
fn test_cognitive_prompt_self_knowledge_before_complex_instructions() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({
        "input": "build me a website",
        "involves_code": true,
        "involves_vision": false,
    });

    let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, true);

    // Self-knowledge should appear before the complex task instructions
    let cap_pos = prompt.find("# Your Identity & Capabilities (Ground Truth)").unwrap();
    let critical_pos = prompt.find("# CRITICAL: You are a COGNITIVE ORCHESTRATOR").unwrap();
    assert!(cap_pos < critical_pos,
        "Capabilities should appear before complex task instructions");
}

#[test]
fn test_cognitive_prompt_honesty_in_simple_mode() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({
        "input": "hi",
        "involves_code": false,
        "involves_vision": false,
    });

    let prompt = sisters.build_cognitive_prompt("", &perceived, false);

    // Memory and honesty rules must be present even in compact mode
    assert!(prompt.contains("## Memory & Honesty Rules"),
        "Compact mode must include honesty rules");
    // Compact mode mentions sisters but NOT the full organ metaphor
    assert!(prompt.contains("17 sister agents"),
        "Compact mode must reference sisters");
}

// ═══════════════════════════════════════════════════════════
// COGNITIVE PROMPT DELTA — New Sections
// ═══════════════════════════════════════════════════════════

#[test]
fn test_cognitive_prompt_organ_metaphor() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({ "input": "hello" });
    // Organ metaphor only in full mode (complex tasks)
    let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, true);

    assert!(prompt.contains("A BODY: Full shell access"),
        "Missing organ metaphor: BODY");
    assert!(prompt.contains("A BRAIN: AgenticMemory"),
        "Missing organ metaphor: BRAIN");
    assert!(prompt.contains("EYES: AgenticVision"),
        "Missing organ metaphor: EYES");
    assert!(prompt.contains("HANDS: AgenticCodebase"),
        "Missing organ metaphor: HANDS");
    assert!(prompt.contains("AN IDENTITY: AgenticIdentity"),
        "Missing organ metaphor: IDENTITY");
    assert!(prompt.contains("A BLUEPRINT ENGINE: AgenticForge"),
        "Missing organ metaphor: FORGE");

    // Compact mode should NOT have the full organ metaphor
    let compact = sisters.build_cognitive_prompt("TestUser", &perceived, false);
    assert!(!compact.contains("A BODY: Full shell access"),
        "Compact mode should not include organ metaphor");
}

#[test]
fn test_cognitive_prompt_15_inventions() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({ "input": "hello" });
    // Inventions only in full mode
    let prompt = sisters.build_cognitive_prompt("", &perceived, true);

    assert!(prompt.contains("## Your 15 Inventions"),
        "Missing inventions section");
    assert!(prompt.contains("System Mutation"),
        "Missing invention: System Mutation");
    assert!(prompt.contains("Resurrection"),
        "Missing invention: Resurrection");
    assert!(prompt.contains("Distributed Self"),
        "Missing invention: Distributed Self");
    assert!(prompt.contains("Cognitive Forking"),
        "Missing invention: Cognitive Forking");
    assert!(prompt.contains("Future Echo"),
        "Missing invention: Future Echo");
    assert!(prompt.contains("Dream State"),
        "Missing invention: Dream State");
    assert!(prompt.contains("Shadow Self"),
        "Missing invention: Shadow Self");
    assert!(prompt.contains("Zero-Trust Autonomy"),
        "Missing invention: Zero-Trust Autonomy");
    assert!(prompt.contains("Graceful Degradation"),
        "Missing invention: Graceful Degradation");

    // Compact mode: no inventions
    let compact = sisters.build_cognitive_prompt("", &perceived, false);
    assert!(!compact.contains("## Your 15 Inventions"),
        "Compact mode should not include inventions");
}

#[test]
fn test_cognitive_prompt_execution_gate() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({ "input": "hello" });
    // Execution gate detail only in full mode
    let prompt = sisters.build_cognitive_prompt("", &perceived, true);

    assert!(prompt.contains("## Execution Gate"),
        "Missing execution gate section");
    assert!(prompt.contains("NONE/LOW: Execute immediately"),
        "Missing LOW risk guidance");
}

#[test]
fn test_cognitive_prompt_personality() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({ "input": "hello" });
    // Personality only in full mode
    let prompt = sisters.build_cognitive_prompt("", &perceived, true);

    assert!(prompt.contains("## Your Personality"),
        "Missing personality section");
    assert!(prompt.contains("brilliant cofounder"),
        "Missing cofounder tone directive");
    assert!(prompt.contains("not a servant"),
        "Missing partner framing");

    // Compact mode: no personality section
    let compact = sisters.build_cognitive_prompt("", &perceived, false);
    assert!(!compact.contains("## Your Personality"),
        "Compact mode should not include personality");
}

#[test]
fn test_cognitive_prompt_response_format() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({ "input": "hello" });
    // Response format only in full mode
    let prompt = sisters.build_cognitive_prompt("", &perceived, true);

    assert!(prompt.contains("## Response Format"),
        "Missing response format section");
    assert!(prompt.contains("For BUILD tasks"),
        "Missing BUILD task format");
    assert!(prompt.contains("For DEBUG tasks"),
        "Missing DEBUG task format");
    assert!(prompt.contains("For CAPABILITY questions"),
        "Missing CAPABILITY format");
}

#[test]
fn test_cognitive_prompt_runtime_context() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({ "input": "hello" });
    let prompt = sisters.build_cognitive_prompt("TestUser", &perceived, false);

    assert!(prompt.contains("## Current Runtime Context"),
        "Missing runtime context section");
    assert!(prompt.contains("USER: TestUser"),
        "Missing user in runtime context");
    assert!(prompt.contains("SISTERS ONLINE: None (offline mode"),
        "Missing sisters status in runtime context");
}

#[test]
fn test_cognitive_prompt_runtime_context_with_trust() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({
        "input": "hello",
        "trust_level": "Partner",
        "project_name": "my-app",
    });
    let prompt = sisters.build_cognitive_prompt("", &perceived, false);

    assert!(prompt.contains("TRUST LEVEL: Partner"),
        "Missing trust level in runtime context");
    assert!(prompt.contains("PROJECT: my-app"),
        "Missing project name in runtime context");
}

#[test]
fn test_cognitive_prompt_inventions_before_complex_instructions() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({ "input": "build me an app" });
    let prompt = sisters.build_cognitive_prompt("", &perceived, true);

    let inv_pos = prompt.find("## Your 15 Inventions").unwrap();
    let critical_pos = prompt.find("# CRITICAL: You are a COGNITIVE ORCHESTRATOR").unwrap();
    assert!(inv_pos < critical_pos,
        "Inventions should appear before complex task instructions");
}

#[test]
fn test_cognitive_prompt_simple_mode_no_complex_instructions() {
    let sisters = offline_sisters();
    let perceived = serde_json::json!({ "input": "hi" });
    let prompt = sisters.build_cognitive_prompt("", &perceived, false);

    // Compact mode should NOT include complex build instructions
    assert!(!prompt.contains("# CRITICAL: You are a COGNITIVE ORCHESTRATOR"),
        "Compact mode should not include complex build instructions");
    // Compact mode should NOT include heavy sections (token optimization)
    assert!(!prompt.contains("## Your 15 Inventions"),
        "Compact mode should not include inventions");
    assert!(!prompt.contains("## Your Personality"),
        "Compact mode should not include personality");
    // BUT must still include core execution rules
    assert!(prompt.contains("<hydra-exec>"),
        "Compact mode must include hydra-exec instructions");
    assert!(prompt.contains("## Memory & Honesty Rules"),
        "Compact mode must include honesty rules");
}
