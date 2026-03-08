use crate::client::HydraClient;
use crate::colors;
use crate::output;

struct SisterInfo {
    name: &'static str,
    key: &'static str,
    topic: &'static str,
    connected: bool,
}

fn all_sisters() -> Vec<SisterInfo> {
    vec![
        SisterInfo { name: "AgenticMemory", key: "memory", topic: "Memory", connected: true },
        SisterInfo { name: "AgenticVision", key: "vision", topic: "Vision", connected: true },
        SisterInfo { name: "AgenticCodebase", key: "codebase", topic: "Codebase", connected: true },
        SisterInfo { name: "AgenticIdentity", key: "identity", topic: "Identity", connected: true },
        SisterInfo { name: "AgenticTime", key: "time", topic: "Time", connected: false },
        SisterInfo { name: "AgenticContract", key: "contract", topic: "Contract", connected: false },
        SisterInfo { name: "AgenticComm", key: "comm", topic: "Communication", connected: false },
        SisterInfo { name: "AgenticPlanning", key: "planning", topic: "Planning", connected: false },
        SisterInfo { name: "AgenticCognition", key: "cognition", topic: "Cognition", connected: true },
        SisterInfo { name: "AgenticReality", key: "reality", topic: "Reality", connected: false },
        SisterInfo { name: "AgenticForge", key: "forge", topic: "Blueprint", connected: false },
        SisterInfo { name: "AgenticAegis", key: "aegis", topic: "Validation", connected: false },
        SisterInfo { name: "AgenticVeritas", key: "veritas", topic: "Truth", connected: false },
        SisterInfo { name: "AgenticEvolve", key: "evolve", topic: "Patterns", connected: false },
    ]
}

pub fn status() {
    let client = HydraClient::new();
    if client.health_check() {
        output::print_info("Server connected");
    } else {
        output::print_warning("Hydra server unreachable. Showing local sister data.");
    }

    output::print_header("Sisters");
    println!();

    let sisters = all_sisters();
    let connected_count = sisters.iter().filter(|s| s.connected).count();
    let total = sisters.len();

    output::print_info(&format!("{}/{} connected", connected_count, total));
    println!();

    let headers = &["Sister", "Key", "Topic", "Status"];
    let rows: Vec<Vec<String>> = sisters
        .iter()
        .map(|s| {
            let status_text = if s.connected {
                format!("{} {}", colors::green("\u{2713}"), "connected")
            } else {
                format!("{} {}", colors::dim("\u{25cb}"), "disconnected")
            };
            vec![
                s.name.to_string(),
                s.key.to_string(),
                s.topic.to_string(),
                status_text,
            ]
        })
        .collect();
    output::print_table(headers, &rows);
    println!();
}

pub fn connect(name: &str) {
    let sisters = all_sisters();
    if !sisters.iter().any(|s| s.key == name) {
        output::print_error(&format!("Unknown sister: {}", name));
        output::print_info("Use 'hydra sisters' to list available sisters");
        return;
    }

    let client = HydraClient::new();
    match client.post(
        &format!("/api/sisters/{}/connect", name),
        &serde_json::json!({}),
    ) {
        Ok(_) => {
            output::print_success(&format!("Connected to sister: {}", colors::bold(name)));
        }
        Err(_) => {
            output::print_error(&format!(
                "Could not reach Hydra server. Sister '{}' was NOT connected.",
                name
            ));
        }
    }
}

pub fn disconnect(name: &str) {
    let sisters = all_sisters();
    if !sisters.iter().any(|s| s.key == name) {
        output::print_error(&format!("Unknown sister: {}", name));
        output::print_info("Use 'hydra sisters' to list available sisters");
        return;
    }

    let client = HydraClient::new();
    match client.post(
        &format!("/api/sisters/{}/disconnect", name),
        &serde_json::json!({}),
    ) {
        Ok(_) => {
            output::print_warning(&format!("Disconnected from sister: {}", colors::bold(name)));
        }
        Err(_) => {
            output::print_error(&format!(
                "Could not reach Hydra server. Sister '{}' was NOT disconnected.",
                name
            ));
        }
    }
}
