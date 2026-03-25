//! Hydra binary — cognitive loop entry point.
//!
//! Three modes:
//!   cargo run -p hydra-kernel --bin hydra -- "your input here"   (single-shot)
//!   cargo run -p hydra-kernel --bin hydra -- --interactive        (REPL)
//!   cargo run -p hydra-kernel --bin hydra -- --daemon             (always-on)

use hydra_kernel::engine::CognitiveLoop;
use hydra_kernel::loop_ambient::{AmbientSubsystems, tick_with_subsystems};
use hydra_kernel::loop_dream::{DreamSubsystems, cycle_with_subsystems};
use hydra_kernel::persistence;
use hydra_kernel::state::HydraState;
use hydra_kernel::{task_engine, workspace};
use std::io::{self, BufRead, Write};

fn main() {
    // Suppress the known tokio/reqwest shutdown panic (not a real error).
    // reqwest::blocking::Client creates its own internal tokio runtime;
    // when the outer runtime drops, the inner one panics. The response
    // is already delivered and workspace saved — this is benign.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();
        if msg.contains("Cannot drop a runtime in a context where blocking is not allowed") {
            return; // Known benign — reqwest blocking client vs tokio shutdown
        }
        default_hook(info);
    }));

    // catch_unwind absorbs the tokio shutdown panic so exit code is 0, not 101
    let _ = std::panic::catch_unwind(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async_main());
    });
}

async fn async_main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(e) = persistence::acquire_boot_lock() {
        eprintln!("Hydra: {}", e);
        std::process::exit(1);
    }
    struct LockGuard;
    impl Drop for LockGuard {
        fn drop(&mut self) {
            // O7: Save workspace snapshot on exit
            let engine = task_engine::TaskEngine::new();
            let snap = workspace::capture(&engine);
            if let Err(e) = workspace::save_snapshot(&snap) {
                eprintln!("hydra: workspace save failed: {e}");
            }
            persistence::release_boot_lock();
        }
    }
    let _guard = LockGuard;

    let args: Vec<String> = std::env::args().skip(1).collect();

    // First-run wizard
    if hydra_kernel::first_run::is_first_run() && !hydra_kernel::first_run::run_wizard() {
        eprintln!("Setup cancelled.");
        std::process::exit(0);
    }

    // Auto-install system dependencies + check permissions
    hydra_desktop::deps::preflight();

    eprintln!("Hydra — Agentra Labs");
    eprintln!(
        "Provider: {}",
        std::env::var("HYDRA_LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into())
    );
    eprintln!("---");

    // Update command
    if args.first().map(|s| s.as_str()) == Some("--update") {
        check_for_update();
        return;
    }

    // Backup command
    if args.first().map(|s| s.as_str()) == Some("--backup") {
        match hydra_kernel::backup::create_backup() {
            Ok(r) => {
                println!("Backup created: {} ({} files)", r.path.display(), r.files_copied);
                hydra_kernel::backup::prune_old_backups(30);
            }
            Err(e) => eprintln!("Backup failed: {e}"),
        }
        return;
    }

    // Daemon mode — always on, runs all three loops
    if args.first().map(|s| s.as_str()) == Some("--daemon") {
        run_daemon().await;
        return;
    }

    // Fix #15: Load workspace snapshot from previous session
    if let Some(snap) = workspace::load_snapshot() {
        eprintln!("hydra: workspace restored ({} tasks, {} processes)",
            snap.pending_tasks.len(), snap.processes.len());
    }

    let mut hydra = CognitiveLoop::new();

    if !args.is_empty() && args[0] != "--interactive" {
        let input = args.join(" ");
        let response = hydra.cycle(&input).await;
        println!("{}", response);
        return;
    }

    // Interactive REPL
    println!("Interactive mode. Type 'exit' to quit.");
    println!("Type '/status' for subsystem status.\n");

    let stdin = io::stdin();
    loop {
        print!("you > ");
        io::stdout().flush().expect("flush stdout");

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "exit" || trimmed == "quit" {
            break;
        }
        if trimmed == "/status" {
            println!("{}", hydra.status());
            continue;
        }

        let response = hydra.cycle(trimmed).await;
        println!("\nhydra > {}\n", response);
    }

    println!("Hydra shutdown.");
}

