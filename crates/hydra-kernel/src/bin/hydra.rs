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
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(e) = persistence::acquire_boot_lock() {
        eprintln!("Hydra: {}", e);
        std::process::exit(1);
    }
    struct LockGuard;
    impl Drop for LockGuard {
        fn drop(&mut self) {
            persistence::release_boot_lock();
        }
    }
    let _guard = LockGuard;

    let args: Vec<String> = std::env::args().skip(1).collect();

    eprintln!("Hydra — Agentra Labs");
    eprintln!(
        "Provider: {}",
        std::env::var("HYDRA_LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into())
    );
    eprintln!("---");

    // Daemon mode — always on, runs all three loops
    if args.first().map(|s| s.as_str()) == Some("--daemon") {
        run_daemon().await;
        return;
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

    let ambient_interval = std::time::Duration::from_millis(100);
    let dream_interval = std::time::Duration::from_millis(500);
    let mut last_dream = std::time::Instant::now();

    eprintln!("hydra: daemon alive — ambient=100ms dream=500ms");
    eprintln!("hydra: genome={} entries, self-writing enabled", dream.genome.len());

    // The alive loop — runs until the process is killed
    loop {
        // Ambient tick
        let result = tick_with_subsystems(&state, 0.1, Some(&mut ambient));
        state = result.state;

        if !result.invariants_ok {
            eprintln!("hydra: INVARIANT FAILURE — {}", result.summary);
        }

        // Dream cycle (every 500ms)
        if last_dream.elapsed() >= dream_interval {
            let dream_result = cycle_with_subsystems(&state, Some(&mut dream));
            if dream_result.genome_entries_created > 0 {
                eprintln!(
                    "hydra: GENOME GREW — {} new entries from experience",
                    dream_result.genome_entries_created
                );
            }
            last_dream = std::time::Instant::now();
        }

        tokio::time::sleep(ambient_interval).await;
    }
}
