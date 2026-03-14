//! Gateway helpers — local fallback functions, types, and result parsers.
//!
//! This is where local logic LIVES — clearly labeled as fallback.
//! Every function here is a last resort when the sister is offline.

use std::path::{Path, PathBuf};

// ── TYPES ──

/// Risk level from contract/aegis assessment.
#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::None => write!(f, "none"),
            RiskLevel::Low => write!(f, "low"),
            RiskLevel::Medium => write!(f, "medium"),
            RiskLevel::High => write!(f, "high"),
            RiskLevel::Critical => write!(f, "critical"),
        }
    }
}

/// Safety check result from aegis validation.
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyResult {
    Safe,
    Suspicious(String),
    Blocked(String),
    Unknown,
}

/// Time context from time sister.
#[derive(Debug, Clone)]
pub struct TimeContext {
    pub raw: String,
}

/// Environment info from reality sister.
#[derive(Debug, Clone)]
pub struct EnvironmentInfo {
    pub raw: String,
}

// ── RESULT PARSERS ──

/// Extract a file path from codebase sister search result text.
pub fn extract_file_path_from_result(text: &str) -> Option<PathBuf> {
    // Sister results often contain file paths — find the first valid-looking one
    for line in text.lines() {
        let trimmed = line.trim();
        // Skip empty lines and headers
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("---") {
            continue;
        }
        // Look for path-like strings (contains / and ends with a file extension or is a path)
        for word in trimmed.split_whitespace() {
            let clean = word.trim_matches(|c: char| c == '"' || c == '\'' || c == ',' || c == ':');
            if clean.contains('/') && !clean.starts_with("http") {
                let p = PathBuf::from(clean);
                if p.extension().is_some() || clean.ends_with('/') {
                    if p.exists() { return Some(p); }
                    // Try relative to cwd
                    let abs = std::env::current_dir().ok()?.join(&p);
                    if abs.exists() { return Some(abs); }
                }
            }
        }
        // Also try: the entire trimmed line as a path
        let p = PathBuf::from(trimmed);
        if p.exists() { return Some(p); }
    }
    None
}

/// Parse memory query results into individual facts.
pub fn parse_memory_results(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "---" || trimmed.starts_with("No memories") {
            continue;
        }
        // Strip leading "- " bullet markers
        let clean = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        if !clean.is_empty() {
            results.push(clean.to_string());
        }
    }
    results
}

/// Parse risk level from sister result text.
pub fn parse_risk_level(text: &str) -> RiskLevel {
    let lower = text.to_lowercase();
    if lower.contains("critical") || lower.contains("blocked") || lower.contains("dangerous") {
        RiskLevel::Critical
    } else if lower.contains("high") || lower.contains("destructive") {
        RiskLevel::High
    } else if lower.contains("medium") || lower.contains("moderate") || lower.contains("caution") {
        RiskLevel::Medium
    } else if lower.contains("low") || lower.contains("minor") {
        RiskLevel::Low
    } else {
        RiskLevel::None
    }
}

/// Parse safety result from aegis sister output.
pub fn parse_safety_result(text: &str) -> SafetyResult {
    let lower = text.to_lowercase();
    if lower.contains("blocked") || lower.contains("rejected") || lower.contains("denied") {
        SafetyResult::Blocked(text.to_string())
    } else if lower.contains("suspicious") || lower.contains("warning") || lower.contains("caution") {
        SafetyResult::Suspicious(text.to_string())
    } else if lower.contains("safe") || lower.contains("allowed") || lower.contains("clean") {
        SafetyResult::Safe
    } else {
        SafetyResult::Unknown
    }
}

/// Parse time context from time sister output.
pub fn parse_time_context(text: &str) -> TimeContext {
    TimeContext { raw: text.to_string() }
}

// ── LOCAL FALLBACKS ──

