use crate::client::HydraClient;
use crate::colors;
use crate::output;

use std::fs;
use std::path::PathBuf;

fn logs_dir() -> PathBuf {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".hydra").join("logs")
}

fn level_color(level: &str) -> String {
    match level {
        "error" => colors::red(level),
        "warn" => colors::yellow(level),
        "info" => colors::blue(level),
        _ => colors::dim(level),
    }
}

struct LogEntry {
    timestamp: String,
    level: String,
    message: String,
}

fn read_local_log_files(level_filter: Option<&str>) -> Vec<LogEntry> {
    let dir = logs_dir();

    if !dir.exists() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    if let Ok(files) = fs::read_dir(&dir) {
        let mut paths: Vec<PathBuf> = files.filter_map(|f| f.ok().map(|f| f.path())).collect();
        paths.sort();

        for path in paths {
            if let Ok(content) = fs::read_to_string(&path) {
                for line in content.lines() {
                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                    if parts.len() >= 3 {
                        let level = parts[1]
                            .trim_start_matches('[')
                            .trim_end_matches(']')
                            .to_lowercase();
                        if let Some(filter) = level_filter {
                            if level != filter {
                                continue;
                            }
                        }
                        entries.push(LogEntry {
                            timestamp: parts[0].to_string(),
                            level,
                            message: parts[2].to_string(),
                        });
                    }
                }
            }
        }
    }

    entries
}

fn parse_server_logs(data: &serde_json::Value, level_filter: Option<&str>) -> Vec<LogEntry> {
    data.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    let timestamp = entry["timestamp"].as_str()?.to_string();
                    let level = entry["level"].as_str()?.to_string();
                    let message = entry["message"].as_str()?.to_string();
                    if let Some(filter) = level_filter {
                        if level != filter {
                            return None;
                        }
                    }
                    Some(LogEntry {
                        timestamp,
                        level,
                        message,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn execute(follow: bool, level_filter: Option<&str>) {
    output::print_header("Logs");
    println!();

    if let Some(lvl) = level_filter {
        output::print_dimmed(&format!("Filter: level={}", lvl));
        println!();
    }

    // Try server first
    let client = HydraClient::new();
    let entries = match client.get("/api/logs") {
        Ok(data) => parse_server_logs(&data, level_filter),
        Err(_) => {
            // Fall back to local files with clear label
            output::print_warning("Reading local log files (server unreachable)");
            println!();
            let local = read_local_log_files(level_filter);
            if local.is_empty() {
                output::print_info("No local log entries found");
                return;
            }
            local
        }
    };

    if entries.is_empty() {
        output::print_info("No log entries found");
        return;
    }

    for entry in &entries {
        println!(
            "  {} {} {}",
            colors::dim(&entry.timestamp),
            level_color(&entry.level),
            entry.message,
        );
    }

    println!();

    if follow {
        output::print_info(&format!(
            "Tailing logs from {} ...",
            logs_dir().display()
        ));
        output::print_dimmed("(Press Ctrl+C to stop)");
    } else {
        output::print_dimmed(&format!("Showing {} entries", entries.len()));
        output::print_dimmed(&format!("Log directory: {}", logs_dir().display()));
    }
}
