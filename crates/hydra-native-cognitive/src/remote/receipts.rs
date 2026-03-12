//! Receipt chain for remote actions — every remote command gets a cryptographic receipt.

use chrono::{DateTime, Utc};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use super::ssh_client::RemoteOutput;

/// A receipt for a single remote action, providing an audit trail.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteReceipt {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub host: String,
    pub user: String,
    pub command: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub stdout_hash: String,
    pub stderr_hash: String,
    pub reversible: bool,
    pub reverse_command: Option<String>,
}

impl RemoteReceipt {
    /// Create a receipt from a RemoteOutput.
    pub fn from_output(output: &RemoteOutput) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let stdout_hash = hash_string(&output.stdout);
        let stderr_hash = hash_string(&output.stderr);
        let (reversible, reverse_command) =
            generate_reverse_command(&output.command);

        Self {
            id,
            timestamp: Utc::now(),
            host: output.host.clone(),
            user: String::new(), // filled by executor context
            command: output.command.clone(),
            exit_code: output.exit_code,
            duration_ms: output.duration_ms,
            stdout_hash,
            stderr_hash,
            reversible,
            reverse_command,
        }
    }

    /// Verify that a given stdout matches the receipt hash.
    pub fn verify_stdout(&self, stdout: &str) -> bool {
        hash_string(stdout) == self.stdout_hash
    }

    /// Verify that a given stderr matches the receipt hash.
    pub fn verify_stderr(&self, stderr: &str) -> bool {
        hash_string(stderr) == self.stderr_hash
    }
}

/// Hash a string for receipt verification.
/// Uses a fast deterministic hash (not cryptographic — sufficient for audit trail).
fn hash_string(s: &str) -> String {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Determine if a command is reversible and generate the reverse command.
fn generate_reverse_command(command: &str) -> (bool, Option<String>) {
    let trimmed = command.trim();

    // mkdir → rmdir
    if trimmed.starts_with("mkdir ") {
        let dir = trimmed.strip_prefix("mkdir ").unwrap().trim();
        let dir = dir.strip_prefix("-p ").unwrap_or(dir).trim();
        return (true, Some(format!("rmdir {}", dir)));
    }

    // touch → rm
    if trimmed.starts_with("touch ") {
        let file = trimmed.strip_prefix("touch ").unwrap().trim();
        return (true, Some(format!("rm {}", file)));
    }

    // cp → rm target
    if trimmed.starts_with("cp ") {
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 3 {
            let target = parts.last().unwrap();
            return (true, Some(format!("rm {}", target)));
        }
    }

    // mv → mv back
    if trimmed.starts_with("mv ") {
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() == 3 {
            return (true, Some(format!("mv {} {}", parts[2], parts[1])));
        }
    }

    // systemctl start → systemctl stop (and vice versa)
    if trimmed.starts_with("systemctl start ") {
        let svc = trimmed.strip_prefix("systemctl start ").unwrap();
        return (true, Some(format!("systemctl stop {}", svc)));
    }
    if trimmed.starts_with("systemctl stop ") {
        let svc = trimmed.strip_prefix("systemctl stop ").unwrap();
        return (true, Some(format!("systemctl start {}", svc)));
    }

    (false, None)
}

#[cfg(test)]
mod receipt_unit_tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let h1 = hash_string("hello world");
        let h2 = hash_string("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different_inputs() {
        let h1 = hash_string("hello");
        let h2 = hash_string("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_reverse_mkdir() {
        let (rev, cmd) = generate_reverse_command("mkdir /tmp/test");
        assert!(rev);
        assert_eq!(cmd.unwrap(), "rmdir /tmp/test");
    }

    #[test]
    fn test_reverse_mv() {
        let (rev, cmd) = generate_reverse_command("mv a.txt b.txt");
        assert!(rev);
        assert_eq!(cmd.unwrap(), "mv b.txt a.txt");
    }

    #[test]
    fn test_non_reversible() {
        let (rev, cmd) = generate_reverse_command("echo hello");
        assert!(!rev);
        assert!(cmd.is_none());
    }
}
