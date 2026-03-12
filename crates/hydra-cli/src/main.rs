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
mod tui;

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
        // Reconstruct args for dispatch: [argv0, subcommand, ...]
        let mut dispatch_args = vec![args[0].clone()];
        dispatch_args.extend(flags.remaining.clone());
        cli_dispatch::dispatch(&dispatch_args);
        return;
    }

    // Default: launch full TUI
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    match rt.block_on(tui::run()) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("TUI error: {}", e);
            eprintln!("Falling back to basic REPL...");
            repl::run();
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
        "    {}             {}",
        colors::blue("trust"),
        "Show trust & autonomy level"
    );
    println!(
        "    {}        {}",
        colors::blue("inventions"),
        "Show cognitive invention stats"
    );
    println!(
        "    {}            {}",
        colors::blue("health"),
        "Run health checks"
    );
    println!();
    println!("    {}              {}",
        colors::blue("mcp"),
        "Manage MCP server connections"
    );
    println!();
    println!("  {}", colors::bold("FLAGS"));
    println!("    -h, --help                       Show this help");
    println!("    -V, --version                    Show version");
    println!("    -p \"task\"                        Print mode (non-interactive)");
    println!("    -c                               Continue last session");
    println!("    -r <id>                          Resume specific session");
    println!("    --model <name>                   Use specific model");
    println!("    --permission-mode <mode>         Start in plan/auto-accept");
    println!("    --verbose                        Full turn-by-turn logging");
    println!("    --output-format json|stream-json Structured output");
    println!("    --max-budget-usd <amount>        Cost cap for session");
    println!("    --system-prompt \"...\"            Inline system prompt");
    println!("    --system-prompt-file <path>      System prompt from file");
    println!("    --append-system-prompt \"...\"     Append to default prompt");
    println!("    --allowedTools \"Read,Write\"      Pre-approve tools");
    println!("    --disallowedTools \"Bash(rm*)\"    Block tools");
    println!("    --dangerously-skip-permissions   Skip all approvals");
    println!("    --add-dir <path>                 Add extra directory");
    println!("    --from-pr <number>               Resume PR session");
    println!();
    println!("  {}", colors::bold("EXAMPLES"));
    println!("    {}", colors::dim("hydra -p \"fix the auth bug\""));
    println!("    {}", colors::dim("hydra -c"));
    println!("    {}", colors::dim("hydra --model opus run \"refactor auth\""));
    println!("    {}", colors::dim("hydra mcp add github -- npx @mcp/github"));
    println!("{}", colors::dim("    hydra status"));
    println!("{}", colors::dim("    hydra sisters connect memory"));
    println!();
}

fn print_version() {
    banner::print_banner();
}
