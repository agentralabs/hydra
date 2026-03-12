use crate::client::HydraClient;
use crate::output;

pub fn freeze(run_id: Option<&str>) {
    let client = HydraClient::new();
    match run_id {
        Some(id) => {
            match client.post(
                &format!("/api/tasks/{}/freeze", id),
                &serde_json::json!({}),
            ) {
                Ok(_) => {
                    output::print_header(&format!("Freeze: {}", id));
                    println!();
                    output::print_warning(&format!("Run {} frozen - all actions paused", id));
                    output::print_info("Use 'hydra resume' to continue");
                }
                Err(_) => {
                    output::print_error(&format!(
                        "Could not reach Hydra server. Run {} was NOT frozen.",
                        id
                    ));
                }
            }
        }
        None => {
            match client.post("/api/tasks/freeze-all", &serde_json::json!({})) {
                Ok(_) => {
                    output::print_header("Freeze All");
                    println!();
                    output::print_warning("All active runs frozen");
                    output::print_info(
                        "Use 'hydra resume <run_id>' to resume individual runs",
                    );
                }
                Err(_) => {
                    output::print_error(
                        "Could not reach Hydra server. Runs were NOT frozen.",
                    );
                }
            }
        }
    }
}

pub fn resume(run_id: &str) {
    let client = HydraClient::new();
    match client.post(
        &format!("/api/tasks/{}/resume", run_id),
        &serde_json::json!({}),
    ) {
        Ok(_) => {
            output::print_header(&format!("Resume: {}", run_id));
            println!();
            output::print_success(&format!("Run {} resumed - execution continuing", run_id));
        }
        Err(_) => {
            output::print_error(&format!(
                "Could not reach Hydra server. Run {} was NOT resumed.",
                run_id
            ));
        }
    }
}

pub fn kill(run_id: Option<&str>) {
    let client = HydraClient::new();
    match run_id {
        Some(id) => {
            match client.post(
                &format!("/api/tasks/{}/kill", id),
                &serde_json::json!({}),
            ) {
                Ok(_) => {
                    output::print_header(&format!("Kill: {}", id));
                    println!();
                    output::print_error(&format!("Run {} killed - execution terminated", id));
                    output::print_info("Sister state has been preserved");
                }
                Err(_) => {
                    output::print_error(&format!(
                        "Could not reach Hydra server. Run {} was NOT killed.",
                        id
                    ));
                }
            }
        }
        None => {
            match client.post("/api/tasks/kill-all", &serde_json::json!({})) {
                Ok(_) => {
                    output::print_header("Kill All");
                    println!();
                    output::print_error("All active runs killed");
                    output::print_info("Sister state has been preserved");
                }
                Err(_) => {
                    output::print_error(
                        "Could not reach Hydra server. Runs were NOT killed.",
                    );
                }
            }
        }
    }
}
