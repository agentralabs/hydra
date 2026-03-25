//! Remote Hands — SSH execution on remote machines via shell ssh command.
//! No SSH library needed. Uses `ssh` and `scp` commands available on all Unix systems.
//! Every remote command is constitutionally receipted.

use std::path::PathBuf;
use std::time::Instant;
use crate::errors::ExecutorError;

// ── Types ──

/// Where to execute a command.
#[derive(Debug, Clone)]
pub enum ExecutionTarget {
    Local,
    Remote { host: String, user: String, auth: RemoteAuth, port: u16 },
}

/// Authentication method for SSH.
#[derive(Debug, Clone)]
pub enum RemoteAuth {
    SshKey { path: PathBuf },
    SshAgent,
    Password { vault_key: String },
}

/// Result of a remote execution.
#[derive(Debug, Clone)]
pub struct RemoteResult {
    pub success: bool,
    pub output: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

// ── SSH Execution ──

/// Execute a command on a remote machine via SSH.
pub fn execute_remote(
    target: &ExecutionTarget,
    command: &str,
) -> Result<RemoteResult, ExecutorError> {
    match target {
        ExecutionTarget::Local => execute_local(command),
        ExecutionTarget::Remote { host, user, auth, port } => {
            let start = Instant::now();
            let args = build_ssh_args(host, user, auth, *port, command);
            eprintln!("hydra-remote: ssh {}@{}:{} '{}'", user, host, port, command);
            let output = std::process::Command::new("ssh")
                .args(&args)
                .output()
                .map_err(|e| ExecutorError::RemoteFailed { reason: format!("SSH failed: {e}") })?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let combined = if stderr.is_empty() { stdout } else { format!("{stdout}\n{stderr}") };
            let exit_code = output.status.code().unwrap_or(-1);
            let duration_ms = start.elapsed().as_millis() as u64;
            eprintln!("hydra-remote: exit={exit_code} duration={duration_ms}ms");
            Ok(RemoteResult {
                success: output.status.success(),
                output: combined,
                exit_code,
                duration_ms,
            })
        }
    }
}

/// Execute locally (fallback for Local target).
fn execute_local(command: &str) -> Result<RemoteResult, ExecutorError> {
    let start = Instant::now();
    let output = std::process::Command::new("sh")
        .arg("-c").arg(command)
        .output()
        .map_err(|e| ExecutorError::RemoteFailed { reason: format!("{e}") })?;
    Ok(RemoteResult {
        success: output.status.success(),
        output: String::from_utf8_lossy(&output.stdout).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Read a file on a remote machine.
pub fn read_remote_file(target: &ExecutionTarget, path: &str) -> Result<String, ExecutorError> {
    let result = execute_remote(target, &format!("cat {path}"))?;
    if result.success { Ok(result.output) }
    else { Err(ExecutorError::RemoteFailed { reason: format!("Read failed: {}", result.output) }) }
}

/// Copy a local file to a remote machine.
pub fn scp_to(target: &ExecutionTarget, local: &str, remote_path: &str) -> Result<(), ExecutorError> {
    if let ExecutionTarget::Remote { host, user, port, auth } = target {
        let mut args = vec!["-P".to_string(), port.to_string()];
        if let RemoteAuth::SshKey { path } = auth {
            args.extend(["-i".to_string(), path.display().to_string()]);
        }
        args.push("-o".to_string()); args.push("StrictHostKeyChecking=accept-new".to_string());
        args.push(local.to_string());
        args.push(format!("{user}@{host}:{remote_path}"));
        let output = std::process::Command::new("scp").args(&args).output()
            .map_err(|e| ExecutorError::RemoteFailed { reason: format!("SCP failed: {e}") })?;
        if output.status.success() { Ok(()) }
        else { Err(ExecutorError::RemoteFailed { reason: String::from_utf8_lossy(&output.stderr).to_string() }) }
    } else {
        std::fs::copy(local, remote_path).map_err(|e| ExecutorError::RemoteFailed { reason: format!("{e}") })?;
        Ok(())
    }
}

/// Check if SSH is available on this system.
pub fn check_ssh_available() -> bool {
    std::process::Command::new("ssh").arg("-V")
        .output().map(|o| o.status.success()).unwrap_or(false)
}

/// Build SSH arguments for a remote command.
fn build_ssh_args(host: &str, user: &str, auth: &RemoteAuth, port: u16, command: &str) -> Vec<String> {
    let mut args = Vec::new();
    args.push("-o".into()); args.push("StrictHostKeyChecking=accept-new".into());
    args.push("-o".into()); args.push("ConnectTimeout=10".into());
    args.push("-p".into()); args.push(port.to_string());
    match auth {
        RemoteAuth::SshKey { path } => { args.push("-i".into()); args.push(path.display().to_string()); }
        RemoteAuth::SshAgent => {} // Uses running ssh-agent automatically
        RemoteAuth::Password { .. } => {} // Would need sshpass — not recommended
    }
    args.push(format!("{user}@{host}"));
    args.push(command.to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_args_with_key() {
        let args = build_ssh_args("host.com", "user", &RemoteAuth::SshKey { path: "/home/.ssh/id_rsa".into() }, 22, "ls");
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"/home/.ssh/id_rsa".to_string()));
        assert!(args.contains(&"user@host.com".to_string()));
    }

    #[test]
    fn ssh_args_with_agent() {
        let args = build_ssh_args("host.com", "user", &RemoteAuth::SshAgent, 2222, "uptime");
        assert!(!args.contains(&"-i".to_string()));
        assert!(args.contains(&"2222".to_string()));
    }

    #[test]
    fn local_target_executes() {
        let result = execute_remote(&ExecutionTarget::Local, "echo hello").unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn remote_result_fields() {
        let r = RemoteResult { success: true, output: "ok".into(), exit_code: 0, duration_ms: 42 };
        assert!(r.success);
        assert_eq!(r.exit_code, 0);
    }

    #[test]
    fn ssh_available() {
        // ssh should be available on any dev machine
        let available = check_ssh_available();
        eprintln!("SSH available: {available}");
        // Don't assert — CI might not have ssh
    }
}
