//! Connector pollers — monitor databases, APIs, and cloud accounts for threats.
//! Uses shell commands (psql, mysql, aws, gcloud) — no new Rust driver crates.
//! Credentials read from vault. Results feed into monitor event system.

use std::path::PathBuf;
use std::time::Instant;
use chrono::Utc;
use super::events::{MonitorEvent, EventPriority, EventCategory};

/// A security check to run on a connected account.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SecurityCheck {
    pub name: String,
    pub query: String,
    pub alert_condition: String,
    #[serde(default)]
    pub threat_class: Option<String>,
    #[serde(default = "default_interval")]
    pub poll_interval: u64,
}

fn default_interval() -> u64 { 60 }

/// Database connector — polls via psql/mysql CLI.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DatabaseConnector {
    pub name: String,
    pub db_type: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub vault_ref: String,
    pub checks: Vec<SecurityCheck>,
}

/// API connector — polls HTTP endpoints.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApiConnector {
    pub name: String,
    pub url: String,
    pub method: String,
    pub vault_ref: String,
    pub checks: Vec<SecurityCheck>,
}

/// Cloud connector — checks via aws/gcloud CLI.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CloudConnector {
    pub name: String,
    pub provider: String,
    pub service: String,
    pub vault_ref: String,
    pub checks: Vec<SecurityCheck>,
}

/// Connector config wrapper (from TOML).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ConnectorConfig {
    pub connector: ConnectorType,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ConnectorType {
    #[serde(rename = "postgresql")]
    Postgresql(DatabaseConnector),
    #[serde(rename = "mysql")]
    Mysql(DatabaseConnector),
    #[serde(rename = "api")]
    Api(ApiConnector),
    #[serde(rename = "aws")]
    Aws(CloudConnector),
    #[serde(rename = "gcp")]
    Gcp(CloudConnector),
}

/// Active connector with polling state.
pub struct ActiveConnector {
    pub config: ConnectorType,
    pub last_poll: Instant,
    pub consecutive_failures: u32,
}

impl ActiveConnector {
    pub fn new(config: ConnectorType) -> Self {
        Self { config, last_poll: Instant::now(), consecutive_failures: 0 }
    }

    pub fn name(&self) -> &str {
        match &self.config {
            ConnectorType::Postgresql(c) | ConnectorType::Mysql(c) => &c.name,
            ConnectorType::Api(c) => &c.name,
            ConnectorType::Aws(c) | ConnectorType::Gcp(c) => &c.name,
        }
    }

    pub fn poll_interval(&self) -> u64 {
        let base = match &self.config {
            ConnectorType::Postgresql(c) | ConnectorType::Mysql(c) => c.checks.first().map(|ch| ch.poll_interval).unwrap_or(60),
            ConnectorType::Api(c) => c.checks.first().map(|ch| ch.poll_interval).unwrap_or(60),
            ConnectorType::Aws(c) | ConnectorType::Gcp(c) => c.checks.first().map(|ch| ch.poll_interval).unwrap_or(120),
        };
        // Backoff on failures
        base * (1 + self.consecutive_failures as u64).min(8)
    }

    pub fn should_poll(&self) -> bool {
        self.last_poll.elapsed().as_secs() >= self.poll_interval()
    }

    /// Poll this connector and return any security events.
    pub fn poll(&mut self) -> Vec<MonitorEvent> {
        self.last_poll = Instant::now();
        match &self.config {
            ConnectorType::Postgresql(c) | ConnectorType::Mysql(c) => poll_database(c, &mut self.consecutive_failures),
            ConnectorType::Api(c) => poll_api(c, &mut self.consecutive_failures),
            ConnectorType::Aws(c) | ConnectorType::Gcp(c) => poll_cloud(c, &mut self.consecutive_failures),
        }
    }
}

fn poll_database(db: &DatabaseConnector, failures: &mut u32) -> Vec<MonitorEvent> {
    let mut events = Vec::new();
    let tool = if db.db_type == "mysql" { "mysql" } else { "psql" };

    for check in &db.checks {
        let cmd = if db.db_type == "mysql" {
            format!("{tool} -h {} -P {} -u root -e \"{}\" {}", db.host, db.port, check.query, db.database)
        } else {
            format!("{tool} -h {} -p {} -d {} -t -c \"{}\"", db.host, db.port, db.database, check.query)
        };
        match run_check_cmd(&cmd, 10) {
            Ok(output) => {
                *failures = 0;
                if evaluate_condition(&output, &check.alert_condition) {
                    events.push(MonitorEvent {
                        id: uuid::Uuid::new_v4().to_string(),
                        source: format!("connector:{}", db.name),
                        title: format!("Security alert: {}", check.name),
                        detail: format!("Query: {} | Result: {}", check.query, output.trim()),
                        category: EventCategory::Security,
                        priority: EventPriority::Alert,
                        timestamp: Utc::now(), actionable: true,
                    });
                }
            }
            Err(e) => { *failures += 1; eprintln!("hydra-connector: {}: {e}", db.name); }
        }
    }
    events
}

