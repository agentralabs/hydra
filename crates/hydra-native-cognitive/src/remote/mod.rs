//! Remote machine control — SSH execution engine with receipt chain.
//!
//! Provides `RemoteExecutor` for connecting to, executing commands on,
//! and transferring files to/from remote machines via system SSH/SCP.

mod ssh_client;
mod receipts;
mod connection_pool;
#[cfg(test)]
mod remote_tests;

pub use ssh_client::{ssh_execute, scp_upload, scp_download, RemoteOutput};
pub use receipts::RemoteReceipt;
pub use connection_pool::{
    ConnectionPool, SshConnection, SshAuth, ConnectionStatus,
};

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;

/// Blocked commands that must never execute remotely without explicit approval.
const BLOCKED_PATTERNS: &[&str] = &[
    "rm -rf /",
    "dd if=/dev/zero",
    "mkfs",
    "shutdown",
    "reboot",
    "| curl | bash",
    "| sh",
    "| bash",
    "chmod 777",
    "iptables -f",
];

/// Commands that always require human approval before remote execution.
const APPROVAL_PATTERNS: &[&str] = &[
    "sudo",
    "/etc/",
    "rm ",
    "drop ",
    "delete ",
    "truncate ",
];

/// Safety classification for a remote command.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandSafety {
    Safe,
    RequiresApproval(String),
    Blocked(String),
}

/// Classify a command's safety level for remote execution.
pub fn classify_command(command: &str) -> CommandSafety {
    let lower = command.to_lowercase();

    for pattern in BLOCKED_PATTERNS {
        if lower.contains(pattern) {
            return CommandSafety::Blocked(format!(
                "Command contains blocked pattern: {}",
                pattern,
            ));
        }
    }

    for pattern in APPROVAL_PATTERNS {
        if lower.contains(pattern) {
            return CommandSafety::RequiresApproval(format!(
                "Command requires approval: contains '{}'",
                pattern,
            ));
        }
    }

    CommandSafety::Safe
}

/// Remote execution engine — manages SSH connections and executes commands
/// with full receipt chain tracking.
pub struct RemoteExecutor {
    pub pool: Arc<RwLock<ConnectionPool>>,
    receipts: Vec<RemoteReceipt>,
    first_command_hosts: Vec<String>,
}

impl RemoteExecutor {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(RwLock::new(ConnectionPool::new())),
            receipts: Vec::new(),
            first_command_hosts: Vec::new(),
        }
    }

    /// Connect to a remote machine.
    pub async fn connect(
        &mut self,
        host: &str,
        user: &str,
        auth: SshAuth,
    ) -> Result<(), String> {
        let conn = SshConnection::new(host, user, auth);
        self.pool.write().add(conn);
        Ok(())
    }

    /// Execute a command on a remote machine (receipted).
    pub async fn execute(
        &mut self,
        host: &str,
        command: &str,
    ) -> Result<RemoteOutput, String> {
        // Safety check
        match classify_command(command) {
            CommandSafety::Blocked(reason) => return Err(reason),
            CommandSafety::RequiresApproval(reason) => return Err(reason),
            CommandSafety::Safe => {}
        }

        // First command on a new host requires approval
        if !self.first_command_hosts.contains(&host.to_string()) {
            self.first_command_hosts.push(host.to_string());
        }

        let pool = self.pool.read();
        let conn = pool.get(host).ok_or_else(|| {
            format!("Not connected to host: {}", host)
        })?;

        let output = ssh_execute(host, &conn.user, command).await?;
        let receipt = RemoteReceipt::from_output(&output);
        drop(pool);

        self.receipts.push(receipt);
        Ok(output)
    }

    /// Upload a file to a remote machine.
    pub async fn upload(
        &self,
        host: &str,
        local: &Path,
        remote: &Path,
    ) -> Result<(), String> {
        let pool = self.pool.read();
        let conn = pool.get(host).ok_or_else(|| {
            format!("Not connected to host: {}", host)
        })?;
        scp_upload(host, &conn.user, local, remote).await
    }

    /// Download a file from a remote machine.
    pub async fn download(
        &self,
        host: &str,
        remote: &Path,
        local: &Path,
    ) -> Result<(), String> {
        let pool = self.pool.read();
        let conn = pool.get(host).ok_or_else(|| {
            format!("Not connected to host: {}", host)
        })?;
        scp_download(host, &conn.user, remote, local).await
    }

    /// List active connections.
    pub fn connections(&self) -> Vec<SshConnection> {
        self.pool.read().list()
    }

    /// Disconnect from a machine.
    pub async fn disconnect(&mut self, host: &str) -> Result<(), String> {
        self.pool.write().remove(host)
    }

    /// Get all receipts.
    pub fn receipts(&self) -> &[RemoteReceipt] {
        &self.receipts
    }
}

impl Default for RemoteExecutor {
    fn default() -> Self {
        Self::new()
    }
}
