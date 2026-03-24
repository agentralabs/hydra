//! Transport layer — communication channels for MCP.
//! Supports stdio (for Claude Code integration) and in-memory (for testing).

use crate::errors::McpError;
use async_trait::async_trait;

/// A bidirectional transport for MCP messages.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a JSON string.
    async fn send(&self, message: &str) -> Result<(), McpError>;

    /// Receive a JSON string (blocks until available).
    async fn receive(&self) -> Result<String, McpError>;

    /// Transport name for logging.
    fn name(&self) -> &str;
}

/// In-memory transport for testing — uses channels.
pub struct MemoryTransport {
    name: String,
    tx: tokio::sync::mpsc::Sender<String>,
    rx: tokio::sync::Mutex<tokio::sync::mpsc::Receiver<String>>,
}

impl MemoryTransport {
    /// Create a pair of connected transports (client, server).
    pub fn pair() -> (Self, Self) {
        let (tx_a, rx_a) = tokio::sync::mpsc::channel(100);
        let (tx_b, rx_b) = tokio::sync::mpsc::channel(100);

        let client = Self {
            name: "client".into(),
            tx: tx_a,
            rx: tokio::sync::Mutex::new(rx_b),
        };
        let server = Self {
            name: "server".into(),
            tx: tx_b,
            rx: tokio::sync::Mutex::new(rx_a),
        };
        (client, server)
    }
}

#[async_trait]
impl Transport for MemoryTransport {
    async fn send(&self, message: &str) -> Result<(), McpError> {
        self.tx
            .send(message.to_string())
            .await
            .map_err(|e| McpError::TransportError(format!("Send failed: {e}")))
    }

    async fn receive(&self) -> Result<String, McpError> {
        let mut rx = self.rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| McpError::TransportError("Channel closed".into()))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Stdio transport for production MCP communication.
pub struct StdioTransport {
    _name: String,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            _name: "stdio".into(),
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&self, message: &str) -> Result<(), McpError> {
        use tokio::io::AsyncWriteExt;
        let mut stdout = tokio::io::stdout();
        stdout
            .write_all(message.as_bytes())
            .await
            .map_err(|e| McpError::TransportError(format!("stdout write: {e}")))?;
        stdout
            .write_all(b"\n")
            .await
            .map_err(|e| McpError::TransportError(format!("stdout newline: {e}")))?;
        stdout
            .flush()
            .await
            .map_err(|e| McpError::TransportError(format!("stdout flush: {e}")))?;
        Ok(())
    }

    async fn receive(&self) -> Result<String, McpError> {
        use tokio::io::AsyncBufReadExt;
        let stdin = tokio::io::stdin();
        let mut reader = tokio::io::BufReader::new(stdin);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| McpError::TransportError(format!("stdin read: {e}")))?;
        if line.is_empty() {
            return Err(McpError::TransportError("stdin EOF".into()));
        }
        Ok(line.trim().to_string())
    }

    fn name(&self) -> &str {
        "stdio"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_transport_roundtrip() {
        let (client, server) = MemoryTransport::pair();
        client.send("hello").await.unwrap();
        let received = server.receive().await.unwrap();
        assert_eq!(received, "hello");
    }

    #[tokio::test]
    async fn memory_transport_bidirectional() {
        let (client, server) = MemoryTransport::pair();

        client.send("request").await.unwrap();
        let req = server.receive().await.unwrap();
        assert_eq!(req, "request");

        server.send("response").await.unwrap();
        let resp = client.receive().await.unwrap();
        assert_eq!(resp, "response");
    }

    #[test]
    fn transport_names() {
        let (client, server) = MemoryTransport::pair();
        assert_eq!(client.name(), "client");
        assert_eq!(server.name(), "server");
    }
}