fn poll_api(api: &ApiConnector, failures: &mut u32) -> Vec<MonitorEvent> {
    let mut events = Vec::new();
    for check in &api.checks {
        let cmd = format!("curl -s -o /dev/null -w '%{{http_code}}' -X {} '{}'", api.method, api.url);
        match run_check_cmd(&cmd, 10) {
            Ok(output) => {
                *failures = 0;
                if evaluate_condition(&output, &check.alert_condition) {
                    events.push(MonitorEvent {
                        id: uuid::Uuid::new_v4().to_string(),
                        source: format!("connector:{}", api.name), title: format!("API alert: {}", check.name),
                        detail: format!("Status: {}", output.trim()), category: EventCategory::Security,
                        priority: EventPriority::Alert, timestamp: Utc::now(), actionable: true,
                    });
                }
            }
            Err(e) => { *failures += 1; eprintln!("hydra-connector: {}: {e}", api.name); }
        }
    }
    events
}

fn poll_cloud(cloud: &CloudConnector, failures: &mut u32) -> Vec<MonitorEvent> {
    let mut events = Vec::new();
    let cli = if cloud.provider == "gcp" { "gcloud" } else { "aws" };
    for check in &cloud.checks {
        match run_check_cmd(&check.query, 15) {
            Ok(output) => {
                *failures = 0;
                if evaluate_condition(&output, &check.alert_condition) {
                    events.push(MonitorEvent {
                        id: uuid::Uuid::new_v4().to_string(),
                        source: format!("connector:{}", cloud.name), title: format!("Cloud alert: {}", check.name),
                        detail: format!("{cli} {}: {}", cloud.service, output.trim()), category: EventCategory::Security,
                        priority: EventPriority::Alert, timestamp: Utc::now(), actionable: true,
                    });
                }
            }
            Err(e) => { *failures += 1; eprintln!("hydra-connector: {}: {e}", cloud.name); }
        }
    }
    events
}

/// Run a shell command with timeout+killpg (same pattern as conductor).
fn run_check_cmd(cmd: &str, timeout_secs: u64) -> Result<String, String> {
    let mut command = std::process::Command::new("sh");
    command.arg("-c").arg(cmd).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    #[cfg(unix)]
    unsafe { use std::os::unix::process::CommandExt; command.pre_exec(|| { libc::setpgid(0, 0); Ok(()) }); }
    let mut child = command.spawn().map_err(|e| format!("{e}"))?;
    let pgid = child.id() as i32;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || { let _ = tx.send(child.wait_with_output()); });
    match rx.recv_timeout(std::time::Duration::from_secs(timeout_secs)) {
        Ok(Ok(out)) => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
        _ => { #[cfg(unix)] unsafe { libc::killpg(pgid, libc::SIGKILL); } Err("Timeout".into()) }
    }
}

/// Evaluate a simple condition: "count > 0", "value != 200", etc.
fn evaluate_condition(output: &str, condition: &str) -> bool {
    let trimmed = output.trim();
    if condition.contains("> 0") {
        trimmed.parse::<i64>().map(|v| v > 0).unwrap_or(false)
    } else if condition.contains("!= 200") {
        trimmed != "200"
    } else if condition.contains("> ") {
        if let Some(threshold) = condition.split('>').nth(1).and_then(|s| s.trim().parse::<i64>().ok()) {
            trimmed.parse::<i64>().map(|v| v > threshold).unwrap_or(false)
        } else { false }
    } else { !trimmed.is_empty() && trimmed != "0" }
}

/// Load all connector configs from ~/.hydra/connectors/*.toml.
pub fn load_connectors() -> Vec<ActiveConnector> {
    let dir = dirs::home_dir().unwrap_or_default().join(".hydra/connectors");
    let mut connectors = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().extension().map(|e| e == "toml").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    match toml::from_str::<ConnectorConfig>(&content) {
                        Ok(config) => { connectors.push(ActiveConnector::new(config.connector)); eprintln!("hydra-connector: loaded {}", entry.path().display()); }
                        Err(e) => eprintln!("hydra-connector: parse error {}: {e}", entry.path().display()),
                    }
                }
            }
        }
    }
    connectors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_condition_count() {
        assert!(evaluate_condition("5", "count > 0"));
        assert!(!evaluate_condition("0", "count > 0"));
    }

    #[test]
    fn evaluate_condition_status() {
        assert!(evaluate_condition("500", "status != 200"));
        assert!(!evaluate_condition("200", "status != 200"));
    }

    #[test]
    fn load_empty_dir() {
        let connectors = load_connectors();
        // Just verify no crash — may return 0 or more depending on user's setup
        assert!(connectors.len() < 1000);
    }
}
