use std::path::PathBuf;
use crate::self_modify::{GapType, SpecGap};
use super::{
    extract_spec_path, parse_gaps_from_response, parse_patches_from_response,
    pick_cheapest_model,
};
use crate::self_modify_llm_parse::{extract_mcp_text, parse_gaps_json_str};

// ═══════════════════════════════════════════════════════════
// SPEC PATH EXTRACTION
// ═══════════════════════════════════════════════════════════

#[test]
fn test_extract_spec_path_basic() {
    let path = extract_spec_path("implement spec test-specs/SPEC-VERSION-COMMAND.md");
    assert_eq!(path, Some(PathBuf::from("test-specs/SPEC-VERSION-COMMAND.md")));
}

#[test]
fn test_extract_spec_path_quoted() {
    let path = extract_spec_path(r#"implement spec "specs/MY-SPEC.md""#);
    assert_eq!(path, Some(PathBuf::from("specs/MY-SPEC.md")));
}

#[test]
fn test_extract_spec_path_txt() {
    let path = extract_spec_path("implement this spec docs/plan.txt");
    assert_eq!(path, Some(PathBuf::from("docs/plan.txt")));
}

#[test]
fn test_extract_spec_path_none() {
    assert!(extract_spec_path("implement a version command").is_none());
    assert!(extract_spec_path("build yourself").is_none());
}

#[test]
fn test_extract_spec_path_no_dir() {
    // Plain filename without directory separator — should NOT match
    assert!(extract_spec_path("implement spec README.md").is_none());
}

// ═══════════════════════════════════════════════════════════
// GAP JSON PARSING
// ═══════════════════════════════════════════════════════════

#[test]
fn test_parse_gaps_valid_json() {
    let json = r#"[
        {"description": "Add /version handler", "target_file": "crates/hydra-cli/src/commands.rs", "gap_type": "missing_function", "priority": 1},
        {"description": "Add version test", "target_file": "crates/hydra-cli/tests/cli_tests.rs", "gap_type": "missing_test", "priority": 2}
    ]"#;
    let gaps = parse_gaps_from_response(json);
    assert_eq!(gaps.len(), 2);
    assert_eq!(gaps[0].description, "Add /version handler");
    assert!(matches!(gaps[0].gap_type, GapType::MissingFunction));
    assert!(matches!(gaps[1].gap_type, GapType::MissingTest));
}

#[test]
fn test_parse_gaps_with_markdown_fences() {
    let json = "```json\n[{\"description\": \"test\", \"target_file\": \"src/lib.rs\", \"gap_type\": \"missing_function\", \"priority\": 1}]\n```";
    let gaps = parse_gaps_from_response(json);
    assert_eq!(gaps.len(), 1);
}

#[test]
fn test_parse_gaps_invalid_json() {
    let gaps = parse_gaps_from_response("not json at all");
    assert!(gaps.is_empty());
}

#[test]
fn test_parse_gaps_empty_array() {
    let gaps = parse_gaps_from_response("[]");
    assert!(gaps.is_empty());
}

#[test]
fn test_parse_gaps_max_five() {
    let json = (0..10)
        .map(|i| format!(
            r#"{{"description": "gap {}", "target_file": "src/f{}.rs", "gap_type": "missing_function", "priority": 1}}"#,
            i, i
        ))
        .collect::<Vec<_>>()
        .join(",");
    let json = format!("[{}]", json);
    let gaps = parse_gaps_from_response(&json);
    assert_eq!(gaps.len(), 10, "Should cap at 10 gaps");
}

// ═══════════════════════════════════════════════════════════
// PATCH JSON PARSING
// ═══════════════════════════════════════════════════════════

#[test]
fn test_parse_patches_valid() {
    let gaps = vec![SpecGap {
        description: "Add handler".into(),
        target_file: "src/lib.rs".into(),
        gap_type: GapType::MissingFunction,
        priority: 1,
    }];
    let json = r#"[{"target_file": "src/lib.rs", "diff_content": "pub fn version() -> &'static str { \"1.0\" }", "description": "Add version function"}]"#;
    let patches = parse_patches_from_response(json, &gaps);
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].target_file, "src/lib.rs");
    assert!(patches[0].diff_content.contains("version"));
}

