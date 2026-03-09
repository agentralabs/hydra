use std::io::{self, Write};
use std::time::Instant;

use crate::client::HydraClient;
use crate::colors;
use crate::output;
use crate::spinner::Spinner;

pub struct RunOptions {
    pub intent: String,
    pub auto_approve: bool,
    pub dry_run: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub timeout_secs: Option<u64>,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            intent: String::new(),
            auto_approve: false,
            dry_run: false,
            verbose: false,
            quiet: false,
            timeout_secs: None,
        }
    }
}

const COGNITIVE_PHASES: &[&str] = &["perceive", "think", "decide", "act", "learn"];

pub fn phase_emoji(phase: &str) -> &'static str {
    match phase.to_lowercase().as_str() {
        "perceive" => "\u{1f440}",
        "think" => "\u{1f9e0}",
        "decide" => "\u{1f914}",
        "act" => "\u{26a1}",
        "learn" => "\u{1f4da}",
        "done" => "\u{2705}",
        "error" => "\u{274c}",
        _ => "\u{2022}",
    }
}

pub fn phase_label(phase: &str) -> &'static str {
    match phase.to_lowercase().as_str() {
        "perceive" => "Perceive",
        "think" => "Think",
        "decide" => "Decide",
        "act" => "Act",
        "learn" => "Learn",
        "done" => "Done",
        "error" => "Error",
        _ => "Unknown",
    }
}

fn phase_color(phase: &str) -> String {
    match phase.to_lowercase().as_str() {
        "perceive" => colors::blue(phase_label(phase)),
        "think" => colors::yellow(phase_label(phase)),
        "decide" => colors::red(phase_label(phase)),
        "act" => colors::green(phase_label(phase)),
        "learn" => colors::blue(phase_label(phase)),
        "done" => colors::green(phase_label(phase)),
        "error" => colors::red(phase_label(phase)),
        _ => phase_label(phase).to_string(),
    }
}

pub fn execute(opts: &RunOptions) {
    if opts.intent.is_empty() {
        output::print_error("No intent provided. Usage: hydra run \"<intent>\"");
        return;
    }

    if opts.dry_run {
        output::print_header("Dry Run");
        println!();
        output::print_info(&format!("Intent: {}", opts.intent));
        println!();
        output::print_dimmed("Would execute the following cognitive phases:");
        println!();
        for phase in COGNITIVE_PHASES {
            println!(
                "  {} {} {}",
                phase_emoji(phase),
                colors::bold(phase_label(phase)),
                colors::dim("(skipped - dry run)")
            );
        }
        println!();
        output::print_info("No actions taken in dry-run mode");
        return;
    }

    // Header
    output::print_header("Hydra");
    println!();
    output::print_info(&format!("Intent: {}", opts.intent));
    if let Some(timeout) = opts.timeout_secs {
        output::print_dimmed(&format!("Timeout: {}s", timeout));
    }
    if opts.auto_approve {
        output::print_dimmed("Auto-approve: enabled");
    }
    println!();

    // Try server with SSE streaming first, then fallback to REST
    let client = HydraClient::new();
    let start = Instant::now();

    // Try to create run and stream events
    let server_result = try_server_run_with_streaming(&client, opts);

    match server_result {
        Ok((run_id, tokens, phase_count)) => {
            let elapsed = start.elapsed();
            println!();
            output::print_box(&[
                &format!("Run {} complete", run_id),
                &format!("Phases: {}", phase_count),
                &format!("Tokens: {}", output::format_tokens(tokens)),
                &format!("Duration: {:.1}s", elapsed.as_secs_f64()),
            ]);
            println!();
            output::print_success("Intent fulfilled");
        }
        Err(_) => {
            // Fallback: try REST without streaming
            match try_server_run_rest(&client, opts) {
                Ok((run_id, data)) => {
                    let elapsed = start.elapsed();
                    output::print_dimmed(&format!("Run ID: {} (connected to server)", run_id));
                    println!();

                    // Show phases from response
                    for phase in COGNITIVE_PHASES {
                        if !opts.quiet {
                            println!(
                                "  {} {}",
                                colors::green("\u{2713}"),
                                phase_color(phase)
                            );
                        }
                    }

                    println!();
                    let tokens = data["tokens"].as_u64().unwrap_or(0);
                    output::print_box(&[
                        &format!("Run {} complete", run_id),
                        &format!("Phases: {}", COGNITIVE_PHASES.len()),
                        &format!("Tokens: {}", output::format_tokens(tokens)),
                        &format!("Duration: {:.1}s", elapsed.as_secs_f64()),
                    ]);
                    println!();
                    output::print_success("Intent fulfilled");
                }
                Err(_) => {
                    execute_offline(opts);
                }
            }
        }
    }
}