/// Daemon mode — Hydra runs continuously.
/// Active thread waits for input. Ambient thread ticks every 100ms.
/// Dream thread runs every 500ms. The genome self-writes.
/// This is what the launchd plist runs.
async fn run_daemon() {
    eprintln!("hydra: daemon mode — always on");

    // Boot
    match hydra_kernel::run_boot_sequence().await {
        Ok(boot) => {
            eprintln!(
                "hydra: boot complete in {}ms ({} phases)",
                boot.boot_duration_ms,
                boot.phases_completed.len()
            );
        }
        Err(e) => {
            eprintln!("hydra: boot FAILED: {e}");
            std::process::exit(1);
        }
    }

    let mut state = HydraState::initial();
    let mut ambient = AmbientSubsystems::new();
    let mut dream = DreamSubsystems::new();
    let mut proactive = hydra_kernel::proactive::ProactiveEngine::new();

    let ambient_interval = std::time::Duration::from_millis(100);
    let dream_interval = std::time::Duration::from_millis(500);
    let proactive_interval = std::time::Duration::from_secs(60);
    let mut last_dream = std::time::Instant::now();
    let mut last_proactive = std::time::Instant::now();

    eprintln!("hydra: daemon alive — ambient=100ms dream=500ms");
    eprintln!("hydra: genome={} entries, self-writing enabled", dream.genome.len());

    // Start HTTP API server on background task
    let api_port = std::env::var("HYDRA_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(hydra_kernel::http_api::DEFAULT_PORT);
    tokio::spawn(async move {
        if let Err(e) = hydra_kernel::http_api::start_server(api_port).await {
            eprintln!("hydra: HTTP API error: {e}");
        }
    });
    // B1: Verify HTTP API is reachable after spawn
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    match reqwest::Client::new().get(format!("http://127.0.0.1:{api_port}/api/health"))
        .timeout(std::time::Duration::from_secs(2)).send().await {
        Ok(r) if r.status().is_success() => eprintln!("hydra: HTTP API listening on port {api_port}"),
        Ok(r) => eprintln!("hydra: HTTP API responded but status={}", r.status()),
        Err(_) => eprintln!("hydra: WARNING — HTTP API not reachable on port {api_port}"),
    }

    // The alive loop — runs until the process is killed
    // Fix #14: Wrap subsystem ticks in catch_unwind so one panic doesn't crash daemon
    loop {
        // Ambient tick (catch panic — subsystem crash must not kill daemon)
        let ambient_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tick_with_subsystems(&state, 0.1, Some(&mut ambient))
        }));
        match ambient_result {
            Ok(result) => {
                state = result.state;
                if !result.invariants_ok {
                    eprintln!("hydra: INVARIANT FAILURE — {}", result.summary);
                }
            }
            Err(_) => eprintln!("hydra: AMBIENT TICK PANIC — recovered, continuing"),
        }

        // Dream cycle (every 500ms)
        if last_dream.elapsed() >= dream_interval {
            let dream_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                cycle_with_subsystems(&state, Some(&mut dream))
            }));
            match dream_result {
                Ok(result) => {
                    if result.genome_entries_created > 0 {
                        eprintln!("hydra: GENOME GREW — {} new entries from experience",
                            result.genome_entries_created);
                    }
                }
                Err(_) => eprintln!("hydra: DREAM CYCLE PANIC — recovered, continuing"),
            }
            last_dream = std::time::Instant::now();
        }

        // O31: Proactive Initiation — check triggers every 60s
        if last_proactive.elapsed() >= proactive_interval {
            let genome = hydra_genome::GenomeStore::open();
            let triggers = hydra_kernel::proactive::ProactiveEngine::collect_triggers(&genome);
            if !triggers.is_empty() {
                let actions = proactive.evaluate_triggers(triggers, &genome, false);
                for action in &actions {
                    eprintln!("hydra-proactive: INITIATING '{}' (autonomy={:.2})",
                        action.goal, action.autonomy_score);
                    // Execute via conductor
                    let result = hydra_kernel::conductor_exec::conduct(&action.goal, &genome);
                    eprintln!("hydra-proactive: completed '{}'  → {:?}",
                        action.goal, if matches!(result, hydra_kernel::conductor::ConductorResult::Complete { .. }) { "OK" } else { "FAILED" });
                }
            }
            last_proactive = std::time::Instant::now();
        }

        tokio::time::sleep(ambient_interval).await;
    }
}

/// Check GitHub for the latest release and offer to update.
fn check_for_update() {
    println!("Checking for updates...");

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("hydra-update/0.1")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("HTTP client error: {e}");
            return;
        }
    };

    let url = "https://api.github.com/repos/agentralabs/hydra/releases/latest";
    match client.get(url).send() {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.text() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    let tag = json
                        .get("tag_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let current = env!("CARGO_PKG_VERSION");
                    println!("Current version: v{current}");
                    println!("Latest release:  {tag}");

                    if tag.trim_start_matches('v') == current {
                        println!("You are up to date.");
                    } else {
                        println!("Update available!");
                        println!(
                            "Download: https://github.com/agentralabs/hydra/releases/tag/{tag}"
                        );
                        println!("Or run: bash scripts/install.sh");
                    }
                }
            }
        }
        Ok(resp) => {
            eprintln!("GitHub API returned: {}", resp.status());
        }
        Err(e) => {
            eprintln!("Network error: {e}");
            eprintln!("Check your internet connection.");
        }
    }
}
