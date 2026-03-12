//! Sister repair diagnosis helpers — protocol mismatch detection and failure reporting.

use tokio::sync::mpsc;

use super::super::super::loop_runner::CognitiveUpdate;

/// Diagnose protocol mismatches in sister source code (Attempt 6).
pub(super) async fn diagnose_protocol_mismatch(
    name: &str,
    bin_name: &str,
    name_lower: &str,
    workspace_root: &str,
    mcp_crate: &str,
    attempts: &[(String, String)],
    report: &mut String,
    tx: &mpsc::UnboundedSender<CognitiveUpdate>,
) {
    let all_errors_so_far = attempts.iter().map(|(_, e)| e.as_str()).collect::<Vec<_>>().join(" ");
    let is_protocol_issue = all_errors_so_far.contains("Content-Length")
        || all_errors_so_far.contains("expected value at line 1 column 1");

    if !is_protocol_issue {
        return;
    }

    report.push_str("**Attempt 6:** Diagnosing protocol mismatch in source code...\n");
    let _ = tx.send(CognitiveUpdate::Message {
        role: "hydra".into(),
        content: report.clone(),
        css_class: "message hydra diagnostics".into(),
    });

    let main_rs = format!("{}/src/main.rs", mcp_crate);
    let broken_src = tokio::fs::read_to_string(&main_rs).await.ok();

    let working_main = format!("{}/agentic-memory/crates/agentic-memory-mcp/src/main.rs", workspace_root);
    let working_src = tokio::fs::read_to_string(&working_main).await.ok();

    let broken_cargo = format!("{}/Cargo.toml", mcp_crate);
    let broken_deps = tokio::fs::read_to_string(&broken_cargo).await.ok();
    let working_cargo = format!("{}/agentic-memory/crates/agentic-memory-mcp/Cargo.toml", workspace_root);
    let working_deps = tokio::fs::read_to_string(&working_cargo).await.ok();

    report.push_str("\n**Source Code Diagnosis:**\n\n");

    if let (Some(ref broken), Some(ref working)) = (&broken_src, &working_src) {
        let broken_has_http = broken.contains("Content-Length")
            || broken.contains("content_length")
            || broken.contains("http_transport")
            || broken.contains("HttpTransport")
            || broken.contains("lsp_transport");
        let working_has_http = working.contains("Content-Length")
            || working.contains("content_length")
            || working.contains("http_transport")
            || working.contains("HttpTransport");

        if broken_has_http && !working_has_http {
            report.push_str(&format!(
                "`{}` uses HTTP/LSP framing (Content-Length headers).\n\
                 Working sister (Memory) uses raw JSON-RPC over stdio.\n\
                 **Fix needed:** Change transport in `{}`\n\n",
                bin_name, main_rs
            ));
        }

        let broken_stdio = broken.contains("StdioTransport")
            || broken.contains("stdio_transport")
            || broken.contains("stdin") && broken.contains("stdout");
        let working_stdio = working.contains("StdioTransport")
            || working.contains("stdio_transport")
            || working.contains("stdin") && working.contains("stdout");

        if working_stdio && !broken_stdio {
            report.push_str(&format!(
                "Working sister uses stdio transport. `{}` does NOT.\n\
                 The sister's MCP server needs to be configured for stdio transport.\n\n",
                bin_name
            ));
        }

        let broken_transport_lines: Vec<&str> = broken.lines()
            .filter(|l| {
                let lower = l.to_lowercase();
                lower.contains("transport") || lower.contains("serve")
                    || lower.contains("stdin") || lower.contains("stdout")
                    || lower.contains("content_length") || lower.contains("content-length")
            })
            .take(10)
            .collect();
        if !broken_transport_lines.is_empty() {
            report.push_str(&format!("Relevant lines in `{}`:\n```rust\n", main_rs));
            for line in &broken_transport_lines {
                report.push_str(&format!("{}\n", line.trim()));
            }
            report.push_str("```\n\n");
        }

        let working_transport_lines: Vec<&str> = working.lines()
            .filter(|l| {
                let lower = l.to_lowercase();
                lower.contains("transport") || lower.contains("serve")
                    || lower.contains("stdin") || lower.contains("stdout")
            })
            .take(10)
            .collect();
        if !working_transport_lines.is_empty() {
            report.push_str(&format!("Working sister (Memory) uses:\n```rust\n"));
            for line in &working_transport_lines {
                report.push_str(&format!("{}\n", line.trim()));
            }
            report.push_str("```\n\n");
        }
    } else {
        if broken_src.is_none() {
            report.push_str(&format!("Could not read `{}`\n", main_rs));
        }
    }

    if let (Some(ref broken_d), Some(ref working_d)) = (&broken_deps, &working_deps) {
        let extract_mcp_deps = |toml: &str| -> Vec<String> {
            toml.lines()
                .filter(|l| l.contains("mcp") || l.contains("transport") || l.contains("jsonrpc"))
                .map(|l| l.trim().to_string())
                .collect()
        };
        let broken_mcp = extract_mcp_deps(broken_d);
        let working_mcp = extract_mcp_deps(working_d);

        if broken_mcp != working_mcp {
            report.push_str("**Dependency differences:**\n");
            if !broken_mcp.is_empty() {
                report.push_str(&format!("  {} uses: {}\n", bin_name, broken_mcp.join(", ")));
            }
            if !working_mcp.is_empty() {
                report.push_str(&format!("  Memory uses: {}\n", working_mcp.join(", ")));
            }
            report.push('\n');
        }
    }
}

/// Emit failure report when all repair attempts fail.
pub(super) fn emit_failure_report(
    name: &str,
    name_lower: &str,
    bin_name: &str,
    attempts: &[(String, String)],
    report: &mut String,
) {
    report.push_str(&format!("\n**Tried {} approaches for {} — all failed.**\n\n", attempts.len(), name));
    for (i, (approach, error)) in attempts.iter().enumerate() {
        let short_err = if error.len() > 100 { &error[..100] } else { error.as_str() };
        report.push_str(&format!("{}. **{}** — {}\n", i + 1, approach, short_err));
    }

    // Root cause summary
    let all_errors = attempts.iter().map(|(_, e)| e.as_str()).collect::<Vec<_>>().join(" ");
    if all_errors.contains("Content-Length") || all_errors.contains("expected value at line 1 column 1")
        || all_errors.contains("Protocol mismatch") {
        report.push_str(&format!(
            "\n**Root blocker:** `{}` outputs HTTP-framed protocol (Content-Length headers) \
             but Hydra expects raw JSON-RPC over stdio. The fix requires changing the MCP transport \
             configuration in `agentic-{}/crates/agentic-{}-mcp/src/main.rs` to match the working sisters.\n\n",
            bin_name, name_lower, name_lower
        ));
    } else if all_errors.contains("File format error") || all_errors.contains("Unknown entity") {
        report.push_str("\n**Root blocker:** Persistent database corruption that survived backup+recreate.\n\n");
    } else if all_errors.contains("No such file") {
        report.push_str("\n**Root blocker:** Binary not installed and repo not available for rebuild.\n\n");
    } else {
        report.push_str(&format!("\n**Root blocker:** Binary crashes on startup. \
            The error is not a known pattern.\n\n"));
    }
}
