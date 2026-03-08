use crate::client::HydraClient;
use crate::colors;
use crate::output;

pub fn status_icon(status: &str) -> &'static str {
    match status {
        "running" => "\u{25c9}",  // ◉
        "complete" => "\u{2713}", // ✓
        "failed" => "\u{2717}",   // ✗
        "pending" => "\u{25cb}",  // ○
        "frozen" => "\u{2744}",   // ❄
        "killed" => "\u{2620}",   // ☠
        _ => "\u{2022}",          // •
    }
}

fn status_color(status: &str, text: &str) -> String {
    match status {
        "running" => colors::blue(text),
        "complete" => colors::green(text),
        "failed" | "killed" => colors::red(text),
        "pending" | "frozen" => colors::yellow(text),
        _ => colors::dim(text),
    }
}

pub fn execute(run_id: Option<&str>) {
    if let Some(id) = run_id {
        show_run_detail(id);
    } else {
        show_overview();
    }
}

fn show_run_detail(run_id: &str) {
    let client = HydraClient::new();
    match client.get(&format!("/api/tasks/{}", run_id)) {
        Ok(data) => {
            output::print_header(&format!("Run: {}", run_id));
            println!();

            let status = data["status"].as_str().unwrap_or("unknown");
            let intent = data["intent"].as_str().unwrap_or("unknown");
            let started = data["started"].as_str().unwrap_or("unknown");
            let duration = data["duration"].as_str().unwrap_or("unknown");
            let phase = data["phase"].as_str().unwrap_or("unknown");
            let tokens = data["tokens"].as_u64().unwrap_or(0);
            let sisters = data["sisters"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "none".to_string());

            let headers = &["Field", "Value"];
            let rows = vec![
                vec![
                    "Status".to_string(),
                    format!("{} {}", status_icon(status), status),
                ],
                vec!["Intent".to_string(), intent.to_string()],
                vec!["Started".to_string(), started.to_string()],
                vec!["Duration".to_string(), duration.to_string()],
                vec!["Phase".to_string(), phase.to_string()],
                vec!["Tokens".to_string(), output::format_tokens(tokens)],
                vec!["Sisters".to_string(), sisters],
            ];
            output::print_table(headers, &rows);
            println!();
            output::print_info("Use 'hydra inspect' for full details");
        }
        Err(_) => {
            show_run_detail_stub(run_id);
        }
    }
}

fn show_run_detail_stub(run_id: &str) {
    output::print_error(&format!(
        "Server unreachable. Cannot fetch status for run '{}'.",
        run_id
    ));
}

fn show_overview() {
    let client = HydraClient::new();
    match client.get("/api/tasks") {
        Ok(data) => {
            output::print_header("Hydra Status");
            println!();

            // Active runs
            let tasks = data.as_array();
            let active: Vec<&serde_json::Value> = tasks
                .map(|arr| {
                    arr.iter()
                        .filter(|t| t["status"].as_str() == Some("running"))
                        .collect()
                })
                .unwrap_or_default();

            println!(
                "  {} {}",
                colors::bold("Active Runs"),
                colors::dim(&format!("({})", active.len()))
            );
            if active.is_empty() {
                output::print_dimmed("  No active runs");
            } else {
                for task in &active {
                    let id = task["id"].as_str().unwrap_or("unknown");
                    let intent = task["intent"].as_str().unwrap_or("unknown");
                    println!(
                        "    {} {} {} {}",
                        status_icon("running"),
                        colors::blue(id),
                        colors::dim("|"),
                        intent
                    );
                }
            }
            println!();

            // Pending approvals
            let pending: Vec<&serde_json::Value> = tasks
                .map(|arr| {
                    arr.iter()
                        .filter(|t| t["status"].as_str() == Some("pending"))
                        .collect()
                })
                .unwrap_or_default();

            println!(
                "  {} {}",
                colors::bold("Pending Approvals"),
                colors::dim(&format!("({})", pending.len()))
            );
            if pending.is_empty() {
                output::print_dimmed("  No pending approvals");
            } else {
                for task in &pending {
                    let id = task["id"].as_str().unwrap_or("unknown");
                    let intent = task["intent"].as_str().unwrap_or("unknown");
                    println!(
                        "    {} {} {} {}",
                        colors::yellow("\u{25cb}"),
                        colors::yellow(id),
                        colors::dim("|"),
                        intent
                    );
                }
            }
            println!();

            // Today's stats
            let all_tasks = tasks.map(|arr| arr.len()).unwrap_or(0);
            let completed: usize = tasks
                .map(|arr| {
                    arr.iter()
                        .filter(|t| t["status"].as_str() == Some("complete"))
                        .count()
                })
                .unwrap_or(0);
            let failed: usize = tasks
                .map(|arr| {
                    arr.iter()
                        .filter(|t| t["status"].as_str() == Some("failed"))
                        .count()
                })
                .unwrap_or(0);
            let total_tokens: u64 = tasks
                .map(|arr| arr.iter().filter_map(|t| t["tokens"].as_u64()).sum())
                .unwrap_or(0);

            println!("  {}", colors::bold("Today"));
            let stats_headers = &["Metric", "Value"];
            let stats_rows = vec![
                vec!["Runs completed".to_string(), completed.to_string()],
                vec!["Runs failed".to_string(), failed.to_string()],
                vec![
                    "Total tokens".to_string(),
                    output::format_tokens(total_tokens),
                ],
                vec!["Total runs".to_string(), all_tasks.to_string()],
            ];
            output::print_table(stats_headers, &stats_rows);
            println!();
        }
        Err(_) => {
            show_overview_stub();
        }
    }
}

fn show_overview_stub() {
    output::print_error("Server offline — no live data available.");
}
