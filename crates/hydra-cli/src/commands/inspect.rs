use crate::client::HydraClient;
use crate::colors;
use crate::output;

pub fn execute(run_id: &str, format: &str) {
    match format {
        "json" => print_json(run_id),
        "yaml" => print_yaml(run_id),
        _ => print_text(run_id),
    }
}

fn print_text(run_id: &str) {
    let client = HydraClient::new();
    match client.get(&format!("/api/tasks/{}", run_id)) {
        Ok(data) => {
            output::print_header(&format!("Inspect: {}", run_id));
            println!();

            let status = data["status"].as_str().unwrap_or("unknown");
            let intent = data["intent"].as_str().unwrap_or("unknown");
            let started = data["started"].as_str().unwrap_or("unknown");
            let finished = data["finished"].as_str().unwrap_or("unknown");
            let duration = data["duration"].as_str().unwrap_or("unknown");
            let tokens = data["tokens"].as_u64().unwrap_or(0);

            let headers = &["Field", "Value"];
            let rows = vec![
                vec!["Run ID".to_string(), run_id.to_string()],
                vec!["Status".to_string(), status.to_string()],
                vec!["Intent".to_string(), intent.to_string()],
                vec!["Started".to_string(), started.to_string()],
                vec!["Finished".to_string(), finished.to_string()],
                vec!["Duration".to_string(), duration.to_string()],
                vec!["Tokens".to_string(), output::format_tokens(tokens)],
            ];
            output::print_table(headers, &rows);

            // Phases
            if let Some(phases) = data["phases"].as_array() {
                println!();
                println!("  {}", colors::bold("Phases"));
                for phase_val in phases {
                    let name = phase_val["name"].as_str().unwrap_or("unknown");
                    let phase_tokens = phase_val["tokens"].as_u64().unwrap_or(0);
                    let dur = phase_val["duration"].as_str().unwrap_or("--");
                    println!(
                        "    {} {} {} {}",
                        colors::green("\u{2713}"),
                        format!("{:<10}", name),
                        colors::dim(&format!("{} tokens", output::format_tokens(phase_tokens))),
                        colors::dim(dur)
                    );
                }
            }

            // Sisters
            if let Some(sisters) = data["sisters"].as_array() {
                println!();
                println!("  {}", colors::bold("Sisters Used"));
                let sister_names: Vec<&str> =
                    sisters.iter().filter_map(|v| v.as_str()).collect();
                println!("    {}", sister_names.join(", "));
            }

            // Actions
            if let Some(actions) = data["actions"].as_array() {
                println!();
                println!("  {}", colors::bold("Actions"));
                for (i, action) in actions.iter().enumerate() {
                    let action_type = action["type"].as_str().unwrap_or("unknown");
                    let target = action["target"].as_str().unwrap_or("unknown");
                    let approved = action["approved"].as_bool();
                    let suffix = match approved {
                        Some(true) => " (approved)",
                        Some(false) => " (denied)",
                        None => "",
                    };
                    println!(
                        "    {}. {} {}{}",
                        i + 1,
                        action_type.chars().next().unwrap_or(' ').to_uppercase(),
                        target,
                        suffix
                    );
                }
            }

            println!();
        }
        Err(_) => {
            output::print_error(&format!(
                "Could not reach Hydra server. Cannot inspect run '{}'.",
                run_id
            ));
        }
    }
}

fn print_json(run_id: &str) {
    let client = HydraClient::new();
    match client.get(&format!("/api/tasks/{}", run_id)) {
        Ok(data) => {
            if let Ok(pretty) = serde_json::to_string_pretty(&data) {
                println!("{}", pretty);
            } else {
                println!("{}", data);
            }
        }
        Err(_) => {
            output::print_error(&format!(
                "Could not reach Hydra server. Cannot inspect run '{}'.",
                run_id
            ));
        }
    }
}

fn print_yaml(run_id: &str) {
    let client = HydraClient::new();
    match client.get(&format!("/api/tasks/{}", run_id)) {
        Ok(data) => {
            print_yaml_from_json(run_id, &data);
        }
        Err(_) => {
            output::print_error(&format!(
                "Could not reach Hydra server. Cannot inspect run '{}'.",
                run_id
            ));
        }
    }
}

fn print_yaml_from_json(run_id: &str, data: &serde_json::Value) {
    println!("run_id: {}", run_id);
    if let Some(obj) = data.as_object() {
        for (key, value) in obj {
            if key == "phases" || key == "actions" || key == "sisters" {
                continue;
            }
            match value {
                serde_json::Value::String(s) => println!("{}: \"{}\"", key, s),
                serde_json::Value::Number(n) => println!("{}: {}", key, n),
                serde_json::Value::Bool(b) => println!("{}: {}", key, b),
                _ => println!("{}: {}", key, value),
            }
        }
    }

    if let Some(phases) = data["phases"].as_array() {
        println!("phases:");
        for phase in phases {
            let name = phase["name"].as_str().unwrap_or("unknown");
            let tokens = phase["tokens"].as_u64().unwrap_or(0);
            println!("  - name: {}", name);
            println!("    tokens: {}", tokens);
        }
    }

    if let Some(sisters) = data["sisters"].as_array() {
        println!("sisters:");
        for s in sisters {
            if let Some(name) = s.as_str() {
                println!("  - {}", name);
            }
        }
    }

    if let Some(actions) = data["actions"].as_array() {
        println!("actions:");
        for action in actions {
            let atype = action["type"].as_str().unwrap_or("unknown");
            let target = action["target"].as_str().unwrap_or("unknown");
            println!("  - type: {}", atype);
            println!("    target: {}", target);
            if let Some(approved) = action["approved"].as_bool() {
                println!("    approved: {}", approved);
            }
        }
    }
}
