//! Serve command — start Hydra in server mode (HTTP + WebSocket API)

use crate::output;

pub fn execute(port: u16, host: &str) {
    output::print_header("Hydra Server");
    output::print_kv("Host", host);
    output::print_kv("Port", &port.to_string());
    output::print_kv("API", &format!("http://{}:{}/api/v1", host, port));
    output::print_kv("WebSocket", &format!("ws://{}:{}/ws", host, port));
    output::print_kv("SSE", &format!("http://{}:{}/events", host, port));
    println!();
    output::print_info("Starting Hydra server...");
    output::print_kv("Status", "Server mode not yet fully wired — use hydra-server crate directly");
    output::print_info("The server provides:");
    println!("  POST /api/v1/runs       — Create a new run");
    println!("  GET  /api/v1/runs       — List runs");
    println!("  GET  /api/v1/runs/:id   — Get run details");
    println!("  POST /api/v1/approve    — Approve pending action");
    println!("  POST /api/v1/deny       — Deny pending action");
    println!("  GET  /api/v1/sisters    — List sister status");
    println!("  GET  /api/v1/health     — Health check");
    println!("  WS   /ws                — Real-time events");
    println!("  SSE  /events            — Server-sent events");
}
