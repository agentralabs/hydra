//! SSH client — command execution and file transfer via system ssh/scp binaries.

use std::path::Path;
use std::time::Instant;

/// Output from a remote command execution.
#[derive(Debug, Clone)]
pub struct RemoteOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub host: String,
    pub command: String,
    pub duration_ms: u64,
}

/// Build SSH command arguments for remote execution.
pub fn build_ssh_args(host: &str, user: &str, command: &str) -> Vec<String> {
    vec![
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        format!("{}@{}", user, host),
        command.to_string(),
    ]
}

/// Build SCP upload arguments.
pub fn build_scp_upload_args(
    host: &str,
    user: &str,
    local: &Path,
    remote: &Path,
) -> Vec<String> {
    vec![
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        local.to_string_lossy().to_string(),
        format!("{}@{}:{}", user, host, remote.display()),
    ]
}

/// Build SCP download arguments.
pub fn build_scp_download_args(
    host: &str,
    user: &str,
    remote: &Path,
    local: &Path,
) -> Vec<String> {
    vec![
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        format!("{}@{}:{}", user, host, remote.display()),
        local.to_string_lossy().to_string(),
    ]
}

/// Execute a command on a remote machine via system SSH.
pub async fn ssh_execute(
    host: &str,
    user: &str,
    command: &str,
) -> Result<RemoteOutput, String> {
    let args = build_ssh_args(host, user, command);
    let start = Instant::now();

    let output = tokio::process::Command::new("ssh")
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("SSH execution failed: {}", e))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(RemoteOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        host: host.to_string(),
        command: command.to_string(),
        duration_ms,
    })
}

/// Upload a file to a remote machine via SCP.
pub async fn scp_upload(
    host: &str,
    user: &str,
    local: &Path,
    remote: &Path,
) -> Result<(), String> {
    let args = build_scp_upload_args(host, user, local, remote);

    let status = tokio::process::Command::new("scp")
        .args(&args)
        .status()
        .await
        .map_err(|e| format!("SCP upload failed: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "SCP upload to {}:{} failed with exit code {}",
            host,
            remote.display(),
            status.code().unwrap_or(-1),
        ))
    }
}

/// Download a file from a remote machine via SCP.
pub async fn scp_download(
    host: &str,
    user: &str,
    remote: &Path,
    local: &Path,
) -> Result<(), String> {
    let args = build_scp_download_args(host, user, remote, local);

    let status = tokio::process::Command::new("scp")
        .args(&args)
        .status()
        .await
        .map_err(|e| format!("SCP download failed: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "SCP download from {}:{} failed with exit code {}",
            host,
            remote.display(),
            status.code().unwrap_or(-1),
        ))
    }
}
