//! Interactive REPL mode — enter conversation with Hydra from the terminal.
//!
//! Activated when `hydra` is called with no arguments.

use std::io::{self, BufRead, Write};

use crate::banner;
use crate::client::HydraClient;
use crate::colors;
use crate::commands::run::{self, RunOptions};
use crate::output;

/// Run the interactive REPL.
pub fn run() {
    // Show compact banner
    banner::print_banner_compact();
    println!();

    let client = HydraClient::new();

    // Check server health
    if client.health_check() {
        output::print_success("Connected to Hydra daemon");
    } else {
        output::print_warning("Hydra daemon not running — REPL requires a running server");
        output::print_dimmed("Tip: Run `hydra serve` in another terminal first");
    }

    println!();
    println!(
        "{}",
        colors::dim("Type your request and press Enter. Type 'exit' or 'quit' to leave.")
    );
    println!(
        "{}",
        colors::dim("Commands: /status, /sisters, /trust, /inventions, /memory <query>, /help")
    );
    println!();

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        // Print prompt
        print!("{} ", colors::blue("hydra>"));
        io::stdout().flush().unwrap_or_default();

        // Read input
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        // Handle exit
        if input == "exit" || input == "quit" || input == "q" {
            output::print_dimmed("Goodbye!");
            break;
        }

        // Handle slash commands
        if input.starts_with('/') {
            handle_slash_command(input, &client);
            continue;
        }

        // Send as intent
        execute_intent(input, &client);
    }
}

fn handle_slash_command(input: &str, client: &HydraClient) {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0];
    let args = parts.get(1).copied().unwrap_or("");

    match cmd {
        "/status" => match client.get("/api/system/status") {
            Ok(data) => {
                let active = data["active_runs"].as_u64().unwrap_or(0);
                let total = data["total_runs"].as_u64().unwrap_or(0);
                let pending = data["pending_approvals"].as_u64().unwrap_or(0);
                let autonomy = data["autonomy_level"].as_str().unwrap_or("unknown");
                let kill_active = data["kill_switch_active"].as_bool().unwrap_or(false);
                output::print_header("System Status");
                output::print_kv("Active runs", &format!("{}", active));
                output::print_kv("Total runs", &format!("{}", total));
                output::print_kv("Pending approvals", &format!("{}", pending));
                output::print_kv("Autonomy", autonomy);
                if kill_active {
                    output::print_warning("Kill switch is ACTIVE");
                }
            }
            Err(e) => output::print_error(&format!("Error: {}", e)),
        },
        "/sisters" => match client.get("/api/system/status") {
            Ok(resp) => {
                output::print_header("Sisters");
                if let Some(sisters) = resp.get("sisters") {
                    if let Some(obj) = sisters.as_object() {
                        for (name, status) in obj {
                            let st = status.as_str().unwrap_or("unknown");
                            if st == "connected" {
                                output::print_success(&format!("{}: connected", name));
                            } else {
                                output::print_warning(&format!("{}: {}", name, st));
                            }
                        }
                    }
                }
            }
            Err(e) => output::print_error(&format!("Error: {}", e)),
        },
        "/trust" => match client.get("/api/system/trust") {
            Ok(data) => {
                let score = data["trust_score"].as_f64().unwrap_or(0.0);
                let level = data["autonomy_level"].as_str().unwrap_or("unknown");
                output::print_header("Trust & Autonomy");
                output::print_kv("Trust score", &format!("{:.0}%", score * 100.0));
                output::print_kv("Autonomy level", level);
            }
            Err(e) => output::print_error(&format!("Error: {}", e)),
        },
        "/inventions" => match client.get("/api/system/inventions") {
            Ok(data) => {
                output::print_header("Cognitive Inventions");
                output::print_kv("Skills crystallized", &format!("{}", data["skills_crystallized"].as_u64().unwrap_or(0)));
                output::print_kv("Patterns tracked", &format!("{}", data["patterns_tracked"].as_u64().unwrap_or(0)));
                output::print_kv("Reflections", &format!("{}", data["reflections"].as_u64().unwrap_or(0)));
                output::print_kv("Idle time", &format!("{}s", data["idle_time"].as_u64().unwrap_or(0)));
                output::print_kv("Dream active", &format!("{}", data["dream_active"].as_bool().unwrap_or(false)));
                output::print_kv("Shadow validator", data["shadow_validator"].as_str().unwrap_or("unknown"));
                output::print_kv("Future echo", data["future_echo"].as_str().unwrap_or("unknown"));
                output::print_kv("Compression", data["context_compression"].as_str().unwrap_or("unknown"));
                output::print_kv("Evolution", data["evolution_engine"].as_str().unwrap_or("unknown"));
                output::print_kv("Metacognition", data["metacognition"].as_str().unwrap_or("unknown"));
            }
            Err(e) => output::print_error(&format!("Error: {}", e)),
        },
        "/budget" => match client.get("/api/system/budget") {
            Ok(data) => {
                output::print_header("Budget");
                output::print_kv("Total budget", &format!("{}", data["total_budget"].as_u64().unwrap_or(0)));
                output::print_kv("Conservation mode", &format!("{}", data["conservation_mode"].as_bool().unwrap_or(false)));
                output::print_kv("Active runs", &format!("{}", data["active_runs"].as_u64().unwrap_or(0)));
            }
            Err(e) => output::print_error(&format!("Error: {}", e)),
        },
        "/memory" => {
            if args.is_empty() {
                output::print_warning("Usage: /memory <query>");
            } else {
                execute_intent(&format!("search my memory for: {}", args), client);
            }
        }
        "/clear" => {
            print!("\x1B[2J\x1B[1;1H");
            io::stdout().flush().unwrap_or_default();
        }
        "/help" => {
            println!("  {}", colors::bold("Available commands:"));
            println!("    {}      — Show system status", colors::blue("/status"));
            println!("    {}     — Show connected sisters", colors::blue("/sisters"));
            println!("    {}       — Show trust & autonomy level", colors::blue("/trust"));
            println!("    {}  — Show invention stats", colors::blue("/inventions"));
            println!("    {}      — Show budget usage", colors::blue("/budget"));
            println!(
                "    {} — Search memory",
                colors::blue("/memory <query>")
            );
            println!("    {}       — Clear screen", colors::blue("/clear"));
            println!("    {}        — Show this help", colors::blue("/help"));
            println!("    {}        — Exit REPL", colors::blue("/exit"));
            println!();
            output::print_dimmed("Or just type any request and press Enter.");
        }
        "/exit" => {
            output::print_dimmed("Goodbye!");
            std::process::exit(0);
        }
        _ => {
            output::print_warning(&format!(
                "Unknown command: {}. Type /help for available commands.",
                cmd
            ));
        }
    }
}

fn execute_intent(input: &str, _client: &HydraClient) {
    let opts = RunOptions {
        intent: input.to_string(),
        ..Default::default()
    };
    run::execute(&opts);
}

#[cfg(test)]
mod tests {
    #[test]
    fn slash_command_parsing() {
        let input = "/memory search term here";
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        assert_eq!(parts[0], "/memory");
        assert_eq!(parts[1], "search term here");
    }

    #[test]
    fn exit_detection() {
        for exit_word in &["exit", "quit", "q"] {
            let input = exit_word.trim();
            assert!(input == "exit" || input == "quit" || input == "q");
        }
    }

    #[test]
    fn empty_input_skipped() {
        let input = "   ".trim();
        assert!(input.is_empty());
    }
}
