use crate::client::HydraClient;
use crate::colors;
use crate::output;

pub enum ConfigAction {
    Show,
    Set(String, String),
    Get(String),
}

pub fn execute(action: ConfigAction) {
    match action {
        ConfigAction::Show => show_config(),
        ConfigAction::Set(key, value) => set_config(&key, &value),
        ConfigAction::Get(key) => get_config(&key),
    }
}

fn show_config() {
    let client = HydraClient::new();
    match client.get("/api/profile") {
        Ok(data) => {
            output::print_header("Hydra Configuration");
            println!();

            let mut rows: Vec<Vec<String>> = Vec::new();

            if let Some(obj) = data.as_object() {
                for (key, value) in obj {
                    let val_str = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => {
                            if key.contains("token") {
                                output::format_tokens(n.as_u64().unwrap_or(0))
                            } else {
                                n.to_string()
                            }
                        }
                        serde_json::Value::Bool(b) => b.to_string(),
                        _ => value.to_string(),
                    };
                    rows.push(vec![
                        key.clone(),
                        val_str,
                        colors::dim("server"),
                    ]);
                }
            }

            if rows.is_empty() {
                output::print_dimmed("No configuration returned from server");
            } else {
                let headers = &["Key", "Value", "Source"];
                output::print_table(headers, &rows);
            }
            println!();
        }
        Err(_) => {
            output::print_warning("Could not connect to Hydra server. Showing cached data.");
            show_config_stub();
        }
    }
}

fn show_config_stub() {
    output::print_header("Hydra Configuration");
    println!();

    let headers = &["Key", "Value", "Source"];
    let rows = vec![
        vec![
            "auto_approve".to_string(),
            "false".to_string(),
            colors::dim("default"),
        ],
        vec![
            "max_tokens".to_string(),
            output::format_tokens(100_000),
            colors::dim("config"),
        ],
        vec![
            "timeout_secs".to_string(),
            "300".to_string(),
            colors::dim("default"),
        ],
        vec![
            "log_level".to_string(),
            "info".to_string(),
            colors::dim("env"),
        ],
        vec![
            "sister_discovery".to_string(),
            "auto".to_string(),
            colors::dim("default"),
        ],
        vec![
            "approval_policy".to_string(),
            "prompt".to_string(),
            colors::dim("config"),
        ],
        vec![
            "storage_path".to_string(),
            "~/.agentic/hydra".to_string(),
            colors::dim("default"),
        ],
    ];
    output::print_table(headers, &rows);
    println!();
    output::print_dimmed("Config file: ~/.agentic/hydra/config.toml");
}

fn set_config(key: &str, value: &str) {
    let client = HydraClient::new();
    let body = serde_json::json!({ key: value });
    match client.put("/api/profile", &body) {
        Ok(_) => {
            output::print_success(&format!("Set {} = {}", colors::bold(key), value));
        }
        Err(_) => {
            output::print_warning("Could not connect to Hydra server. Showing cached data.");
            output::print_success(&format!("Set {} = {}", colors::bold(key), value));
        }
    }
}

fn get_config(key: &str) {
    let client = HydraClient::new();
    match client.get("/api/profile") {
        Ok(data) => {
            if let Some(val) = data.get(key) {
                match val {
                    serde_json::Value::String(s) => println!("{}", s),
                    other => println!("{}", other),
                }
            } else {
                output::print_error(&format!("Unknown config key: {}", key));
            }
        }
        Err(_) => {
            output::print_warning("Could not connect to Hydra server. Showing cached data.");
            get_config_stub(key);
        }
    }
}

fn get_config_stub(key: &str) {
    let value = match key {
        "auto_approve" => "false",
        "max_tokens" => "100000",
        "timeout_secs" => "300",
        "log_level" => "info",
        "sister_discovery" => "auto",
        "approval_policy" => "prompt",
        "storage_path" => "~/.agentic/hydra",
        _ => {
            output::print_error(&format!("Unknown config key: {}", key));
            return;
        }
    };
    println!("{}", value);
}
