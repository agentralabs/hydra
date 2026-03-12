//! Anomaly detection — burst detection, destructive pattern matching,
//! scope creep tracking, and exfiltration pattern detection.

use hydra_autonomy::ActionRisk;
use hydra_autonomy::AutonomyLevel;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Decision context passed from DECIDE to ACT
#[derive(Debug, Clone)]
pub struct DecideResult {
    pub allowed: bool,
    pub requires_approval: bool,
    pub autonomy_level: AutonomyLevel,
    pub trust_score: f64,
    pub risk_level: ActionRisk,
    pub reason: String,
}

/// Result of evaluating a specific command through the full security pipeline
#[derive(Debug, Clone)]
pub struct CommandGateResult {
    pub allowed: bool,
    pub risk_score: f64,
    pub risk_level: String,
    pub reason: String,
    pub boundary_blocked: bool,
    pub anomaly_detected: bool,
}

/// Simple anomaly detector — tracks action frequency and scope
pub struct AnomalyDetector {
    /// Number of commands executed in the current burst window
    burst_count: AtomicU64,
    /// Timestamp of the burst window start
    burst_window_start: parking_lot::Mutex<Instant>,
    /// Paths accessed in this session (for scope creep detection)
    accessed_paths: parking_lot::Mutex<Vec<String>>,
    /// Total commands executed this session
    total_commands: AtomicU64,
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self {
            burst_count: AtomicU64::new(0),
            burst_window_start: parking_lot::Mutex::new(Instant::now()),
            accessed_paths: parking_lot::Mutex::new(Vec::new()),
            total_commands: AtomicU64::new(0),
        }
    }

    /// Check for anomalies before executing a command.
    /// Returns Some(reason) if anomaly detected.
    pub fn check(&self, command: &str) -> Option<String> {
        self.total_commands.fetch_add(1, Ordering::Relaxed);

        // ── Burst detection: >20 commands in 10 seconds ──
        {
            let mut start = self.burst_window_start.lock();
            if start.elapsed().as_secs() > 10 {
                // Reset window
                *start = Instant::now();
                self.burst_count.store(1, Ordering::Relaxed);
            } else {
                let count = self.burst_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count > 20 {
                    return Some(format!(
                        "Burst detected: {} commands in {:.1}s (limit: 20/10s). Possible automated attack or runaway loop.",
                        count,
                        start.elapsed().as_secs_f64()
                    ));
                }
            }
        }

        // ── Destructive pattern detection ──
        let lower = command.to_lowercase();
        // Patterns that must match exactly (not as prefix of a longer path)
        let exact_path_patterns = [
            ("rm -rf /", "Recursive delete from root"),
            ("rm -rf ~", "Recursive delete from home"),
            ("rm -rf /*", "Wildcard delete from root"),
            ("chmod -r 777 /", "Recursive permission change on root"),
            ("mv / /dev/null", "Move root to null"),
        ];
        for (pattern, desc) in &exact_path_patterns {
            // "rm -rf /" must be the END of command or followed by space/;/|
            // NOT followed by a path component like "tmp/"
            if let Some(pos) = lower.find(pattern) {
                let after = pos + pattern.len();
                let next_char = lower.chars().nth(after);
                // If pattern ends with /, check that the next char isn't an alphanumeric path
                if pattern.ends_with('/') {
                    match next_char {
                        None => return Some(format!(
                            "CRITICAL: Destructive pattern detected — {}. Command: {}",
                            desc, command
                        )),
                        Some(c) if c == '*' || c == ' ' || c == ';' || c == '|' || c == '&' => return Some(format!(
                            "CRITICAL: Destructive pattern detected — {}. Command: {}",
                            desc, command
                        )),
                        _ => {} // "rm -rf /tmp/..." is NOT destructive — it's a specific path
                    }
                } else {
                    return Some(format!(
                        "CRITICAL: Destructive pattern detected — {}. Command: {}",
                        desc, command
                    ));
                }
            }
        }
        // Patterns that are always dangerous regardless of context
        let always_dangerous = [
            ("mkfs", "Filesystem format"),
            ("dd if=/dev/", "Raw disk write"),
            (":(){:|:&};:", "Fork bomb"),
            ("| sh", "Remote code execution via pipe to shell"),
            ("| bash", "Remote code execution via pipe to bash"),
            ("> /dev/sda", "Direct disk overwrite"),
        ];
        for (pattern, desc) in &always_dangerous {
            if lower.contains(pattern) {
                return Some(format!(
                    "CRITICAL: Destructive pattern detected — {}. Command: {}",
                    desc, command
                ));
            }
        }

        // ── Scope creep: accessing too many distinct directories ──
        {
            let mut paths = self.accessed_paths.lock();
            // Extract directory from command (rough heuristic)
            for word in command.split_whitespace() {
                if word.contains('/') && !word.starts_with('-') {
                    let dir = if let Some(idx) = word.rfind('/') {
                        &word[..idx]
                    } else {
                        word
                    };
                    if !paths.contains(&dir.to_string()) {
                        paths.push(dir.to_string());
                    }
                }
            }
            // If accessing >50 distinct directories in one session, flag it
            if paths.len() > 50 {
                return Some(format!(
                    "Scope creep: {} distinct directories accessed this session. Possible data exfiltration.",
                    paths.len()
                ));
            }
        }

        // ── Exfiltration patterns ──
        if (lower.contains("curl") || lower.contains("wget") || lower.contains("nc "))
            && (lower.contains(".ssh") || lower.contains(".env") || lower.contains("password")
                || lower.contains("secret") || lower.contains("token") || lower.contains("credential"))
        {
            return Some(format!(
                "Potential exfiltration: network command accessing sensitive data. Command: {}",
                command
            ));
        }

        None
    }

    /// Get current stats for reporting
    pub fn stats(&self) -> (u64, usize) {
        let total = self.total_commands.load(Ordering::Relaxed);
        let paths = self.accessed_paths.lock().len();
        (total, paths)
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new()
    }
}
