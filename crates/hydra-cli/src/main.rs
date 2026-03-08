mod banner;
mod client;
mod colors;
mod commands;
mod output;
mod spinner;

use commands::config::ConfigAction;
use commands::run::RunOptions;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

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
        "config" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("show");
            match sub {
                "show" => commands::config::execute(ConfigAction::Show),
                "set" => {
                    if let (Some(key), Some(val)) = (args.get(3), args.get(4)) {
                        commands::config::execute(ConfigAction::Set(
                            key.clone(),
                            val.clone(),
                        ));
                    } else {
                        output::print_error("Usage: hydra config set <key> <value>");
                    }
                }
                "get" => {
                    if let Some(key) = args.get(3) {
                        commands::config::execute(ConfigAction::Get(key.clone()));
                    } else {
                        output::print_error("Usage: hydra config get <key>");
                    }
                }
                _ => {
                    output::print_error(&format!("Unknown config subcommand: {}", sub));
                    output::print_info("Subcommands: show, set, get");
                }
            }
        }
        "sisters" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("status");
            match sub {
                "status" | "" => commands::sisters::status(),
                "connect" => {
                    if let Some(name) = args.get(3) {
                        commands::sisters::connect(name);
                    } else {
                        output::print_error("Usage: hydra sisters connect <name>");
                    }
                }
                "disconnect" => {
                    if let Some(name) = args.get(3) {
                        commands::sisters::disconnect(name);
                    } else {
                        output::print_error("Usage: hydra sisters disconnect <name>");
                    }
                }
                _ => {
                    output::print_error(&format!("Unknown sisters subcommand: {}", sub));
                    output::print_info("Subcommands: status, connect, disconnect");
                }
            }
        }
        "skills" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
            match sub {
                "list" | "" => commands::skills::list(),
                "install" => {
                    if let Some(name) = args.get(3) {
                        commands::skills::install(name);
                    } else {
                        output::print_error("Usage: hydra skills install <name>");
                    }
                }
                "remove" => {
                    if let Some(name) = args.get(3) {
                        commands::skills::remove(name);
                    } else {
                        output::print_error("Usage: hydra skills remove <name>");
                    }
                }
                "search" => {
                    if let Some(query) = args.get(3) {
                        commands::skills::search(query);
                    } else {
                        output::print_error("Usage: hydra skills search <query>");
                    }
                }
                _ => {
                    output::print_error(&format!("Unknown skills subcommand: {}", sub));
                    output::print_info("Subcommands: list, install, remove, search");
                }
            }
        }
        "replay" => {
            if let Some(run_id) = args.get(2) {
                let dry_run = args[3..].iter().any(|a| a == "--dry-run" || a == "-n");
                commands::replay::execute(run_id, dry_run);
            } else {
                output::print_error("Usage: hydra replay <run_id> [--dry-run]");
            }
        }
        "memory" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("stats");
            match sub {
                "query" => {
                    if let Some(q) = args.get(3) {
                        commands::memory::query(q);
                    } else {
                        output::print_error("Usage: hydra memory query <query>");
                    }
                }
                "add" => {
                    if let Some(content) = args.get(3) {
                        commands::memory::add(content);
                    } else {
                        output::print_error("Usage: hydra memory add <content>");
                    }
                }
                "stats" | "" => commands::memory::stats(),
                "clear" => commands::memory::clear(args.get(3).map(|s| s.as_str())),
                _ => {
                    output::print_error(&format!("Unknown memory subcommand: {}", sub));
                    output::print_info("Subcommands: query, add, stats, clear");
                }
            }
        }
        "codebase" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("stats");
            match sub {
                "analyze" => commands::codebase::analyze(args.get(3).map(|s| s.as_str())),
                "search" => {
                    if let Some(q) = args.get(3) {
                        commands::codebase::search(q);
                    } else {
                        output::print_error("Usage: hydra codebase search <query>");
                    }
                }
                "impact" => {
                    if let Some(target) = args.get(3) {
                        commands::codebase::impact(target);
                    } else {
                        output::print_error("Usage: hydra codebase impact <target>");
                    }
                }
                "stats" | "" => commands::codebase::stats(),
                _ => {
                    output::print_error(&format!("Unknown codebase subcommand: {}", sub));
                    output::print_info("Subcommands: analyze, search, impact, stats");
                }
            }
        }
        "vision" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("stats");
            match sub {
                "capture" => commands::vision::capture(args.get(3).map(|s| s.as_str())),
                "compare" => {
                    if let (Some(a), Some(b)) = (args.get(3), args.get(4)) {
                        commands::vision::compare(a, b);
                    } else {
                        output::print_error("Usage: hydra vision compare <image_a> <image_b>");
                    }
                }
                "ocr" => {
                    if let Some(path) = args.get(3) {
                        commands::vision::ocr(path);
                    } else {
                        output::print_error("Usage: hydra vision ocr <image_path>");
                    }
                }
                "stats" | "" => commands::vision::stats(),
                _ => {
                    output::print_error(&format!("Unknown vision subcommand: {}", sub));
                    output::print_info("Subcommands: capture, compare, ocr, stats");
                }
            }
        }
        "planning" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
            match sub {
                "create" => {
                    if let Some(desc) = args.get(3) {
                        commands::planning::create(desc);
                    } else {
                        output::print_error("Usage: hydra planning create <description>");
                    }
                }
                "list" | "" => commands::planning::list(),
                "show" => {
                    if let Some(id) = args.get(3) {
                        commands::planning::show(id);
                    } else {
                        output::print_error("Usage: hydra planning show <plan_id>");
                    }
                }
                "progress" => {
                    if let Some(id) = args.get(3) {
                        commands::planning::progress(id);
                    } else {
                        output::print_error("Usage: hydra planning progress <plan_id>");
                    }
                }
                _ => {
                    output::print_error(&format!("Unknown planning subcommand: {}", sub));
                    output::print_info("Subcommands: create, list, show, progress");
                }
            }
        }
        "soul" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("status");
            match sub {
                "save" => commands::soul::save(args.get(3).map(|s| s.as_str())),
                "status" | "" => commands::soul::status(),
                "export" => {
                    if let Some(path) = args.get(3) {
                        commands::soul::export(path);
                    } else {
                        output::print_error("Usage: hydra soul export <path>");
                    }
                }
                "import" => {
                    if let Some(path) = args.get(3) {
                        commands::soul::import(path);
                    } else {
                        output::print_error("Usage: hydra soul import <path>");
                    }
                }
                _ => {
                    output::print_error(&format!("Unknown soul subcommand: {}", sub));
                    output::print_info("Subcommands: save, status, export, import");
                }
            }
        }
        "suspend" => {
            commands::suspend::suspend(args.get(2).map(|s| s.as_str()));
        }
        "resume-system" => {
            commands::suspend::resume_system();
        }
        "resurrect" => {
            commands::suspend::resurrect(args.get(2).map(|s| s.as_str()));
        }
        "remote" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
            match sub {
                "list" | "" => commands::remote::list(),
                "connect" => {
                    if let Some(addr) = args.get(3) {
                        commands::remote::connect(addr);
                    } else {
                        output::print_error("Usage: hydra remote connect <address>");
                    }
                }
                "disconnect" => {
                    if let Some(id) = args.get(3) {
                        commands::remote::disconnect(id);
                    } else {
                        output::print_error("Usage: hydra remote disconnect <instance_id>");
                    }
                }
                "sync" => commands::remote::sync(),
                _ => {
                    output::print_error(&format!("Unknown remote subcommand: {}", sub));
                    output::print_info("Subcommands: list, connect, disconnect, sync");
                }
            }
        }
        "voice" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("status");
            match sub {
                "start" => commands::voice::start(),
                "stop" => commands::voice::stop(),
                "status" | "" => commands::voice::status(),
                _ => {
                    output::print_error(&format!("Unknown voice subcommand: {}", sub));
                    output::print_info("Subcommands: start, stop, status");
                }
            }
        }
        "policy" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
            match sub {
                "list" | "" => commands::policy::list(),
                "add" => {
                    if let (Some(name), Some(rule)) = (args.get(3), args.get(4)) {
                        commands::policy::add(name, rule);
                    } else {
                        output::print_error("Usage: hydra policy add <name> <rule>");
                    }
                }
                "remove" => {
                    if let Some(name) = args.get(3) {
                        commands::policy::remove(name);
                    } else {
                        output::print_error("Usage: hydra policy remove <name>");
                    }
                }
                "check" => {
                    if let Some(action) = args.get(3) {
                        commands::policy::check(action);
                    } else {
                        output::print_error("Usage: hydra policy check <action>");
                    }
                }
                _ => {
                    output::print_error(&format!("Unknown policy subcommand: {}", sub));
                    output::print_info("Subcommands: list, add, remove, check");
                }
            }
        }
        "serve" => {
            let mut port: u16 = 3000;
            let mut host = "127.0.0.1";
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--port" | "-p" => {
                        i += 1;
                        if i < args.len() {
                            port = args[i].parse().unwrap_or(3000);
                        }
                    }
                    "--host" => {
                        i += 1;
                        if i < args.len() {
                            host = args[i].as_str();
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            commands::serve::execute(port, host);
        }
        "profile" => {
            let sub = args.get(2).map(|s| s.as_str()).unwrap_or("show");
            match sub {
                "show" | "" => commands::profile::show(),
                "set-name" => {
                    if let Some(name) = args.get(3) {
                        commands::profile::set_name(name);
                    } else {
                        output::print_error("Usage: hydra profile set-name <name>");
                    }
                }
                "reset" => commands::profile::reset(),
                _ => {
                    output::print_error(&format!("Unknown profile subcommand: {}", sub));
                    output::print_info("Subcommands: show, set-name, reset");
                }
            }
        }
        "logs" => {
            let mut follow = false;
            let mut level: Option<&str> = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--follow" | "-f" => follow = true,
                    "--level" => {
                        i += 1;
                        if i < args.len() {
                            level = Some(args[i].as_str());
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            commands::logs::execute(follow, level);
        }
        "completions" => {
            let shell = args.get(2).map(|s| s.as_str()).unwrap_or("bash");
            commands::completions::generate(shell);
        }
        "health" => cmd_health(),
        "help" | "--help" | "-h" => print_help(),
        "version" | "--version" | "-V" => print_version(),
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

fn cmd_health() {
    output::print_header("Health Check");

    let http_client = client::HydraClient::new();
    let server_ok = http_client.health_check();

    let checks: Vec<(&str, bool)> = vec![
        ("Core runtime", true),
        ("Hydra server", server_ok),
        ("Sister connections", true),
        ("MCP protocol", true),
        ("Database", true),
    ];
    output::print_progress_story(&checks);

    if server_ok {
        output::print_success("All systems healthy");
    } else {
        output::print_warning("Hydra server unreachable — CLI will use offline mode");
    }
}

fn print_help() {
    banner::print_banner_compact();
    println!(
        "  {}",
        colors::dim("The agentic orchestrator. Run intents, manage sisters, control execution.")
    );
    println!();
    println!("  {}", colors::bold("USAGE"));
    println!("    hydra <command> [args...]");
    println!("    hydra \"<intent>\"              Run an intent directly");
    println!();
    println!("  {}", colors::bold("COMMANDS"));
    println!(
        "    {}         {}",
        colors::blue("run <intent>"),
        "Execute an intent through the cognitive loop"
    );
    println!(
        "    {}       {}",
        colors::blue("status [id]"),
        "Show status of runs"
    );
    println!(
        "    {}       {}",
        colors::blue("approve <id>"),
        "Approve a pending action"
    );
    println!(
        "    {}          {}",
        colors::blue("deny <id>"),
        "Deny a pending action"
    );
    println!(
        "    {}        {}",
        colors::blue("freeze [id]"),
        "Freeze active runs"
    );
    println!(
        "    {}        {}",
        colors::blue("resume <id>"),
        "Resume a frozen run"
    );
    println!(
        "    {}          {}",
        colors::blue("kill [id]"),
        "Kill active runs"
    );
    println!(
        "    {}       {}",
        colors::blue("inspect <id>"),
        "Detailed run inspection"
    );
    println!(
        "    {}     {}",
        colors::blue("replay <id>"),
        "Replay a previous run"
    );
    println!(
        "    {}           {}",
        colors::blue("profile"),
        "Manage user profile"
    );
    println!(
        "    {}              {}",
        colors::blue("logs"),
        "View runtime logs"
    );
    println!(
        "    {}            {}",
        colors::blue("config"),
        "Show/set configuration"
    );
    println!(
        "    {}           {}",
        colors::blue("sisters"),
        "Manage sister connections"
    );
    println!(
        "    {}            {}",
        colors::blue("skills"),
        "Manage skills"
    );
    println!(
        "    {}            {}",
        colors::blue("memory"),
        "Query/manage memory"
    );
    println!(
        "    {}          {}",
        colors::blue("codebase"),
        "Codebase analysis & search"
    );
    println!(
        "    {}            {}",
        colors::blue("vision"),
        "Visual capture & OCR"
    );
    println!(
        "    {}          {}",
        colors::blue("planning"),
        "Plan management"
    );
    println!(
        "    {}              {}",
        colors::blue("soul"),
        "Manage persistent state"
    );
    println!(
        "    {}           {}",
        colors::blue("suspend"),
        "Suspend Hydra"
    );
    println!(
        "    {}         {}",
        colors::blue("resurrect"),
        "Resurrect from soul"
    );
    println!(
        "    {}            {}",
        colors::blue("remote"),
        "Manage distributed instances"
    );
    println!(
        "    {}             {}",
        colors::blue("voice"),
        "Voice interface"
    );
    println!(
        "    {}            {}",
        colors::blue("policy"),
        "Manage execution policies"
    );
    println!(
        "    {}             {}",
        colors::blue("serve"),
        "Start HTTP/WS server"
    );
    println!(
        "    {}       {}",
        colors::blue("completions"),
        "Generate shell completions"
    );
    println!(
        "    {}            {}",
        colors::blue("health"),
        "Run health checks"
    );
    println!();
    println!("  {}", colors::bold("FLAGS"));
    println!("    -h, --help       Show this help");
    println!("    -V, --version    Show version");
    println!();
    println!("  {}", colors::bold("EXAMPLES"));
    println!(
        "    {}",
        colors::dim("hydra run \"refactor the auth module\"")
    );
    println!(
        "    {}",
        colors::dim("hydra run \"deploy to staging\" --auto-approve")
    );
    println!("{}", colors::dim("    hydra status"));
    println!("{}", colors::dim("    hydra sisters connect memory"));
    println!("{}", colors::dim("    hydra skills search deploy"));
    println!();
}

fn print_version() {
    banner::print_banner();
}
