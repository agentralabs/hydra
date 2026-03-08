use crate::client::HydraClient;
use crate::colors;
use crate::output;

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
    match phase {
        "perceive" => "\u{1f440}",
        "think" => "\u{1f9e0}",
        "decide" => "\u{1f914}",
        "act" => "\u{26a1}",
        "learn" => "\u{1f4da}",
        _ => "\u{2022}",
    }
}

pub fn phase_label(phase: &str) -> &'static str {
    match phase {
        "perceive" => "Perceive",
        "think" => "Think",
        "decide" => "Decide",
        "act" => "Act",
        "learn" => "Learn",
        _ => "Unknown",
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

    // Try server first
    let client = HydraClient::new();
    let server_result = try_server_run(&client, opts);

    match server_result {
        Ok((run_id, data)) => {
            output::print_dimmed(&format!("Run ID: {} (connected to server)", run_id));
            println!();

            // Display phases from server response or use defaults
            let phases = data["phases"].as_array();
            if let Some(phase_list) = phases {
                for (i, phase_val) in phase_list.iter().enumerate() {
                    let name = phase_val["name"].as_str().unwrap_or("unknown");
                    let emoji = phase_emoji(name);
                    let label = phase_label(name);
                    let status = phase_val["status"].as_str().unwrap_or("pending");
                    let detail = phase_val["detail"].as_str().unwrap_or("");

                    if opts.verbose {
                        println!(
                            "  {} {} {} {}",
                            colors::green(&format!("[{}/{}]", i + 1, phase_list.len())),
                            emoji,
                            colors::bold(label),
                            colors::dim(detail)
                        );
                    } else if !opts.quiet {
                        let status_indicator = if status == "complete" {
                            colors::green("\u{2713}")
                        } else {
                            emoji.to_string()
                        };
                        println!("  {} {}", status_indicator, colors::bold(label));
                    }
                }
            } else {
                // Fallback: show standard phases
                for phase in COGNITIVE_PHASES {
                    if !opts.quiet {
                        println!("  {} {}", phase_emoji(phase), colors::bold(phase_label(phase)));
                    }
                }
            }

            // Summary
            println!();
            let tokens = data["tokens"].as_u64().unwrap_or(0);
            let duration = data["duration"].as_str().unwrap_or("--");
            let phase_count = phases.map(|p| p.len()).unwrap_or(COGNITIVE_PHASES.len());
            output::print_box(&[
                &format!("Run {} complete", run_id),
                &format!("Phases: {}", phase_count),
                &format!("Tokens: {}", output::format_tokens(tokens)),
                &format!("Duration: {}", duration),
            ]);
            println!();
            output::print_success("Intent fulfilled");
        }
        Err(_) => {
            execute_stub(opts);
        }
    }
}

fn try_server_run(
    client: &HydraClient,
    opts: &RunOptions,
) -> Result<(String, serde_json::Value), String> {
    // Create conversation
    let create_body = serde_json::json!({
        "intent": opts.intent,
        "auto_approve": opts.auto_approve,
    });
    let conv = client.post("/api/conversations", &create_body)?;
    let conv_id = conv["id"]
        .as_str()
        .ok_or_else(|| "Missing conversation id".to_string())?
        .to_string();

    // Send message
    let msg_body = serde_json::json!({
        "content": opts.intent,
    });
    let result = client.post(
        &format!("/api/conversations/{}/messages", conv_id),
        &msg_body,
    )?;

    Ok((conv_id, result))
}

fn execute_stub(_opts: &RunOptions) {
    output::print_error("Server unreachable. Cannot execute run.");
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
