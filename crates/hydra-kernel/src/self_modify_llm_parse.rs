//! JSON parsing helpers for LLM gap analysis and patch generation responses.

use crate::self_modify::{GapType, Patch, SpecGap};

/// Strip markdown fences from LLM response.
pub fn strip_fences(response: &str) -> &str {
    let trimmed = response.trim();
    let inner = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .unwrap_or(trimmed);
    inner.trim()
}

/// Parse gap analysis from LLM text response.
pub fn parse_gaps_from_response(response: &str) -> Vec<SpecGap> {
    let json_str = strip_fences(response);
    parse_gaps_json_str(json_str)
}

/// Parse gaps from a Forge sister JSON response.
pub fn parse_gaps_from_json(value: &serde_json::Value) -> Option<Vec<SpecGap>> {
    let text = extract_mcp_text(value)?;
    Some(parse_gaps_json_str(&text))
}

pub fn parse_gaps_json_str(json_str: &str) -> Vec<SpecGap> {
    let arr: Vec<serde_json::Value> = match serde_json::from_str(json_str) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };

    arr.into_iter()
        .filter_map(|v| {
            let desc = v.get("description")?.as_str()?.to_string();
            let file = v.get("target_file")?.as_str()?.to_string();
            let gap_str = v.get("gap_type")
                .and_then(|g| g.as_str())
                .unwrap_or("missing_function");
            let priority = v.get("priority")
                .and_then(|p| p.as_u64())
                .unwrap_or(1) as u8;

            let gap_type = match gap_str {
                "missing_module" => GapType::MissingModule,
                "missing_test" => GapType::MissingTest,
                "missing_integration" => GapType::MissingIntegration,
                "incomplete_implementation" => GapType::IncompleteImplementation,
                _ => GapType::MissingFunction,
            };

            Some(SpecGap { description: desc, target_file: file, gap_type, priority })
        })
        .take(10)
        .collect()
}

/// Parse patches from LLM text response.
/// Handles common LLM JSON issues: literal newlines in strings, truncated arrays.
pub fn parse_patches_from_response(response: &str, gaps: &[SpecGap]) -> Vec<Patch> {
    let json_str = strip_fences(response);
    let patches = parse_patches_json_str(json_str, gaps);
    if !patches.is_empty() {
        return patches;
    }
    // LLMs often put literal newlines in "diff_content" -- repair by escaping them
    let repaired = repair_json_newlines(json_str);
    parse_patches_json_str(&repaired, gaps)
}

/// Repair JSON with literal newlines inside string values.
fn repair_json_newlines(input: &str) -> String {
    let mut result = String::with_capacity(input.len() + 100);
    let mut in_string = false;
    let mut prev_backslash = false;
    for ch in input.chars() {
        if ch == '"' && !prev_backslash {
            in_string = !in_string;
            result.push(ch);
        } else if ch == '\n' && in_string {
            result.push_str("\\n");
        } else if ch == '\r' && in_string {
            // skip carriage returns
        } else if ch == '\t' && in_string {
            result.push_str("\\t");
        } else {
            result.push(ch);
        }
        prev_backslash = ch == '\\' && !prev_backslash;
    }
    result
}

/// Parse patches from a Forge sister JSON response.
pub fn parse_patches_from_json(value: &serde_json::Value, gaps: &[SpecGap]) -> Option<Vec<Patch>> {
    let text = extract_mcp_text(value)?;
    Some(parse_patches_json_str(&text, gaps))
}

fn parse_patches_json_str(json_str: &str, gaps: &[SpecGap]) -> Vec<Patch> {
    let arr: Vec<serde_json::Value> = match serde_json::from_str(json_str) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };

    arr.into_iter()
        .enumerate()
        .filter_map(|(i, v)| {
            let target = v.get("target_file")?.as_str()?.to_string();
            let diff = v.get("diff_content")?.as_str()?.to_string();
            let desc = v.get("description")?.as_str().unwrap_or("patch").to_string();

            let gap = gaps.get(i).cloned().unwrap_or(SpecGap {
                description: desc.clone(),
                target_file: target.clone(),
                gap_type: GapType::MissingFunction,
                priority: 1,
            });

            Some(Patch {
                target_file: target,
                gap,
                diff_content: diff,
                description: desc,
                touches_critical: false,
            })
        })
        .take(10)
        .collect()
}

/// Extract text from MCP tool response format.
pub fn extract_mcp_text(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    value
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}
