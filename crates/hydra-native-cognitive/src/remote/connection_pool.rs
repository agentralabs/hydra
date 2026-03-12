//! Connection pool — manages multiple SSH connections to remote machines.

use std::collections::HashMap;
use std::path::PathBuf;

/// SSH authentication method.
#[derive(Debug, Clone)]
pub enum SshAuth {
    /// Authenticate using a key file (e.g. ~/.ssh/id_rsa).
    KeyFile(PathBuf),
    /// Use the SSH agent for authentication.
    Agent,
}

/// Status of an SSH connection.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Error(String),
}

/// A single SSH connection to a remote machine.
#[derive(Debug, Clone)]
pub struct SshConnection {
    pub host: String,
    pub user: String,
    pub auth: SshAuth,
    pub status: ConnectionStatus,
}

impl SshConnection {
    pub fn new(host: &str, user: &str, auth: SshAuth) -> Self {
        Self {
            host: host.to_string(),
            user: user.to_string(),
            auth,
            status: ConnectionStatus::Connected,
        }
    }

    /// Format as user@host for display.
    pub fn display_addr(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }
}

/// Pool of SSH connections keyed by hostname.
pub struct ConnectionPool {
    connections: HashMap<String, SshConnection>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    /// Add or update a connection.
    pub fn add(&mut self, conn: SshConnection) {
        self.connections.insert(conn.host.clone(), conn);
    }

    /// Get a connection by hostname.
    pub fn get(&self, host: &str) -> Option<&SshConnection> {
        self.connections.get(host)
    }

    /// Remove a connection by hostname.
    pub fn remove(&mut self, host: &str) -> Result<(), String> {
        self.connections
            .remove(host)
            .map(|_| ())
            .ok_or_else(|| format!("No connection to host: {}", host))
    }

    /// List all connections (cloned).
    pub fn list(&self) -> Vec<SshConnection> {
        self.connections.values().cloned().collect()
    }

    /// Number of active connections.
    pub fn count(&self) -> usize {
        self.connections.len()
    }

    /// Check if a host is connected.
    pub fn is_connected(&self, host: &str) -> bool {
        self.connections.contains_key(host)
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}
