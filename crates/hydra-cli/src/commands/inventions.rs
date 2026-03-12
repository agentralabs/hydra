use crate::client::HydraClient;
use crate::colors;
use crate::output;

pub fn execute() {
    output::print_header("Cognitive Inventions");
    println!();

    let client = HydraClient::new();
    match client.get("/api/system/inventions") {
        Ok(data) => {
            let skills = data["skills_crystallized"].as_u64().unwrap_or(0);
            let patterns = data["patterns_tracked"].as_u64().unwrap_or(0);
            let reflections = data["reflections"].as_u64().unwrap_or(0);
            let idle = data["idle_time"].as_u64().unwrap_or(0);
            let dream_active = data["dream_active"].as_bool().unwrap_or(false);

            // Stats table
            let headers = &["Invention", "Status", "Count"];
            let rows = vec![
                vec![
                    "\u{1f4a4} Dream State".to_string(),
                    if dream_active { colors::green("Active") } else { colors::dim("Idle") },
                    format!("{}s idle", idle),
                ],
                vec![
                    "\u{1f47b} Shadow Self".to_string(),
                    status_from(&data, "shadow_validator"),
                    "—".to_string(),
                ],
                vec![
                    "\u{1f52e} Future Echo".to_string(),
                    status_from(&data, "future_echo"),
                    "—".to_string(),
                ],
                vec![
                    "\u{1f48e} Crystallization".to_string(),
                    status_from(&data, "crystallization"),
                    format!("{} skills", skills),
                ],
                vec![
                    "\u{1f9ec} Pattern Mutation".to_string(),
                    status_from(&data, "pattern_mutation"),
                    format!("{} patterns", patterns),
                ],
                vec![
                    "\u{1f300} Evolution Engine".to_string(),
                    status_from(&data, "evolution_engine"),
                    "—".to_string(),
                ],
                vec![
                    "\u{1f9e0} Metacognition".to_string(),
                    status_from(&data, "metacognition"),
                    format!("{} reflections", reflections),
                ],
                vec![
                    "\u{1f5dc}\u{fe0f}  Compression".to_string(),
                    status_from(&data, "context_compression"),
                    "—".to_string(),
                ],
                vec![
                    "\u{1f50d} Semantic Dedup".to_string(),
                    status_from(&data, "semantic_dedup"),
                    "—".to_string(),
                ],
                vec![
                    "\u{23f0} Temporal Memory".to_string(),
                    status_from(&data, "temporal_memory"),
                    "—".to_string(),
                ],
            ];

            output::print_table(headers, &rows);
            println!();

            output::print_info(&format!(
                "Total: {} skills, {} patterns, {} reflections",
                skills, patterns, reflections
            ));
        }
        Err(e) => {
            output::print_error(&format!("Cannot reach server: {}", e));
            output::print_dimmed("Start the server with: hydra serve");
        }
    }
}

fn status_from(data: &serde_json::Value, key: &str) -> String {
    match data[key].as_str() {
        Some("active") => colors::green("Active"),
        Some(s) => s.to_string(),
        None => colors::dim("Unknown"),
    }
}
