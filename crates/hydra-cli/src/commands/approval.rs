use crate::client::HydraClient;
use crate::colors;
use crate::output;

pub fn print_approval_card(action: &str, risk: &str, preview: &str) {
    let risk_colored = match risk {
        "high" => colors::red(risk),
        "medium" => colors::yellow(risk),
        "low" => colors::green(risk),
        _ => colors::dim(risk),
    };

    println!();
    output::print_box(&[
        &format!("Action:  {}", action),
        &format!("Risk:    {}", risk),
        &format!("Preview: {}", preview),
    ]);
    // Re-print risk with color outside box (box strips ANSI)
    println!(
        "  {} Risk level: {}",
        colors::yellow("\u{26a0}"),
        risk_colored
    );
    println!();
}

pub fn approve(run_id: &str) {
    let client = HydraClient::new();
    match client.post(
        &format!("/api/tasks/{}/approve", run_id),
        &serde_json::json!({}),
    ) {
        Ok(data) => {
            output::print_header(&format!("Approve: {}", run_id));
            println!();

            let action = data["action"].as_str().unwrap_or("Execute planned mutations");
            let risk = data["risk"].as_str().unwrap_or("medium");
            let preview = data["preview"]
                .as_str()
                .unwrap_or("3 files modified, 1 created");

            print_approval_card(action, risk, preview);
            output::print_success(&format!("Run {} approved - resuming execution", run_id));
        }
        Err(_) => {
            output::print_error("Could not reach Hydra server. Approval not sent.");
        }
    }
}

pub fn deny(run_id: &str, reason: Option<&str>) {
    let client = HydraClient::new();
    let body = serde_json::json!({
        "reason": reason.unwrap_or(""),
    });
    match client.post(&format!("/api/tasks/{}/deny", run_id), &body) {
        Ok(data) => {
            output::print_header(&format!("Deny: {}", run_id));
            println!();

            let action = data["action"].as_str().unwrap_or("Execute planned mutations");
            let risk = data["risk"].as_str().unwrap_or("medium");
            let preview = data["preview"]
                .as_str()
                .unwrap_or("3 files modified, 1 created");

            print_approval_card(action, risk, preview);

            if let Some(r) = reason {
                output::print_info(&format!("Reason: {}", r));
            }
            output::print_warning(&format!("Run {} denied - execution halted", run_id));
        }
        Err(_) => {
            output::print_error("Could not reach Hydra server. Denial not sent.");
        }
    }
}
