use crate::client::HydraClient;
use crate::colors;
use crate::output;

use std::fs;
use std::path::PathBuf;

fn profile_path() -> PathBuf {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".hydra").join("profile.json")
}

fn default_profile() -> String {
    r#"{
  "name": "default",
  "created": "2026-03-07T00:00:00Z",
  "onboarding_complete": false,
  "preferences": {
    "auto_approve": false,
    "verbose": false,
    "color": true
  }
}"#
    .to_string()
}

fn load_profile() -> String {
    let path = profile_path();
    fs::read_to_string(&path).unwrap_or_else(|_| default_profile())
}

fn extract_json_field<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\"", key);
    let pos = json.find(&needle)?;
    let after_key = &json[pos + needle.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();

    if after_colon.starts_with('"') {
        let start = 1;
        let end = after_colon[start..].find('"')?;
        Some(&after_colon[start..start + end])
    } else if after_colon.starts_with('{') {
        // Return the object as a summary
        Some("{...}")
    } else {
        let end = after_colon
            .find(|c: char| c == ',' || c == '}' || c == '\n')
            .unwrap_or(after_colon.len());
        Some(after_colon[..end].trim())
    }
}

pub fn show() {
    let client = HydraClient::new();
    match client.get("/api/profile") {
        Ok(data) => {
            output::print_header("Profile");
            println!();

            let name = data["name"].as_str().unwrap_or("unknown");
            let created = data["created"].as_str().unwrap_or("unknown");
            let onboarding = data["onboarding_complete"].as_bool().unwrap_or(false);

            let rows = vec![
                vec!["Name".to_string(), colors::bold(name)],
                vec!["Created".to_string(), created.to_string()],
                vec![
                    "Onboarding".to_string(),
                    if onboarding {
                        colors::green("complete")
                    } else {
                        colors::yellow("pending")
                    },
                ],
                vec![
                    "Preferences".to_string(),
                    if let Some(prefs) = data.get("preferences") {
                        prefs.to_string()
                    } else {
                        colors::dim("(see profile.json)")
                    },
                ],
            ];

            output::print_table(&["Field", "Value"], &rows);

            println!();
            output::print_dimmed("Source: Hydra server");
        }
        Err(_) => {
            output::print_warning("Could not connect to Hydra server. Showing local data.");
            show_local();
        }
    }
}

fn show_local() {
    output::print_header("Profile");
    println!();

    let json = load_profile();
    let name = extract_json_field(&json, "name").unwrap_or("unknown");
    let created = extract_json_field(&json, "created").unwrap_or("unknown");
    let onboarding = extract_json_field(&json, "onboarding_complete").unwrap_or("false");

    let rows = vec![
        vec!["Name".to_string(), colors::bold(name)],
        vec!["Created".to_string(), created.to_string()],
        vec![
            "Onboarding".to_string(),
            if onboarding == "true" {
                colors::green("complete")
            } else {
                colors::yellow("pending")
            },
        ],
        vec![
            "Preferences".to_string(),
            colors::dim("(see profile.json)"),
        ],
    ];

    output::print_table(&["Field", "Value"], &rows);

    let path = profile_path();
    println!();
    output::print_dimmed(&format!("Profile path: {}", path.display()));
}

pub fn set_name(name: &str) {
    let client = HydraClient::new();
    match client.put(
        "/api/profile/name",
        &serde_json::json!({ "name": name }),
    ) {
        Ok(_) => {
            output::print_header("Profile");
            println!();
            output::print_success(&format!("Display name set to \"{}\"", name));
            output::print_dimmed("Saved to server");
        }
        Err(_) => {
            output::print_warning("Could not connect to Hydra server. Saving locally.");
            set_name_local(name);
        }
    }
}

fn set_name_local(name: &str) {
    output::print_header("Profile");
    println!();

    let path = profile_path();

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut json = load_profile();

    // Replace name in JSON
    if let Some(start) = json.find("\"name\"") {
        let after = &json[start..];
        if let Some(colon) = after.find(':') {
            let val_start = start + colon + 1;
            let rest = json[val_start..].trim_start();
            if rest.starts_with('"') {
                let offset = json[val_start..].find('"').unwrap() + val_start + 1;
                let end_quote = json[offset..].find('"').unwrap() + offset;
                json = format!("{}\"{}\"{}",
                    &json[..offset],
                    name,
                    &json[end_quote..]
                );
            }
        }
    }

    match fs::write(&path, &json) {
        Ok(_) => {
            output::print_success(&format!("Display name set to \"{}\"", name));
            output::print_dimmed(&format!("Saved to {}", path.display()));
        }
        Err(e) => {
            output::print_error(&format!("Failed to save profile: {}", e));
        }
    }
}

pub fn reset() {
    output::print_header("Profile");
    println!();

    output::print_warning("This will reset your profile to defaults.");

    let path = profile_path();

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::write(&path, default_profile()) {
        Ok(_) => {
            output::print_success("Profile reset to defaults");
            output::print_dimmed(&format!("Saved to {}", path.display()));
        }
        Err(e) => {
            output::print_error(&format!("Failed to reset profile: {}", e));
        }
    }
}
