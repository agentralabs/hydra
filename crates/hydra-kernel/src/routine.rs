//! Daily Routine System — scheduled training regimen for capability building.
//! Routines are TOML files in ~/.hydra/routines/. Dropped via gateway or shipped default.
//! Proactive engine fires them on schedule. Conductor executes. Genome learns.

use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A routine configuration loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutineConfig {
    pub routine: RoutineMeta,
    #[serde(default)]
    pub steps: Vec<RoutineStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutineMeta {
    pub name: String,
    pub description: String,
    pub capability_area: String,
    pub schedule: String,        // "daily 09:00" | "weekly mon 10:00" | "hourly" | "every 30m"
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_difficulty")]
    pub difficulty: u8,          // 1-5
    pub day_start: Option<u32>,  // Active from training day N
    pub day_end: Option<u32>,    // Active until training day N
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutineStep {
    pub goal: String,
    #[serde(default = "default_step_type")]
    pub step_type: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    pub success_criteria: Option<String>,
}

fn default_true() -> bool { true }
fn default_difficulty() -> u8 { 1 }
fn default_step_type() -> String { "shell".into() }
fn default_timeout() -> u64 { 60 }

/// A record of a routine execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutineRunRecord {
    pub routine_name: String,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub steps_completed: usize,
    pub steps_total: usize,
    pub duration_ms: u64,
    pub capability_area: String,
}

/// Load all routines from ~/.hydra/routines/.
pub fn load_routines() -> Vec<RoutineConfig> {
    let dir = routines_dir();
    let mut routines = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    match toml::from_str::<RoutineConfig>(&content) {
                        Ok(r) if r.routine.enabled => routines.push(r),
                        Ok(_) => {} // disabled
                        Err(e) => eprintln!("hydra-routine: parse error {}: {e}", path.display()),
                    }
                }
            }
        }
    }
    routines.sort_by(|a, b| a.routine.name.cmp(&b.routine.name));
    routines
}

/// Check if a routine should fire now based on schedule and last run.
pub fn should_fire(routine: &RoutineConfig, now: DateTime<Utc>) -> bool {
    // Check day range (training day = days since first routine installed)
    if let Some(start) = routine.routine.day_start {
        let training_day = training_day_number();
        if training_day < start { return false; }
    }
    if let Some(end) = routine.routine.day_end {
        let training_day = training_day_number();
        if training_day > end { return false; }
    }

    let last = last_run_time(&routine.routine.name);
    let schedule = &routine.routine.schedule;

    if schedule.starts_with("daily") {
        // "daily 09:00" → fire once per day at ~09:00
        let parts: Vec<&str> = schedule.split_whitespace().collect();
        let (target_h, target_m) = parse_time(parts.get(1).unwrap_or(&"09:00"));
        let now_h = now.hour() as u8;
        let now_m = now.minute() as u8;
        // Within 30-minute window of target time
        let in_window = now_h == target_h && now_m >= target_m && now_m < target_m + 30;
        let already_ran_today = last.map(|l| l.date_naive() == now.date_naive()).unwrap_or(false);
        in_window && !already_ran_today
    } else if schedule.starts_with("weekly") {
        // "weekly mon 10:00"
        let parts: Vec<&str> = schedule.split_whitespace().collect();
        let target_day = parse_weekday(parts.get(1).unwrap_or(&"mon"));
        let (target_h, _) = parse_time(parts.get(2).unwrap_or(&"10:00"));
        let now_day = now.weekday().num_days_from_monday() as u8;
        let already_ran = last.map(|l| (now - l).num_hours() < 24).unwrap_or(false);
        now_day == target_day && now.hour() as u8 >= target_h && !already_ran
    } else if schedule == "hourly" {
        last.map(|l| (now - l).num_minutes() >= 55).unwrap_or(true)
    } else if schedule.starts_with("every") {
        // "every 30m"
        let parts: Vec<&str> = schedule.split_whitespace().collect();
        let interval = parse_interval(parts.get(1).unwrap_or(&"60m"));
        last.map(|l| (now - l).num_minutes() >= interval).unwrap_or(true)
    } else {
        false
    }
}

