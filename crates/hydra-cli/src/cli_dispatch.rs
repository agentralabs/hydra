use crate::cli_subcommands::*;
use crate::commands;
use crate::commands::run::RunOptions;
use crate::output;

/// `hydra mcp add|list|remove` — MCP server management (spec §10, §12).
fn dispatch_mcp(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
    match sub {
        "list" | "" => {
            output::print_header("MCP Servers");
            output::print_info("Built-in (Sisters):");
            let sisters = [
                "Memory", "Identity", "Codebase", "Vision", "Comm", "Contract",
                "Time", "Planning", "Cognition", "Reality", "Veritas", "Aegis",
                "Evolve", "Forge",
            ];
            for name in &sisters {
                println!("  ● {}  (sister)", name);
            }
            let home = std::env::var("HOME").unwrap_or_default();
            let cfg = format!("{}/.hydra/mcp-servers.json", home);
            if let Ok(content) = std::fs::read_to_string(&cfg) {
                output::print_info("\nUser-configured:");
                for line in content.lines().take(20) { println!("  {}", line); }
            } else {
                output::print_info("\nNo user-configured MCP servers.");
                output::print_info("Add one with: hydra mcp add <name> -- <command>");
            }
        }
        "add" => {
            let mut name = String::new();
            let mut transport = "stdio";
            let mut cmd_parts: Vec<String> = Vec::new();
            let mut i = 3;
            let mut past_sep = false;
            while i < args.len() {
                match args[i].as_str() {
                    "--transport" => { i += 1; if i < args.len() { transport = if args[i] == "http" { "http" } else { "stdio" }; } }
                    "--" => past_sep = true,
                    _ if name.is_empty() && !past_sep => name = args[i].clone(),
                    _ => cmd_parts.push(args[i].clone()),
                }
                i += 1;
            }
            if name.is_empty() {
                output::print_error("Usage: hydra mcp add <name> -- <command>");
                return;
            }
            let command = cmd_parts.join(" ");
            output::print_success(&format!("Added MCP server '{}' (transport: {}, cmd: {})",
                name, transport, if command.is_empty() { "<none>" } else { &command }));
            let home = std::env::var("HOME").unwrap_or_default();
            let cfg = format!("{}/.hydra/mcp-servers.json", home);
            let _ = std::fs::create_dir_all(format!("{}/.hydra", home));
            let mut servers: serde_json::Value = std::fs::read_to_string(&cfg)
                .ok().and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::json!({}));
            servers[&name] = serde_json::json!({ "transport": transport, "command": command });
            let _ = std::fs::write(&cfg, serde_json::to_string_pretty(&servers).unwrap_or_default());
        }
        "remove" => {
            if let Some(name) = args.get(3) {
                let home = std::env::var("HOME").unwrap_or_default();
                let cfg = format!("{}/.hydra/mcp-servers.json", home);
                if let Ok(content) = std::fs::read_to_string(&cfg) {
                    if let Ok(mut servers) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(obj) = servers.as_object_mut() {
                            if obj.remove(name.as_str()).is_some() {
                                let _ = std::fs::write(&cfg, serde_json::to_string_pretty(&servers).unwrap_or_default());
                                output::print_success(&format!("Removed MCP server '{}'", name));
                            } else {
                                output::print_error(&format!("MCP server '{}' not found", name));
                            }
                        }
                    }
                } else {
                    output::print_error(&format!("MCP server '{}' not found", name));
                }
            } else {
                output::print_error("Usage: hydra mcp remove <name>");
            }
        }
        _ => {
            output::print_error(&format!("Unknown mcp subcommand: {}", sub));
            output::print_info("Subcommands: add, list, remove");
        }
    }
}

/// Dispatch CLI subcommands based on args[1..].
pub fn dispatch(args: &[String]) {
    match args[1].as_str() {
        "run" => commands::run::parse_and_execute(&args[2..]),
        "status" => {
            let run_id = args.get(2).map(|s| s.as_str());
            commands::status::execute(run_id);
        }
        "approve" => {
            if let Some(id) = args.get(2) {
                commands::approval::approve(id);
            } else {
                output::print_error("Usage: hydra approve <run_id>");
            }
        }
        "deny" => {
            if let Some(id) = args.get(2) {
                let reason = args.get(3).map(|s| s.as_str());
                commands::approval::deny(id, reason);
            } else {
                output::print_error("Usage: hydra deny <run_id> [reason]");
            }
        }
        "freeze" => {
            let run_id = args.get(2).map(|s| s.as_str());
            commands::control::freeze(run_id);
        }
        "resume" => {
            if let Some(id) = args.get(2) {
                commands::control::resume(id);
            } else {
                output::print_error("Usage: hydra resume <run_id>");
            }
        }
        "kill" => {
            let run_id = args.get(2).map(|s| s.as_str());
            commands::control::kill(run_id);
        }
        "inspect" => {
            if let Some(id) = args.get(2) {
                let mut format = "text";
                if args.get(3).map(|s| s.as_str()) == Some("--format") {
                    if let Some(f) = args.get(4) {
                        format = match f.as_str() {
                            "json" => "json",
                            "yaml" => "yaml",
                            _ => "text",
                        };
                    }
                }
                commands::inspect::execute(id, format);
            } else {
                output::print_error("Usage: hydra inspect <run_id> [--format text|json|yaml]");
            }
        }
        "config" => dispatch_config(args),
        "sisters" => dispatch_sisters(args),
        "skills" => dispatch_skills(args),
        "replay" => {
            if let Some(run_id) = args.get(2) {
                let dry_run = args[3..].iter().any(|a| a == "--dry-run" || a == "-n");
                commands::replay::execute(run_id, dry_run);
            } else {
                output::print_error("Usage: hydra replay <run_id> [--dry-run]");
            }
        }
        "memory" => dispatch_memory(args),
        "codebase" => dispatch_codebase(args),
        "vision" => dispatch_vision(args),
        "planning" => dispatch_planning(args),
        "soul" => dispatch_soul(args),
        "suspend" => {
            commands::suspend::suspend(args.get(2).map(|s| s.as_str()));
        }
        "resume-system" => {
            commands::suspend::resume_system();
        }
        "resurrect" => {
            commands::suspend::resurrect(args.get(2).map(|s| s.as_str()));
        }
        "remote" => dispatch_remote(args),
        "voice" => dispatch_voice(args),
        "policy" => dispatch_policy(args),
        "serve" => dispatch_serve(args),
        "profile" => dispatch_profile(args),
        "logs" => dispatch_logs(args),
        "completions" => {
            let shell = args.get(2).map(|s| s.as_str()).unwrap_or("bash");
            commands::completions::generate(shell);
        }
        "trust" => commands::trust::execute(),
        "inventions" => commands::inventions::execute(),
        "mcp" => dispatch_mcp(args),
        "tui" => {
            // Launch interactive TUI (same as running with no args)
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            match rt.block_on(crate::tui::run()) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("TUI error: {}", e);
                    eprintln!("Falling back to basic REPL...");
                    crate::repl::run();
                }
            }
        }
        "health" => super::cmd_health(),
        "help" | "--help" | "-h" => super::print_help(),
        "version" | "--version" | "-V" => super::print_version(),
        intent => {
            // Treat unknown commands as intents: hydra "do something"
            let mut all_words: Vec<String> = vec![intent.to_string()];
            all_words.extend(args[2..].iter().cloned());
            let combined = all_words.join(" ");
            commands::run::execute(&RunOptions {
                intent: combined,
                ..Default::default()
            });
        }
    }
}
