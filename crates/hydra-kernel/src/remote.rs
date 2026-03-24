//! O18 Remote Presence — WebSocket server, PIN auth, client management.
//! Lightweight web interface served on LAN for phone/tablet access.
//! All clients share the same Hydra instance, memory, and genome.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Default remote access port (separate from API port 3141).
pub const REMOTE_PORT: u16 = 7476;
/// Max simultaneous connected clients (EC-17.7).
const MAX_CLIENTS: usize = 3;
/// Max failed PIN attempts before lockout (EC-17.8).
const MAX_PIN_ATTEMPTS: u32 = 3;
/// Lockout duration in seconds after max failed attempts (EC-17.8).
const LOCKOUT_SECS: u64 = 300;

// ── Message Types ──

/// Messages from the web client to Hydra.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Auth { pin: String },
    Chat { text: String },
    Command { cmd: String },
    Ping,
}

/// Messages from Hydra to the web client.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    AuthResult { success: bool, reason: Option<String> },
    Chat { text: String, timestamp: String },
    Event { content: String, priority: String, timestamp: String },
    Pong,
    Error { message: String },
}

impl ServerMessage {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"type":"error","message":"serialize failed"}"#.into())
    }
}

// ── Client Tracking ──

/// A connected remote client (phone/tablet/laptop).
#[derive(Debug, Clone)]
pub struct RemoteClient {
    pub id: String,
    pub ip: String,
    pub connected_at: DateTime<Utc>,
    pub authenticated: bool,
}

// ── Remote Server ──

/// Remote presence server — manages clients, PIN auth, broadcasts.
pub struct RemoteServer {
    clients: Arc<Mutex<Vec<RemoteClient>>>,
    pin: String,
    max_clients: usize,
    /// Per-IP failed attempt tracking: ip → (count, last_attempt_time).
    failed_attempts: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    port: u16,
}

impl RemoteServer {
    pub fn new(port: u16) -> Self {
        let pin = generate_pin();
        eprintln!("hydra-remote: server on port {port}, PIN: {pin}");
        Self {
            clients: Arc::new(Mutex::new(Vec::new())),
            pin,
            max_clients: MAX_CLIENTS,
            failed_attempts: Arc::new(Mutex::new(HashMap::new())),
            port,
        }
    }

    pub fn pin(&self) -> &str { &self.pin }
    pub fn port(&self) -> u16 { self.port }

    /// Verify PIN with rate limiting (EC-17.8).
    pub fn verify_pin(&self, pin: &str, ip: &str) -> Result<(), String> {
        let mut attempts = self.failed_attempts.lock().unwrap();
        // Check lockout
        if let Some((count, last)) = attempts.get(ip) {
            if *count >= MAX_PIN_ATTEMPTS {
                let elapsed = last.elapsed().as_secs();
                if elapsed < LOCKOUT_SECS {
                    let remaining = LOCKOUT_SECS - elapsed;
                    return Err(format!("Locked out. Try again in {remaining}s"));
                }
                // Lockout expired — reset
                attempts.remove(ip);
            }
        }
        if pin == self.pin {
            attempts.remove(ip); // Reset on success
            Ok(())
        } else {
            let entry = attempts.entry(ip.to_string()).or_insert((0, Instant::now()));
            entry.0 += 1;
            entry.1 = Instant::now();
            let remaining = MAX_PIN_ATTEMPTS.saturating_sub(entry.0);
            Err(format!("Wrong PIN. {remaining} attempts remaining"))
        }
    }

    /// Add a client. Returns false if at capacity (EC-17.7).
    pub fn add_client(&self, client: RemoteClient) -> bool {
        let mut clients = self.clients.lock().unwrap();
        if clients.len() >= self.max_clients {
            eprintln!("hydra-remote: client rejected — max {}", self.max_clients);
            return false;
        }
        eprintln!("hydra-remote: client connected: {} from {}", client.id, client.ip);
        clients.push(client);
        true
    }

    /// Remove a client by ID.
    pub fn remove_client(&self, id: &str) {
        let mut clients = self.clients.lock().unwrap();
        let before = clients.len();
        clients.retain(|c| c.id != id);
        if clients.len() < before {
            eprintln!("hydra-remote: client disconnected: {id}");
        }
    }

    /// Number of connected clients.
    pub fn client_count(&self) -> usize {
        self.clients.lock().unwrap().len()
    }

