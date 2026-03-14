//! Pre-dispatch handlers for utility sisters (Data, Connect, Workflow).
//!
//! These intercept data/API/workflow requests BEFORE the LLM, calling sisters
//! directly from Rust code. Same pattern as memory_recall — don't ASK the LLM
//! to use a sister, CALL the sister and give the LLM the results.

use tokio::sync::mpsc;
use crate::sisters::SistersHandle;
use super::super::loop_runner::CognitiveUpdate;

/// Detect and handle data operations (parse, schema, quality).
/// Returns true if handled (caller should skip LLM).
pub(crate) async fn handle_data_operation(
    text: &str,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let lower = text.to_lowercase();
    eprintln!("[hydra:pre-dispatch] checking data keywords for: {}", &lower[..lower.len().min(80)]);

    let has_data_keyword = lower.contains("parse") || lower.contains("format")
        || lower.contains("schema") || lower.contains("infer")
        || lower.contains("analyze");
    let has_inline_data = lower.contains("csv") || lower.contains("json")
        || lower.contains("xml") || lower.contains("yaml")
        || lower.contains("tsv") || lower.contains("toml")
        || (text.contains(',') && (text.contains('\n') || text.contains("\\n")));

    eprintln!("[hydra:pre-dispatch] data_keyword={} inline_data={}", has_data_keyword, has_inline_data);

    if !has_data_keyword || !has_inline_data {
        return false;
    }

    let Some(ref sh) = sisters_handle else { return false; };
    let Some(ref data) = sh.data else { return false; };

    let inline = extract_inline_data(text);
    if inline.is_empty() { return false; }

    // Convert literal \n to actual newlines so Data sister parses rows
    let cleaned = inline.replace("\\n", "\n");

    let _ = tx.send(CognitiveUpdate::Phase("Data Analysis".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));
    eprintln!("[hydra:data] Pre-dispatch: calling data_schema_infer with {} chars", cleaned.len());

    let result = data.call_tool("data_schema_infer", serde_json::json!({
        "data": cleaned
    })).await;

    match result {
        Ok(response) => {
            let raw = crate::sisters::connection::extract_text(&response);
            let msg = format_data_response(&raw, &cleaned);
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(),
                content: msg,
                css_class: "message hydra".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            eprintln!("[hydra:data] Pre-dispatch: data_schema_infer succeeded");
            true
        }
        Err(e) => {
            eprintln!("[hydra:data] Pre-dispatch: data_schema_infer failed: {}", e);
            false
        }
    }
}

/// Detect and handle API/URL health checks via Connect sister.
/// Returns true if handled.
///
/// This must fire BEFORE the /health system command matcher.
/// If the input contains a URL, route to Connect sister.
pub(crate) async fn handle_connect_operation(
    text: &str,
    sisters_handle: &Option<SistersHandle>,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) -> bool {
    let lower = text.to_lowercase();
    eprintln!("[hydra:pre-dispatch] checking URL health for: {}", &lower[..lower.len().min(80)]);

    // URL presence is the primary signal — if there's a URL, this is a connect operation
    let has_url = text.contains("http://") || text.contains("https://");
    if !has_url { return false; }

    let Some(ref sh) = sisters_handle else { return false; };
    let Some(ref connect) = sh.connect else {
        eprintln!("[hydra:pre-dispatch] connect sister not available");
        return false;
    };

    let url = extract_url(text);
    if url.is_empty() { return false; }

    let _ = tx.send(CognitiveUpdate::Phase("Connect".into()));
    let _ = tx.send(CognitiveUpdate::IconState("working".into()));
    eprintln!("[hydra:connect] Pre-dispatch: calling connect_api_call for {}", url);

    // Try connect_api_call (GET request) — more general than connect_health
    let result = connect.call_tool("connect_api_call", serde_json::json!({
        "url": url,
        "method": "GET"
    })).await;

    match result {
        Ok(response) => {
            let raw = crate::sisters::connection::extract_text(&response);
            let msg = format_connect_response(&raw, &url);
            let _ = tx.send(CognitiveUpdate::Message {
                role: "hydra".into(), content: msg,
                css_class: "message hydra".into(),
            });
            let _ = tx.send(CognitiveUpdate::ResetIdle);
            eprintln!("[hydra:connect] Pre-dispatch: connect_api_call succeeded");
            true
        }
        Err(e) => {
            eprintln!("[hydra:connect] Pre-dispatch: connect_api_call failed: {}, trying connect_health", e);
            // Fallback to connect_health
            let result2 = connect.call_tool("connect_health", serde_json::json!({
                "url": url
            })).await;
            match result2 {
                Ok(response) => {
                    let raw = crate::sisters::connection::extract_text(&response);
                    let msg = format_connect_response(&raw, &url);
                    let _ = tx.send(CognitiveUpdate::Message {
                        role: "hydra".into(), content: msg,
                        css_class: "message hydra".into(),
                    });
                    let _ = tx.send(CognitiveUpdate::ResetIdle);
                    true
                }
                Err(e2) => {
                    eprintln!("[hydra:connect] Pre-dispatch: both tools failed: {}", e2);
                    false
                }
            }
        }
    }
}