#[test]
fn test_parse_patches_invalid() {
    let patches = parse_patches_from_response("broken", &[]);
    assert!(patches.is_empty());
}

#[test]
fn test_parse_patches_max_five() {
    let json = (0..10)
        .map(|i| format!(
            r#"{{"target_file": "src/f{}.rs", "diff_content": "fn f{}() {{}}", "description": "p{}"}}"#,
            i, i, i
        ))
        .collect::<Vec<_>>()
        .join(",");
    let json = format!("[{}]", json);
    let patches = parse_patches_from_response(&json, &[]);
    assert_eq!(patches.len(), 10, "Should cap at 10 patches");
}

// ═══════════════════════════════════════════════════════════
// MCP RESPONSE PARSING
// ═══════════════════════════════════════════════════════════

#[test]
fn test_extract_mcp_text_content_array() {
    let value = serde_json::json!({
        "content": [{"type": "text", "text": "[{\"description\": \"gap\", \"target_file\": \"src/lib.rs\", \"gap_type\": \"missing_function\", \"priority\": 1}]"}]
    });
    let text = extract_mcp_text(&value);
    assert!(text.is_some());
    let gaps = parse_gaps_json_str(&text.unwrap());
    assert_eq!(gaps.len(), 1);
}

#[test]
fn test_extract_mcp_text_string() {
    let value = serde_json::json!("direct string");
    assert_eq!(extract_mcp_text(&value), Some("direct string".into()));
}

#[test]
fn test_extract_mcp_text_none() {
    let value = serde_json::json!({"other": "field"});
    assert!(extract_mcp_text(&value).is_none());
}

// ═══════════════════════════════════════════════════════════
// OPTIONAL FIELD DEFAULTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_parse_gaps_missing_optional_fields() {
    // LLMs often omit gap_type and priority — should default, not drop
    let json = r#"[
        {"description": "Add handler", "target_file": "src/lib.rs"},
        {"description": "Add test", "target_file": "src/test.rs", "gap_type": "missing_test"}
    ]"#;
    let gaps = parse_gaps_from_response(json);
    assert_eq!(gaps.len(), 2, "Gaps with missing optional fields should not be dropped");
    assert!(matches!(gaps[0].gap_type, GapType::MissingFunction)); // default
    assert_eq!(gaps[0].priority, 1); // default
    assert!(matches!(gaps[1].gap_type, GapType::MissingTest));
    assert_eq!(gaps[1].priority, 1); // default when omitted
}

// ═══════════════════════════════════════════════════════════
// MODEL SELECTION
// ═══════════════════════════════════════════════════════════

#[test]
fn test_pick_cheapest_model_anthropic() {
    let config = hydra_model::LlmConfig {
        anthropic_api_key: Some("sk-ant-test".into()),
        openai_api_key: None,
        anthropic_base_url: String::new(),
        openai_base_url: String::new(),
    };
    let (model, provider) = pick_cheapest_model(&config);
    assert!(model.contains("haiku"));
    assert_eq!(provider, "anthropic");
}

#[test]
fn test_pick_cheapest_model_openai() {
    let config = hydra_model::LlmConfig {
        anthropic_api_key: None,
        openai_api_key: Some("sk-test".into()),
        anthropic_base_url: String::new(),
        openai_base_url: String::new(),
    };
    let (model, provider) = pick_cheapest_model(&config);
    assert_eq!(model, "gpt-4o-mini");
    assert_eq!(provider, "openai");
}

#[test]
fn test_pick_cheapest_model_none() {
    let config = hydra_model::LlmConfig {
        anthropic_api_key: None,
        openai_api_key: None,
        anthropic_base_url: String::new(),
        openai_base_url: String::new(),
    };
    let (model, provider) = pick_cheapest_model(&config);
    assert!(model.is_empty());
    assert_eq!(provider, "none");
}