    /// Process a client message and return a response.
    pub fn handle_message(&self, msg: &ClientMessage, client_ip: &str) -> ServerMessage {
        match msg {
            ClientMessage::Auth { pin } => {
                match self.verify_pin(pin, client_ip) {
                    Ok(()) => ServerMessage::AuthResult { success: true, reason: None },
                    Err(reason) => ServerMessage::AuthResult { success: false, reason: Some(reason) },
                }
            }
            ClientMessage::Chat { text } => {
                if text.trim().is_empty() {
                    return ServerMessage::Error { message: "Empty message".into() };
                }
                // Route through API cycle — placeholder for now, wired to cognitive loop
                let response = format!("[Hydra] Received: {}", text.trim());
                ServerMessage::Chat {
                    text: response,
                    timestamp: Utc::now().format("%H:%M").to_string(),
                }
            }
            ClientMessage::Command { cmd } => {
                ServerMessage::Chat {
                    text: format!("[Command] /{cmd}"),
                    timestamp: Utc::now().format("%H:%M").to_string(),
                }
            }
            ClientMessage::Ping => ServerMessage::Pong,
        }
    }

    /// Get the URL for remote access.
    pub fn url(&self) -> String {
        // Try to get LAN IP
        let ip = local_ip().unwrap_or_else(|| "127.0.0.1".into());
        format!("http://{}:{}", ip, self.port)
    }
}

/// Generate a random 4-digit PIN.
pub fn generate_pin() -> String {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{:04}", seed % 10000)
}

/// Get local LAN IP address.
fn local_ip() -> Option<String> {
    let output = std::process::Command::new("hostname")
        .arg("-I")
        .output()
        .or_else(|_| {
            // macOS fallback
            std::process::Command::new("ipconfig")
                .arg("getifaddr")
                .arg("en0")
                .output()
        })
        .ok()?;
    let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let first = ip.split_whitespace().next()?.to_string();
    if first.is_empty() { None } else { Some(first) }
}

/// The embedded HTML for the remote interface.
pub fn remote_page_html() -> &'static str {
    include_str!("../static/remote.html")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_generation_is_4_digits() {
        let pin = generate_pin();
        assert_eq!(pin.len(), 4);
        assert!(pin.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn verify_pin_correct() {
        let server = RemoteServer::new(7476);
        let pin = server.pin().to_string();
        assert!(server.verify_pin(&pin, "127.0.0.1").is_ok());
    }

    #[test]
    fn verify_pin_wrong_locks_after_3() {
        let server = RemoteServer::new(7476);
        let ip = "192.168.1.100";
        // Use a wrong PIN guaranteed different from the generated one
        let wrong = if server.pin() == "9999" { "1111" } else { "9999" };
        assert!(server.verify_pin(wrong, ip).is_err());
        assert!(server.verify_pin(wrong, ip).is_err());
        assert!(server.verify_pin(wrong, ip).is_err());
        // 4th attempt should be locked out
        let result = server.verify_pin(wrong, ip);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Locked out"));
    }

    #[test]
    fn max_clients_enforced() {
        let server = RemoteServer::new(7476);
        for i in 0..3 {
            assert!(server.add_client(RemoteClient {
                id: format!("c{i}"), ip: "127.0.0.1".into(),
                connected_at: Utc::now(), authenticated: true,
            }));
        }
        // 4th should fail
        assert!(!server.add_client(RemoteClient {
            id: "c3".into(), ip: "127.0.0.1".into(),
            connected_at: Utc::now(), authenticated: true,
        }));
        assert_eq!(server.client_count(), 3);
    }

    #[test]
    fn client_message_deserializes() {
        let json = r#"{"type":"chat","text":"hello"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Chat { .. }));

        let auth = r#"{"type":"auth","pin":"1234"}"#;
        let msg: ClientMessage = serde_json::from_str(auth).unwrap();
        assert!(matches!(msg, ClientMessage::Auth { .. }));
    }

    #[test]
    fn server_message_serializes() {
        let msg = ServerMessage::Chat {
            text: "hello".into(),
            timestamp: "14:30".into(),
        };
        let json = msg.to_json();
        assert!(json.contains("hello"));
        assert!(json.contains("chat"));
    }

    #[test]
    fn handle_ping_returns_pong() {
        let server = RemoteServer::new(7476);
        let resp = server.handle_message(&ClientMessage::Ping, "127.0.0.1");
        assert!(matches!(resp, ServerMessage::Pong));
    }

    #[test]
    fn remove_client_works() {
        let server = RemoteServer::new(7476);
        server.add_client(RemoteClient {
            id: "test".into(), ip: "127.0.0.1".into(),
            connected_at: Utc::now(), authenticated: true,
        });
        assert_eq!(server.client_count(), 1);
        server.remove_client("test");
        assert_eq!(server.client_count(), 0);
    }
}
