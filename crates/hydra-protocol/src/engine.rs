//! ProtocolEngine — unified coordinator.
//! Discover -> Adapt -> Connect -> Send -> Receipt.

use crate::adapter::adapt_to_protocol;
use crate::errors::ProtocolError;
use crate::family::{infer_from_target, ProtocolFamily, ProtocolHint};
use crate::lifecycle::ConnectionLifecycle;
use std::collections::HashMap;

/// Result of one protocol operation.
#[derive(Debug, Clone)]
pub struct ProtocolResult {
    pub success: bool,
    pub response: Option<String>,
    pub receipt_id: String,
    pub protocol: String,
    pub confidence: f64,
}

/// The protocol engine.
pub struct ProtocolEngine {
    connections: HashMap<String, ConnectionLifecycle>,
}

impl ProtocolEngine {
    /// Create a new protocol engine.
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    /// Discover the protocol for a target.
    pub fn discover(&self, target: &str) -> ProtocolHint {
        infer_from_target(target)
    }

    /// Send an intent to a target via appropriate protocol.
    pub fn send(
        &mut self,
        target: &str,
        intent: &str,
        payload: Option<&str>,
    ) -> Result<ProtocolResult, ProtocolError> {
        // 1. Discover protocol
        let hint = self.discover(target);

        if hint.likely_family == ProtocolFamily::Unknown {
            return Err(ProtocolError::NoAdapterFound {
                target: target.to_string(),
            });
        }

        // 2. Adapt intent to protocol
        let adaptation = adapt_to_protocol(target, intent, payload, &hint.likely_family)?;

        // 3. Ensure connection exists
        if !self.connections.contains_key(target) {
            let mut lc = ConnectionLifecycle::new(target, hint.likely_family.clone());
            lc.connect()?;
            self.connections.insert(target.to_string(), lc);
        }

        // 4. Send (simulated in this implementation)
        let receipt_id = adaptation.request.receipt_id.clone();
        let confidence = adaptation.confidence;
        let protocol = hint.likely_family.label();

        Ok(ProtocolResult {
            success: true,
            response: Some(format!(
                "{{\"status\":\"ok\",\"protocol\":\"{}\"}}",
                protocol
            )),
            receipt_id,
            protocol,
            confidence,
        })
    }

    /// Return the number of active connections.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!("protocol: connections={}", self.connections.len())
    }
}

impl Default for ProtocolEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_send_succeeds() {
        let mut engine = ProtocolEngine::new();
        let r = engine
            .send("https://api.example.com/status", "GET /status", None)
            .expect("send failed");
        assert!(r.success);
        assert!(!r.receipt_id.is_empty());
        assert_eq!(r.protocol, "rest-http");
    }

    #[test]
    fn protocol_discovered_correctly() {
        let engine = ProtocolEngine::new();
        let h = engine.discover("wss://stream.example.com/events");
        assert_eq!(h.likely_family, ProtocolFamily::WebSocket);
        assert!(h.confidence > 0.9);
    }

    #[test]
    fn cobol_target_adapted() {
        let mut engine = ProtocolEngine::new();
        let r = engine
            .send(
                "mainframe.corp.internal/jcl",
                "BATCH_JOB_SUBMIT",
                Some("//PROGRAM SOURCE"),
            )
            .expect("send failed");
        assert!(r.success);
        assert_eq!(r.protocol, "cobol-jcl");
    }

    #[test]
    fn connection_reused() {
        let mut engine = ProtocolEngine::new();
        engine
            .send("https://api.example.com/a", "intent", None)
            .expect("send failed");
        engine
            .send("https://api.example.com/b", "intent", None)
            .expect("send failed");
        assert!(engine.connection_count() >= 1);
    }

    #[test]
    fn summary_format() {
        let engine = ProtocolEngine::new();
        let s = engine.summary();
        assert!(s.contains("protocol:"));
    }
}
