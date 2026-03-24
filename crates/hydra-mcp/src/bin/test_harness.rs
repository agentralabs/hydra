//! hydra-mcp test harness — validates all MCP components.
//! Run: cargo run -p hydra-mcp --bin mcp-harness

use hydra_mcp::client::McpClient;
use hydra_mcp::protocol::{JsonRpcRequest, JsonRpcResponse, ToolResult};
use hydra_mcp::server::McpServer;
use hydra_mcp::tools;
use std::sync::Arc;

struct Test {
    name: &'static str,
    passed: bool,
    notes: String,
}

fn main() {
    println!("=== hydra-mcp test harness ===");
    println!("Phase: HANDS  Layer: MCP Integration\n");

    let mut tests: Vec<Test> = Vec::new();

    // ── Protocol Tests ──
    {
        let req = JsonRpcRequest::new(1, "tools/list");
        let json = serde_json::to_string(&req).unwrap();
        let back: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        tests.push(Test {
            name: "jsonrpc_request_roundtrip",
            passed: back.method == "tools/list" && back.jsonrpc == "2.0",
            notes: format!("method={}", back.method),
        });
    }
    {
        let resp = JsonRpcResponse::success(
            serde_json::json!(1),
            serde_json::json!({"status": "ok"}),
        );
        tests.push(Test {
            name: "jsonrpc_response_success",
            passed: !resp.is_error() && resp.result.is_some(),
            notes: "success response created".into(),
        });
    }
    {
        let resp = JsonRpcResponse::error(
            serde_json::json!(1),
            -32803,
            "Tool not found",
        );
        tests.push(Test {
            name: "jsonrpc_response_error",
            passed: resp.is_error() && resp.error.as_ref().unwrap().code == -32803,
            notes: format!("code={}", resp.error.unwrap().code),
        });
    }

    // ── Tool Schema Tests ──
    {
        let all_tools = tools::hydra_tools();
        tests.push(Test {
            name: "tool_count",
            passed: all_tools.len() == 8,
            notes: format!("{} tools", all_tools.len()),
        });
    }
    {
        let all_tools = tools::hydra_tools();
        let no_trailing_periods = all_tools.iter().all(|t| !t.description.ends_with('.'));
        tests.push(Test {
            name: "mcp_quality_no_trailing_periods",
            passed: no_trailing_periods,
            notes: "MCP Quality Standard compliance".into(),
        });
    }
    {
        let found = tools::find_tool("hydra_query");
        tests.push(Test {
            name: "find_tool_existing",
            passed: found.is_some(),
            notes: "hydra_query found".into(),
        });
    }
    {
        let found = tools::find_tool("nonexistent");
        tests.push(Test {
            name: "find_tool_missing",
            passed: found.is_none(),
            notes: "correctly returns None".into(),
        });
    }

    // ── Server Tests ──
    {
        let mut server = McpServer::new();
        let req = JsonRpcRequest::new(1, "initialize");
        let resp = server.handle_request(&req);
        tests.push(Test {
            name: "server_initialize",
            passed: !resp.is_error() && server.is_initialized(),
            notes: "server initialized".into(),
        });
    }
    {
        let mut server = McpServer::new();
        let req = JsonRpcRequest::new(1, "tools/list");
        let resp = server.handle_request(&req);
        let count = resp.result.as_ref()
            .and_then(|r| r.get("tools"))
            .and_then(|t| t.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        tests.push(Test {
            name: "server_tools_list",
            passed: count == 8,
            notes: format!("{count} tools listed"),
        });
    }
    {
        let mut server = McpServer::new();
        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({
                "name": "nonexistent_tool",
                "arguments": {}
            }));
        let resp = server.handle_request(&req);
        let is_tool_not_found = resp.error.as_ref().map(|e| e.code) == Some(-32803);
        tests.push(Test {
            name: "server_unknown_tool_error_code",
            passed: is_tool_not_found,
            notes: "error code -32803 (TOOL_NOT_FOUND)".into(),
        });
    }
    {
        let mut server = McpServer::new();
        server.set_handler(Arc::new(|name, _args| {
            ToolResult::text(format!("OK: {name}"))
        }));
        let req = JsonRpcRequest::new(1, "tools/call")
            .with_params(serde_json::json!({
                "name": "hydra_status",
                "arguments": {}
            }));
        let resp = server.handle_request(&req);
        tests.push(Test {
            name: "server_tool_call_with_handler",
            passed: !resp.is_error(),
            notes: "handler executed successfully".into(),
        });
    }

    // ── ToolResult Tests ──
    {
        let result = ToolResult::text("hello");
        tests.push(Test {
            name: "tool_result_text",
            passed: !result.is_error && !result.content.is_empty(),
            notes: "text result created".into(),
        });
    }
    {
        let result = ToolResult::error("failed");
        tests.push(Test {
            name: "tool_result_error",
            passed: result.is_error,
            notes: "error result created".into(),
        });
    }

    // ── Client Tests ──
    {
        let client = McpClient::new();
        tests.push(Test {
            name: "client_starts_uninitialized",
            passed: !client.is_initialized() && client.cached_tools().is_empty(),
            notes: "clean initial state".into(),
        });
    }

    // ── Results ──
    println!();
    let mut passed = 0;
    let mut failed = 0;
    for t in &tests {
        let status = if t.passed { "PASS" } else { "FAIL" };
        println!("  [{status}] {} — {}", t.name, t.notes);
        if t.passed { passed += 1; } else { failed += 1; }
    }

    println!("\n=== Results: {passed}/{} passed, {failed} failed ===", tests.len());
    if failed > 0 { std::process::exit(1); }
    println!("Phase HANDS — MCP Integration: COMPLETE");
}
