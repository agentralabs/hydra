//! Slash commands — SSH remote machine control (P8).
//! /ssh, /ssh-exec, /ssh-upload, /ssh-download, /ssh-disconnect, /ssh-list

use super::app::{App, Message, MessageRole, PendingApproval};

impl App {
    /// /ssh <user@host> — Connect to a remote machine.
    pub(crate) fn slash_cmd_ssh(&mut self, args: &str, timestamp: &str) {
        let args = args.trim();
        if args.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /ssh <user@host>\n\
                         \n\
                         Example:\n\
                           /ssh deploy@server1.example.com\n\
                           /ssh root@192.168.1.100"
                    .to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }

        let (user, host) = if args.contains('@') {
            let parts: Vec<&str> = args.splitn(2, '@').collect();
            (parts[0].to_string(), parts[1].to_string())
        } else {
            ("root".to_string(), args.to_string())
        };

        // Store connection request for async handling
        let auth = hydra_native::remote::SshAuth::Agent;
        self.remote_executor.pool.write().add(
            hydra_native::remote::SshConnection::new(&host, &user, auth),
        );

        self.messages.push(Message {
            role: MessageRole::System,
            content: format!(
                "Connected to {}@{} (SSH agent auth)\n\
                 Use /ssh-exec {} <command> to run commands.",
                user, host, host,
            ),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    /// /ssh-exec <host> <command> — Execute command on connected machine.
    pub(crate) fn slash_cmd_ssh_exec(&mut self, args: &str, timestamp: &str) {
        let args = args.trim();
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.len() < 2 || parts[1].is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /ssh-exec <host> <command>\n\
                         \n\
                         Example: /ssh-exec server1 cargo test"
                    .to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }

        let host = parts[0];
        let command = parts[1];

        // Safety check
        let safety = hydra_native::remote::classify_command(command);
        match safety {
            hydra_native::remote::CommandSafety::Blocked(reason) => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("BLOCKED: {}", reason),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
                return;
            }
            hydra_native::remote::CommandSafety::RequiresApproval(reason) => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!(
                        "{}. Approve? (y/n)\n\
                         Command: {} on {}",
                        reason, command, host,
                    ),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
                self.pending_approval = Some(PendingApproval {
                    approval_id: None,
                    risk_level: "HIGH".to_string(),
                    action: format!("ssh-exec:{}:{}", host, command),
                    description: format!("Execute '{}' on {}", command, host),
                });
                return;
            }
            hydra_native::remote::CommandSafety::Safe => {}
        }

        // Check connection
        if !self.remote_executor.pool.read().is_connected(host) {
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!(
                    "Not connected to '{}'. Use /ssh <user@host> first.",
                    host,
                ),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }

        let user = self.remote_executor.pool.read()
            .get(host).map(|c| c.user.clone())
            .unwrap_or_else(|| "root".to_string());

        // Spawn as background command via system SSH
        self.spawn_command(
            &format!("ssh {}@{}: {}", user, host, command),
            "ssh",
            &[
                "-o", "StrictHostKeyChecking=accept-new",
                "-o", "ConnectTimeout=10",
                "-o", "BatchMode=yes",
                &format!("{}@{}", user, host),
                command,
            ],
        );

        self.messages.push(Message {
            role: MessageRole::System,
            content: format!("Executing on {}: {}", host, command),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    /// /ssh-upload <host> <local> <remote> — Upload file.
    pub(crate) fn slash_cmd_ssh_upload(&mut self, args: &str, timestamp: &str) {
        let parts: Vec<&str> = args.trim().split_whitespace().collect();
        if parts.len() < 3 {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /ssh-upload <host> <local-path> <remote-path>"
                    .to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }

        let host = parts[0];
        let local = parts[1];
        let remote = parts[2];

        if !self.remote_executor.pool.read().is_connected(host) {
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Not connected to '{}'. Use /ssh first.", host),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }

        let user = self.remote_executor.pool.read()
            .get(host).map(|c| c.user.clone())
            .unwrap_or_else(|| "root".to_string());

        self.spawn_command(
            &format!("scp → {}:{}", host, remote),
            "scp",
            &[
                "-o", "StrictHostKeyChecking=accept-new",
                "-o", "ConnectTimeout=10",
                local,
                &format!("{}@{}:{}", user, host, remote),
            ],
        );

        self.messages.push(Message {
            role: MessageRole::System,
            content: format!("Uploading {} → {}:{}", local, host, remote),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    /// /ssh-download <host> <remote> <local> — Download file.
    pub(crate) fn slash_cmd_ssh_download(
        &mut self,
        args: &str,
        timestamp: &str,
    ) {
        let parts: Vec<&str> = args.trim().split_whitespace().collect();
        if parts.len() < 3 {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /ssh-download <host> <remote-path> <local-path>"
                    .to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }

        let host = parts[0];
        let remote = parts[1];
        let local = parts[2];

        if !self.remote_executor.pool.read().is_connected(host) {
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!("Not connected to '{}'. Use /ssh first.", host),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }

        let user = self.remote_executor.pool.read()
            .get(host).map(|c| c.user.clone())
            .unwrap_or_else(|| "root".to_string());

        self.spawn_command(
            &format!("scp ← {}:{}", host, remote),
            "scp",
            &[
                "-o", "StrictHostKeyChecking=accept-new",
                "-o", "ConnectTimeout=10",
                &format!("{}@{}:{}", user, host, remote),
                local,
            ],
        );

        self.messages.push(Message {
            role: MessageRole::System,
            content: format!("Downloading {}:{} → {}", host, remote, local),
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }

    /// /ssh-disconnect <host> — Disconnect.
    pub(crate) fn slash_cmd_ssh_disconnect(
        &mut self,
        args: &str,
        timestamp: &str,
    ) {
        let host = args.trim();
        if host.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "Usage: /ssh-disconnect <host>".to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }

        match self.remote_executor.pool.write().remove(host) {
            Ok(()) => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: format!("Disconnected from {}", host),
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
            Err(e) => {
                self.messages.push(Message {
                    role: MessageRole::System,
                    content: e,
                    timestamp: timestamp.to_string(),
                    phase: None,
                });
            }
        }
    }

    /// /ssh-list — List active connections.
    pub(crate) fn slash_cmd_ssh_list(&mut self, timestamp: &str) {
        let connections = self.remote_executor.connections();
        if connections.is_empty() {
            self.messages.push(Message {
                role: MessageRole::System,
                content: "SSH Connections\n\n  No active connections.\n\n\
                         Use /ssh <user@host> to connect."
                    .to_string(),
                timestamp: timestamp.to_string(),
                phase: None,
            });
            return;
        }

        let mut msg = format!(
            "SSH Connections ({} active)\n\n",
            connections.len(),
        );
        for conn in &connections {
            let status = match &conn.status {
                hydra_native::remote::ConnectionStatus::Connected => "connected",
                hydra_native::remote::ConnectionStatus::Disconnected => "disconnected",
                hydra_native::remote::ConnectionStatus::Error(e) => e.as_str(),
            };
            msg.push_str(&format!(
                "  {} — {}\n",
                conn.display_addr(),
                status,
            ));
        }
        msg.push_str("\nUse /ssh-exec <host> <cmd> to run commands.");

        self.messages.push(Message {
            role: MessageRole::System,
            content: msg,
            timestamp: timestamp.to_string(),
            phase: None,
        });
    }
}