/// Record a routine run in history.
pub fn record_run(record: &RoutineRunRecord) {
    let path = routines_dir().join("history.jsonl");
    if let Ok(json) = serde_json::to_string(record) {
        match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut f) => { use std::io::Write; let _ = writeln!(f, "{json}"); }
            Err(e) => eprintln!("hydra-routine: history write failed: {e}"),
        }
    }
}

/// Get training day number (days since first routine was installed).
pub fn training_day_number() -> u32 {
    let marker = routines_dir().join(".training-start");
    if let Ok(content) = std::fs::read_to_string(&marker) {
        if let Ok(start) = DateTime::parse_from_rfc3339(content.trim()) {
            return (Utc::now() - start.with_timezone(&Utc)).num_days().max(1) as u32;
        }
    }
    // First run — create marker
    let _ = std::fs::write(&marker, Utc::now().to_rfc3339());
    1
}

/// Training progress summary across all capability areas.
pub fn training_progress() -> Vec<(String, usize, usize, f64)> {
    // (area, total_runs, successes, success_rate)
    let path = routines_dir().join("history.jsonl");
    let mut by_area: std::collections::HashMap<String, (usize, usize)> = std::collections::HashMap::new();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            if let Ok(record) = serde_json::from_str::<RoutineRunRecord>(line) {
                let entry = by_area.entry(record.capability_area).or_default();
                entry.0 += 1;
                if record.success { entry.1 += 1; }
            }
        }
    }
    let mut result: Vec<_> = by_area.into_iter().map(|(area, (total, success))| {
        let rate = if total > 0 { success as f64 / total as f64 } else { 0.0 };
        (area, total, success, rate)
    }).collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

/// Schedule label for display.
pub fn schedule_label(routine: &RoutineConfig) -> String {
    routine.routine.schedule.clone()
}

fn last_run_time(name: &str) -> Option<DateTime<Utc>> {
    let path = routines_dir().join("history.jsonl");
    let content = std::fs::read_to_string(&path).ok()?;
    content.lines().rev()
        .filter_map(|l| serde_json::from_str::<RoutineRunRecord>(l).ok())
        .find(|r| r.routine_name == name)
        .map(|r| r.timestamp)
}

fn parse_time(s: &str) -> (u8, u8) {
    let parts: Vec<&str> = s.split(':').collect();
    let h = parts.first().and_then(|p| p.parse().ok()).unwrap_or(9);
    let m = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    (h, m)
}

fn parse_weekday(s: &str) -> u8 {
    match s.to_lowercase().as_str() {
        "mon" => 0, "tue" => 1, "wed" => 2, "thu" => 3,
        "fri" => 4, "sat" => 5, "sun" => 6, _ => 0,
    }
}

fn parse_interval(s: &str) -> i64 {
    let s = s.trim();
    if s.ends_with('m') { s[..s.len()-1].parse().unwrap_or(60) }
    else if s.ends_with('h') { s[..s.len()-1].parse::<i64>().unwrap_or(1) * 60 }
    else { s.parse().unwrap_or(60) }
}

fn routines_dir() -> PathBuf {
    let dir = dirs::home_dir().unwrap_or_default().join(".hydra/routines");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_time_works() {
        assert_eq!(parse_time("09:30"), (9, 30));
        assert_eq!(parse_time("14:00"), (14, 0));
    }

    #[test]
    fn parse_weekday_works() {
        assert_eq!(parse_weekday("mon"), 0);
        assert_eq!(parse_weekday("fri"), 4);
    }

    #[test]
    fn parse_interval_works() {
        assert_eq!(parse_interval("30m"), 30);
        assert_eq!(parse_interval("2h"), 120);
    }

    #[test]
    fn training_day_starts_at_1() {
        assert!(training_day_number() >= 1);
    }
}