/// LOCAL FALLBACK: Find a file using filesystem search.
/// Only used when Codebase sister is offline.
pub fn local_find_file(name: &str, project_root: &Path) -> Option<PathBuf> {
    eprintln!("[hydra:gateway:fallback] local file search for: {}", name);
    // Common directories to search
    let search_dirs = ["test-specs", "specs", "spec", "docs", "src", "."];
    for dir in &search_dirs {
        let candidate = project_root.join(dir).join(name);
        if candidate.exists() { return Some(candidate); }
    }
    // Try find command as last resort
    let output = std::process::Command::new("find")
        .args([project_root.to_str().unwrap_or("."), "-name", name, "-type", "f"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().next().map(|l| PathBuf::from(l.trim()))
}

/// LOCAL FALLBACK: Risk assessment using command pattern matching.
/// Only used when Contract + Aegis sisters are offline.
pub fn local_risk_assessment(command: &str) -> RiskLevel {
    eprintln!("[hydra:gateway:fallback] local risk assessment for command");
    let lower = command.to_lowercase();
    let destructive = ["rm -rf", "drop table", "delete from", "format ", "mkfs",
        "dd if=", "> /dev/", "truncate", "shred"];
    let dangerous = ["rm ", "kill ", "pkill", "shutdown", "reboot",
        "git push --force", "git reset --hard", "chmod 777"];
    let moderate = ["git push", "npm publish", "cargo publish", "docker push",
        "curl -X DELETE", "curl -X PUT"];

    if destructive.iter().any(|p| lower.contains(p)) { return RiskLevel::Critical; }
    if dangerous.iter().any(|p| lower.contains(p)) { return RiskLevel::High; }
    if moderate.iter().any(|p| lower.contains(p)) { return RiskLevel::Medium; }
    RiskLevel::Low
}

/// LOCAL FALLBACK: Safety check using basic blocklist.
/// Only used when Aegis sister is offline.
pub fn local_safety_check(input: &str) -> SafetyResult {
    eprintln!("[hydra:gateway:fallback] local safety check");
    let lower = input.to_lowercase();
    let blocked = ["eval(", "exec(", "<script>", "javascript:", "data:text/html",
        "'; drop table", "\" or 1=1", "$()", "`"];
    for pattern in &blocked {
        if lower.contains(pattern) {
            return SafetyResult::Blocked(format!("Local blocklist: contains '{}'", pattern));
        }
    }
    SafetyResult::Safe
}

/// LOCAL FALLBACK: Time context from system clock.
/// Only used when Time sister is offline.
pub fn local_time_context() -> TimeContext {
    eprintln!("[hydra:gateway:fallback] local time context");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    TimeContext { raw: format!("unix_timestamp: {}", now) }
}

/// LOCAL FALLBACK: Grep for code search.
/// Only used when Codebase sister is offline.
pub fn local_grep(query: &str, project_root: &Path) -> Vec<String> {
    eprintln!("[hydra:gateway:fallback] local grep for: {}", query);
    let output = std::process::Command::new("grep")
        .args(["-rn", "--include=*.rs", query, project_root.to_str().unwrap_or(".")])
        .output();
    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.lines().take(20).map(String::from).collect()
        }
        Err(_) => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_risk_level() {
        assert_eq!(parse_risk_level("This is a critical action"), RiskLevel::Critical);
        assert_eq!(parse_risk_level("high risk detected"), RiskLevel::High);
        assert_eq!(parse_risk_level("moderate risk"), RiskLevel::Medium);
        assert_eq!(parse_risk_level("low risk"), RiskLevel::Low);
        assert_eq!(parse_risk_level("all clear"), RiskLevel::None);
    }

    #[test]
    fn test_parse_safety_result() {
        assert_eq!(parse_safety_result("Input is safe and allowed"), SafetyResult::Safe);
        assert!(matches!(parse_safety_result("Blocked: injection detected"), SafetyResult::Blocked(_)));
        assert!(matches!(parse_safety_result("Warning: suspicious pattern"), SafetyResult::Suspicious(_)));
        assert_eq!(parse_safety_result("unknown status"), SafetyResult::Unknown);
    }

    #[test]
    fn test_parse_memory_results() {
        let text = "- fact one\n- fact two\n---\n- fact three\n\nNo memories found";
        let results = parse_memory_results(text);
        assert_eq!(results, vec!["fact one", "fact two", "fact three"]);
    }

    #[test]
    fn test_local_risk_assessment() {
        assert_eq!(local_risk_assessment("rm -rf /tmp/test"), RiskLevel::Critical);
        assert_eq!(local_risk_assessment("rm file.txt"), RiskLevel::High);
        assert_eq!(local_risk_assessment("git push origin main"), RiskLevel::Medium);
        assert_eq!(local_risk_assessment("cargo build"), RiskLevel::Low);
    }

    #[test]
    fn test_local_safety_check() {
        assert!(matches!(local_safety_check("normal input"), SafetyResult::Safe));
        assert!(matches!(local_safety_check("eval(bad_code)"), SafetyResult::Blocked(_)));
        assert!(matches!(local_safety_check("<script>alert(1)</script>"), SafetyResult::Blocked(_)));
    }

    #[test]
    fn test_extract_file_path_nonexistent() {
        // Non-existent paths should return None
        assert!(extract_file_path_from_result("no paths here").is_none());
    }

    #[test]
    fn test_parse_memory_results_empty() {
        assert!(parse_memory_results("No memories found").is_empty());
        assert!(parse_memory_results("").is_empty());
        assert!(parse_memory_results("---").is_empty());
    }
}