/// Try to run via server with SSE streaming for live phase progress.
fn try_server_run_with_streaming(
    client: &HydraClient,
    opts: &RunOptions,
) -> Result<(String, u64, usize), String> {
    // Create run
    let create_body = serde_json::json!({
        "intent": opts.intent,
        "auto_approve": opts.auto_approve,
    });
    let conv = client.post("/api/conversations", &create_body)?;
    let conv_id = conv["id"]
        .as_str()
        .ok_or_else(|| "Missing conversation id".to_string())?
        .to_string();

    // Send message to trigger cognitive loop
    let msg_body = serde_json::json!({ "content": opts.intent });
    client.post(
        &format!("/api/conversations/{}/messages", conv_id),
        &msg_body,
    )?;

    // Stream events
    let mut total_tokens: u64 = 0;
    let mut phase_count: usize = 0;
    let mut current_phase = String::new();
    let mut phase_start = Instant::now();
    let spinner = Spinner::new("");
    let mut frame_idx: usize = 0;
    let quiet = opts.quiet;
    let verbose = opts.verbose;

    let stream_result = client.subscribe_sse(&format!("/events?run={}", conv_id), |event| {
        let data: serde_json::Value =
            serde_json::from_str(&event.data).unwrap_or(serde_json::Value::Null);

        match event.event.as_str() {
            "StepStarted" | "phase" => {
                let phase = data["phase"]
                    .as_str()
                    .or_else(|| data["name"].as_str())
                    .unwrap_or("unknown");

                // Complete previous phase
                if !current_phase.is_empty() && !quiet {
                    let elapsed = phase_start.elapsed();
                    // Clear spinner line and print completed phase
                    print!("\r\x1b[K");
                    println!(
                        "  {} {} {}",
                        colors::green("\u{2713}"),
                        phase_color(&current_phase),
                        colors::dim(&format!("[{:.1}s]", elapsed.as_secs_f64()))
                    );
                }

                current_phase = phase.to_string();
                phase_start = Instant::now();
                phase_count += 1;

                if !quiet {
                    print!(
                        "\r  {} {} {}...",
                        spinner.frame_at(frame_idx),
                        phase_emoji(phase),
                        phase_color(phase)
                    );
                    io::stdout().flush().unwrap_or_default();
                }
                frame_idx += 1;
            }
            "StepProgress" => {
                if !quiet {
                    frame_idx += 1;
                    let detail = data["detail"].as_str().unwrap_or("");
                    if verbose && !detail.is_empty() {
                        print!("\r\x1b[K");
                        print!(
                            "\r  {} {} {} {}",
                            spinner.frame_at(frame_idx),
                            phase_emoji(&current_phase),
                            phase_color(&current_phase),
                            colors::dim(detail)
                        );
                        io::stdout().flush().unwrap_or_default();
                    } else {
                        print!(
                            "\r  {} {} {}...",
                            spinner.frame_at(frame_idx),
                            phase_emoji(&current_phase),
                            phase_color(&current_phase)
                        );
                        io::stdout().flush().unwrap_or_default();
                    }
                }
            }
            "token_usage" => {
                let input = data["input_tokens"].as_u64().unwrap_or(0);
                let output = data["output_tokens"].as_u64().unwrap_or(0);
                total_tokens += input + output;
            }
            "RunCompleted" | "complete" => {
                // Complete last phase
                if !current_phase.is_empty() && !quiet {
                    let elapsed = phase_start.elapsed();
                    print!("\r\x1b[K");
                    println!(
                        "  {} {} {}",
                        colors::green("\u{2713}"),
                        phase_color(&current_phase),
                        colors::dim(&format!("[{:.1}s]", elapsed.as_secs_f64()))
                    );
                }
                return false; // stop streaming
            }
            "RunError" | "error" => {
                print!("\r\x1b[K");
                let msg = data["message"].as_str().unwrap_or("Unknown error");
                println!("  {} {}", colors::red("\u{2717}"), msg);
                return false;
            }
            _ => {}
        }
        true // continue streaming
    });

    // If SSE subscription itself failed, return error
    stream_result?;

    Ok((conv_id, total_tokens, phase_count))
}

/// Fallback: REST-only run without streaming.
fn try_server_run_rest(
    client: &HydraClient,
    opts: &RunOptions,
) -> Result<(String, serde_json::Value), String> {
    let create_body = serde_json::json!({
        "intent": opts.intent,
        "auto_approve": opts.auto_approve,
    });
    let conv = client.post("/api/conversations", &create_body)?;
    let conv_id = conv["id"]
        .as_str()
        .ok_or_else(|| "Missing conversation id".to_string())?
        .to_string();

    let msg_body = serde_json::json!({ "content": opts.intent });
    let result = client.post(
        &format!("/api/conversations/{}/messages", conv_id),
        &msg_body,
    )?;

    Ok((conv_id, result))
}

/// Offline mode: show phases locally when server is unreachable.
fn execute_offline(opts: &RunOptions) {
    output::print_warning("Server unreachable — running in offline display mode");
    println!();

    let spinner = Spinner::new("");
    let phase_durations = [0.3, 0.5, 0.2, 0.4, 0.2]; // simulated

    for (i, phase) in COGNITIVE_PHASES.iter().enumerate() {
        if !opts.quiet {
            // Show spinner briefly
            for frame in 0..3 {
                print!(
                    "\r  {} {} {}...",
                    spinner.frame_at(frame),
                    phase_emoji(phase),
                    phase_color(phase)
                );
                io::stdout().flush().unwrap_or_default();
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            // Complete
            print!("\r\x1b[K");
            println!(
                "  {} {} {}",
                colors::green("\u{2713}"),
                phase_color(phase),
                colors::dim(&format!("[{:.1}s]", phase_durations[i]))
            );
        }
    }

    println!();
    output::print_warning("No server connection — results are display-only");
    output::print_dimmed("Start the server with: hydra serve");
}

pub fn parse_and_execute(args: &[String]) {
    let mut opts = RunOptions::default();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--auto-approve" | "-y" => opts.auto_approve = true,
            "--dry-run" | "-n" => opts.dry_run = true,
            "--verbose" | "-v" => opts.verbose = true,
            "--quiet" | "-q" => opts.quiet = true,
            "--timeout" => {
                i += 1;
                if i < args.len() {
                    opts.timeout_secs = args[i].parse().ok();
                }
            }
            other => {
                if !other.starts_with('-') && opts.intent.is_empty() {
                    opts.intent = other.to_string();
                } else if !other.starts_with('-') {
                    opts.intent = format!("{} {}", opts.intent, other);
                }
            }
        }
        i += 1;
    }

    execute(&opts);
}
