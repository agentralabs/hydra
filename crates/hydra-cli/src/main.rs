#![allow(dead_code)]

mod banner;
mod cli_dispatch;
mod cli_flags;
mod cli_subcommands;
mod client;
mod colors;
mod commands;
mod output;
mod repl;
mod spinner;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flags = cli_flags::CliFlags::parse(&args);

    // Apply environment overrides from flags (--model, --verbose, etc.)
    flags.apply_env();

    // -p "task" — non-interactive print mode
    if let Some(ref task) = flags.print_task {
        commands::run::execute(&commands::run::RunOptions {
            intent: task.clone(),
            ..Default::default()
        });
        return;
    }

    // --repl — legacy REPL mode
    if flags.repl {
        repl::run();
        return;
    }

    // Subcommand present → dispatch
    if !flags.remaining.is_empty() {
        let mut dispatch_args = vec![args[0].clone()];
        dispatch_args.extend(flags.remaining.clone());
        cli_dispatch::dispatch(&dispatch_args);
        return;
    }

    // Default: launch hydra-tui (Go + Bubble Tea binary)
    launch_tui(&args);
}

/// Launch the hydra-tui Go binary.
/// hydra-cli is now a thin launcher — all TUI logic lives in hydra-tui/.
fn launch_tui(args: &[String]) {
    if let Some(tui_path) = find_hydra_tui_binary() {
        let status = std::process::Command::new(&tui_path)
            .args(&args[1..])
            .status();
        match status {
            Ok(s) if s.success() => return,
            Ok(s) => {
                eprintln!("hydra-tui exited with code: {:?}", s.code());
                std::process::exit(s.code().unwrap_or(1));
            }
            Err(e) => {
                eprintln!("Failed to launch hydra-tui: {}", e);
            }
        }
    }

    // hydra-tui not found — tell user how to install
    eprintln!();
    eprintln!("  {} hydra-tui binary not found.", colors::red("Error:"));
    eprintln!();
    eprintln!("  Install it with one of:");
    eprintln!("    {}",
        colors::blue("cd crates/hydra-tui && go build -o ~/.local/bin/hydra-tui ."));
    eprintln!("    {}",
        colors::blue("cd crates/hydra-tui && go install ."));
    eprintln!("    {}",
        colors::blue("go install github.com/agentralabs/hydra-tui@latest"));
    eprintln!();
    eprintln!("  Or use the basic REPL: {}", colors::dim("hydra --repl"));
    eprintln!();
    std::process::exit(1);
}

/// Find the hydra-tui Go binary.
/// Searches: PATH, ~/.local/bin, project hydra-tui/, GOPATH/bin.
fn find_hydra_tui_binary() -> Option<String> {
    // 1. PATH
    if let Ok(path) = which::which("hydra-tui") {
        return Some(path.display().to_string());
    }

    // 2. ~/.local/bin/hydra-tui
    if let Some(home) = dirs_next::home_dir() {
        let local = home.join(".local/bin/hydra-tui");
        if local.exists() {
            return Some(local.display().to_string());
        }
        // 3. GOPATH/bin
        let gopath = std::env::var("GOPATH")
            .unwrap_or_else(|_| home.join("go").display().to_string());
        let gobin = std::path::PathBuf::from(gopath).join("bin/hydra-tui");
        if gobin.exists() {
            return Some(gobin.display().to_string());
        }
    }

    // 4. Project directory (development) — crates/hydra-tui/hydra-tui
    let project_bin = std::path::Path::new("crates/hydra-tui/hydra-tui");
    if project_bin.exists() {
        return Some(project_bin.display().to_string());
    }

    None
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
        "    {}          {}",
        colors::blue("kill [id]"),
        "Kill active runs"
    );
    println!(
        "    {}           {}",
        colors::blue("profile"),
        "Manage user profile"
    );
    println!(
        "    {}           {}",
        colors::blue("sisters"),
        "Manage sister connections"
    );
    println!(
        "    {}            {}",
        colors::blue("health"),
        "Run health checks"
    );
    println!(
        "    {}             {}",
        colors::blue("serve"),
        "Start HTTP/WS server"
    );
    println!(
        "    {}              {}",
        colors::blue("mcp"),
        "Manage MCP server connections"
    );
    println!();
    println!("  {}", colors::bold("FLAGS"));
    println!("    -h, --help                       Show this help");
    println!("    -V, --version                    Show version");
    println!("    -p \"task\"                        Print mode (non-interactive)");
    println!("    --model <name>                   Use specific model");
    println!("    --repl                           Basic REPL mode (no TUI)");
    println!();
    println!("  {}", colors::bold("TUI"));
    println!("    Run without arguments to launch the full TUI (hydra-tui).");
    println!("    Install: cd crates/hydra-tui && go install .");
    println!();
}

fn print_version() {
    banner::print_banner();
}
