use crate::client::HydraClient;
use crate::colors;
use crate::output;

pub fn execute(run_id: &str, dry_run: bool) {
    let client = HydraClient::new();

    // Fetch run data from server
    let run_data = match client.get(&format!("/api/runs/{}", run_id)) {
        Ok(data) => data,
        Err(_) => {
            output::print_error("Could not fetch run data from server.");
            return;
        }
    };

    // Fetch steps from server
    let steps_data = match client.get(&format!("/api/steps?run_id={}", run_id)) {
        Ok(data) => data,
        Err(_) => {
            output::print_error("Could not fetch run steps from server.");
            return;
        }
    };

    output::print_header("Replay");
    println!();

    output::print_info(&format!("Run ID: {}", run_id));
    println!();

    // Show original run info
    let original_intent = run_data["intent"].as_str().unwrap_or("unknown");
    let original_time = run_data["started"].as_str().unwrap_or("unknown");
    let original_status = run_data["status"].as_str().unwrap_or("unknown");
    output::print_dimmed(&format!("Original intent: \"{}\"", original_intent));
    output::print_dimmed(&format!("Original time:   {}", original_time));
    let status_colored = match original_status {
        "complete" | "completed" => colors::green(original_status),
        "failed" => colors::red(original_status),
        _ => colors::yellow(original_status),
    };
    output::print_dimmed(&format!("Original status: {}", status_colored));
    println!();

    // Parse steps
    let steps: Vec<(String, String)> = steps_data
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|step| {
                    let phase = step["phase"].as_str().unwrap_or("unknown").to_string();
                    let detail = step["detail"].as_str().unwrap_or("").to_string();
                    (phase, detail)
                })
                .collect()
        })
        .unwrap_or_default();

    if steps.is_empty() {
        output::print_warning("No steps found for this run.");
        return;
    }

    if dry_run {
        output::print_warning("Dry-run mode — no actions will be taken");
        println!();
        output::print_dimmed("Steps that would execute:");
        println!();
        for (phase, detail) in &steps {
            println!(
                "  {} {} {}",
                colors::dim("[skip]"),
                colors::bold(phase),
                colors::dim(detail),
            );
        }
        println!();
        output::print_info("No actions taken in dry-run mode");
    } else {
        output::print_dimmed("Replaying cognitive phases:");
        println!();
        for (i, (phase, detail)) in steps.iter().enumerate() {
            println!(
                "  {} {} {}",
                colors::green(&format!("[{}/{}]", i + 1, steps.len())),
                colors::bold(phase),
                colors::dim(detail),
            );
        }
        println!();

        let tokens = run_data["tokens"].as_u64().unwrap_or(0);
        let duration = run_data["duration"].as_str().unwrap_or("--");
        output::print_box(&[
            &format!("Replay of {} complete", run_id),
            &format!("Phases: {}", steps.len()),
            &format!("Tokens: {}", output::format_tokens(tokens)),
            &format!("Duration: {}", duration),
        ]);
        println!();
        output::print_success("Replay complete");
    }
}