/// Extract inline data from a message (text after ":" or data block).
fn extract_inline_data(text: &str) -> String {
    if let Some(colon_pos) = text.find(':') {
        let after = text[colon_pos + 1..].trim();
        if !after.is_empty() && after.len() > 5 {
            return after.to_string();
        }
    }
    if let Some(start) = text.find('`') {
        if let Some(end) = text[start + 1..].find('`') {
            let inner = &text[start + 1..start + 1 + end];
            if !inner.is_empty() { return inner.to_string(); }
        }
    }
    String::new()
}

/// Extract a URL from text.
fn extract_url(text: &str) -> String {
    for word in text.split_whitespace() {
        let w = word.trim_matches(|c| c == '"' || c == '\'' || c == '<' || c == '>');
        if w.starts_with("http://") || w.starts_with("https://") {
            return w.to_string();
        }
    }
    String::new()
}

/// Format Data sister JSON response for human readability.
fn format_data_response(raw: &str, original_data: &str) -> String {
    // Try to parse as JSON for structured output
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        let fields = v.get("fields").and_then(|f| f.as_u64()).unwrap_or(0);
        let records = v.get("records").and_then(|r| r.as_u64()).unwrap_or(0);

        // Parse the original data ourselves for a friendlier display
        let lines: Vec<&str> = original_data.lines().collect();
        let mut result = String::from("**Data Analysis** \n\n");

        if lines.len() > 1 {
            let headers: Vec<&str> = lines[0].split(',').map(|s| s.trim()).collect();
            result.push_str(&format!("**{} columns** found: {}\n", headers.len(), headers.join(", ")));
            result.push_str(&format!("**{} rows** of data\n\n", lines.len() - 1));

            // Show data as a table
            result.push_str("| ");
            result.push_str(&headers.join(" | "));
            result.push_str(" |\n|");
            for _ in &headers { result.push_str("---|"); }
            result.push('\n');

            for line in &lines[1..] {
                let vals: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                result.push_str("| ");
                result.push_str(&vals.join(" | "));
                result.push_str(" |\n");
            }
        } else {
            result.push_str(&format!("{} fields detected, {} records parsed.\n", fields, records));
        }

        if let Some(schema_id) = v.get("schema_id").and_then(|s| s.as_str()) {
            result.push_str(&format!("\n*Schema ID: {}*", &schema_id[..schema_id.len().min(8)]));
        }

        result
    } else {
        format!("**Data Analysis** \n\n{}", raw)
    }
}

/// Format Connect sister response for human readability.
fn format_connect_response(raw: &str, url: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        let status = v.get("status").or(v.get("status_code"))
            .and_then(|s| s.as_u64())
            .map(|s| format!("{}", s))
            .unwrap_or_else(|| "unknown".into());

        let healthy = status.starts_with('2');
        let icon = if healthy { "🟢" } else { "🔴" };

        let mut result = format!("**Health Check** \n\n");
        result.push_str(&format!("{} **{}** — HTTP {}\n", icon, url, status));

        if let Some(body) = v.get("body").and_then(|b| b.as_str()) {
            if body.len() < 500 {
                result.push_str(&format!("\n```json\n{}\n```", body));
            } else {
                result.push_str(&format!("\nResponse: {} bytes", body.len()));
            }
        }

        result
    } else {
        format!("**Health Check** \n\n`{}` → {}", url, raw)
    }
}
