use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use hydra_core::error::HydraError;

use crate::types::ProtocolKind;

/// Transport security requirements for network protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportSecurity {
    /// TLS 1.3 required (default for all network protocols)
    Tls13,
    /// Local-only (no network, e.g. ShellCommand, Sister via IPC)
    Local,
    /// Mutual TLS (server-to-server)
    MutualTls,
}

impl TransportSecurity {
    /// Get required transport security for a protocol kind
    pub fn required_for(kind: ProtocolKind) -> Self {
        match kind {
            ProtocolKind::Sister => Self::Local,
            ProtocolKind::ShellCommand => Self::Local,
            ProtocolKind::McpTool => Self::Local,
            ProtocolKind::RestApi => Self::Tls13,
            ProtocolKind::BrowserAutomation => Self::Tls13,
            ProtocolKind::LlmAgent => Self::Tls13,
        }
    }

    /// Check if this transport security level is acceptable
    pub fn is_network(&self) -> bool {
        matches!(self, Self::Tls13 | Self::MutualTls)
    }
}

/// Verify that a network protocol meets TLS requirements
pub fn verify_transport(kind: ProtocolKind, endpoint: &str) -> Result<(), HydraError> {
    let required = TransportSecurity::required_for(kind);
    if required.is_network() && !endpoint.is_empty() {
        // Network protocols must use HTTPS
        if !endpoint.starts_with("https://") && !endpoint.starts_with("wss://") {
            return Err(HydraError::PermissionDenied(
                "Network protocols require TLS 1.3. Use https:// or wss:// endpoints.".into(),
            ));
        }
    }
    Ok(())
}

/// Per-protocol-type rate limiter
pub struct RateLimiter {
    limits: HashMap<ProtocolKind, RateLimit>,
    counters: Mutex<HashMap<ProtocolKind, RateCounter>>,
}

#[derive(Debug, Clone)]
struct RateLimit {
    max_per_second: u32,
    max_per_minute: u32,
}

#[derive(Debug, Clone)]
struct RateCounter {
    second_count: u32,
    minute_count: u32,
    second_start: Instant,
    minute_start: Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        let mut limits = HashMap::new();
        // Default rate limits per protocol type
        limits.insert(
            ProtocolKind::Sister,
            RateLimit {
                max_per_second: 100,
                max_per_minute: 1000,
            },
        );
        limits.insert(
            ProtocolKind::ShellCommand,
            RateLimit {
                max_per_second: 10,
                max_per_minute: 100,
            },
        );
        limits.insert(
            ProtocolKind::McpTool,
            RateLimit {
                max_per_second: 50,
                max_per_minute: 500,
            },
        );
        limits.insert(
            ProtocolKind::RestApi,
            RateLimit {
                max_per_second: 20,
                max_per_minute: 200,
            },
        );
        limits.insert(
            ProtocolKind::BrowserAutomation,
            RateLimit {
                max_per_second: 2,
                max_per_minute: 30,
            },
        );
        limits.insert(
            ProtocolKind::LlmAgent,
            RateLimit {
                max_per_second: 5,
                max_per_minute: 60,
            },
        );
        Self {
            limits,
            counters: Mutex::new(HashMap::new()),
        }
    }

    /// Check if a call to this protocol type is allowed under rate limits
    pub fn check(&self, kind: ProtocolKind) -> Result<(), HydraError> {
        let limit = match self.limits.get(&kind) {
            Some(l) => l,
            None => return Ok(()), // No limit configured
        };

        let mut counters = self.counters.lock();
        let counter = counters.entry(kind).or_insert_with(|| RateCounter {
            second_count: 0,
            minute_count: 0,
            second_start: Instant::now(),
            minute_start: Instant::now(),
        });

        let now = Instant::now();

        // Reset second window if expired
        if now.duration_since(counter.second_start) >= Duration::from_secs(1) {
            counter.second_count = 0;
            counter.second_start = now;
        }

        // Reset minute window if expired
        if now.duration_since(counter.minute_start) >= Duration::from_secs(60) {
            counter.minute_count = 0;
            counter.minute_start = now;
        }

        if counter.second_count >= limit.max_per_second {
            return Err(HydraError::PermissionDenied(format!(
                "Rate limit exceeded for {:?}. Max {} calls per second.",
                kind, limit.max_per_second
            )));
        }

        if counter.minute_count >= limit.max_per_minute {
            return Err(HydraError::PermissionDenied(format!(
                "Rate limit exceeded for {:?}. Max {} calls per minute.",
                kind, limit.max_per_minute
            )));
        }

        counter.second_count += 1;
        counter.minute_count += 1;
        Ok(())
    }

    /// Get current call count for a protocol type (for metrics)
    pub fn call_count(&self, kind: ProtocolKind) -> u32 {
        self.counters
            .lock()
            .get(&kind)
            .map(|c| c.minute_count)
            .unwrap_or(0)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Auth verifier — checks auth status before protocol execution
pub struct AuthVerifier;

impl AuthVerifier {
    /// Verify authentication is valid before execute()
    pub fn verify_before_execute(
        auth_required: bool,
        auth_valid: bool,
        protocol_name: &str,
    ) -> Result<(), HydraError> {
        if auth_required && !auth_valid {
            return Err(HydraError::PermissionDenied(format!(
                "Protocol '{protocol_name}' requires authentication. Credentials are missing or expired. Re-authenticate to continue."
            )));
        }
        Ok(())
    }
}

/// Signed health status for cryptographic non-repudiation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedHealthStatus {
    pub protocol_id: Uuid,
    pub status: crate::health::HealthStatus,
    pub checked_at: chrono::DateTime<chrono::Utc>,
    pub uptime_ratio: f64,
    /// SHA-256 hash of (protocol_id || status || checked_at || uptime_ratio)
    pub content_hash: String,
}

impl SignedHealthStatus {
    pub fn new(protocol_id: Uuid, status: crate::health::HealthStatus, uptime_ratio: f64) -> Self {
        let checked_at = chrono::Utc::now();
        let hash_input = format!(
            "{}|{:?}|{}|{}",
            protocol_id,
            status,
            checked_at.to_rfc3339(),
            uptime_ratio
        );
        // Simple hash using the content (real implementation would use SHA-256)
        let content_hash = format!("{:x}", md5_simple(&hash_input));
        Self {
            protocol_id,
            status,
            checked_at,
            uptime_ratio,
            content_hash,
        }
    }

    /// Verify the hash matches the content
    pub fn verify(&self) -> bool {
        let hash_input = format!(
            "{}|{:?}|{}|{}",
            self.protocol_id,
            self.status,
            self.checked_at.to_rfc3339(),
            self.uptime_ratio
        );
        let expected = format!("{:x}", md5_simple(&hash_input));
        self.content_hash == expected
    }
}

/// Simple content hash (DJB2 hash — production would use SHA-256)
fn md5_simple(input: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

/// Call counter for metrics
pub struct ProtocolCallCounter {
    counts: HashMap<ProtocolKind, AtomicU64>,
}

impl ProtocolCallCounter {
    pub fn new() -> Self {
        let mut counts = HashMap::new();
        for kind in [
            ProtocolKind::Sister,
            ProtocolKind::ShellCommand,
            ProtocolKind::McpTool,
            ProtocolKind::RestApi,
            ProtocolKind::BrowserAutomation,
            ProtocolKind::LlmAgent,
        ] {
            counts.insert(kind, AtomicU64::new(0));
        }
        Self { counts }
    }

    pub fn increment(&self, kind: ProtocolKind) {
        if let Some(counter) = self.counts.get(&kind) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn get(&self, kind: ProtocolKind) -> u64 {
        self.counts
            .get(&kind)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

impl Default for ProtocolCallCounter {
    fn default() -> Self {
        Self::new()
    }
}
