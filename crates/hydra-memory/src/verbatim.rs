//! The VerbatimRecord system — write-ahead, SHA256 integrity, immutable.
//! This is the foundation of total recall.
//! Every exchange is stored verbatim before Hydra responds.

use crate::{
    constants::MAX_VERBATIM_SIZE_BYTES,
    errors::MemoryError,
    layers::{MemoryLayer, MemoryRecord},
};
use hydra_temporal::timestamp::Timestamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Which surface the interaction came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Surface {
    /// Terminal user interface.
    Tui,
    /// Voice interface.
    Voice,
    /// Remote API call.
    Remote,
    /// Programmatic API.
    Api,
}

/// A snapshot of Hydra's system state at the moment of an exchange.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextSnapshot {
    /// Number of active projects.
    pub active_project_count: u32,
    /// Number of fleet members.
    pub fleet_size: u32,
    /// Current trust temperature.
    pub trust_temperature: f64,
    /// Lyapunov exponent value.
    pub lyapunov_value: f64,
    /// Number of genomes in the system.
    pub genome_count: u64,
    /// Number of pending tasks.
    pub pending_task_count: u32,
    /// Signal backlog count.
    pub signal_backlog: u32,
}

/// The verbatim record of one complete exchange.
/// Stored before Hydra sends its response (write-ahead guarantee).
/// Never modified after creation (Constitutional Law 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerbatimRecord {
    /// Unique ID for this exchange.
    pub id: String,
    /// Which session this belongs to.
    pub session_id: String,
    /// Position within the session (0-indexed).
    pub sequence: u64,
    /// When the principal started speaking.
    pub timestamp_start: Timestamp,
    /// When Hydra finished responding.
    pub timestamp_end: Option<Timestamp>,
    /// Which surface this came from.
    pub surface: Surface,
    /// EXACT words from the principal. Never paraphrased.
    pub principal_input: String,
    /// EXACT words from Hydra. Never paraphrased.
    pub hydra_response: Option<String>,
    /// SHA256 hash of (principal_input + hydra_response).
    /// Computed when the record is finalized.
    pub content_hash: Option<String>,
    /// Hydra's belief manifold state before this exchange.
    pub manifold_curvature_before: f64,
    /// Hydra's belief manifold state after this exchange.
    pub manifold_curvature_after: Option<f64>,
    /// System context at the moment of this exchange.
    pub context: ContextSnapshot,
    /// Causal chain root (links to hydra-temporal).
    pub causal_root: String,
}

impl VerbatimRecord {
    /// Create a new verbatim record at exchange start.
    /// Called BEFORE Hydra generates a response — this is the write-ahead.
    pub fn begin(
        session_id: impl Into<String>,
        sequence: u64,
        surface: Surface,
        input: impl Into<String>,
        context: ContextSnapshot,
        causal_root: impl Into<String>,
    ) -> Result<Self, MemoryError> {
        let input_str = input.into();

        if input_str.len() > MAX_VERBATIM_SIZE_BYTES {
            return Err(MemoryError::WriteAheadFailed {
                exchange_id: Uuid::new_v4().to_string(),
                reason: format!(
                    "input size {} exceeds maximum {}",
                    input_str.len(),
                    MAX_VERBATIM_SIZE_BYTES
                ),
            });
        }

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            sequence,
            timestamp_start: Timestamp::now(),
            timestamp_end: None,
            surface,
            principal_input: input_str,
            hydra_response: None,
            content_hash: None,
            manifold_curvature_before: context.lyapunov_value,
            manifold_curvature_after: None,
            context,
            causal_root: causal_root.into(),
        })
    }

    /// Finalize the record with Hydra's response.
    /// Computes the SHA256 hash. Record is immutable after this.
    pub fn finalize(&mut self, response: impl Into<String>, manifold_curvature_after: f64) {
        let response_str = response.into();

        // Compute SHA256 hash of the complete exchange
        let hash = compute_hash(&self.principal_input, &response_str);

        self.hydra_response = Some(response_str);
        self.content_hash = Some(hash);
        self.timestamp_end = Some(Timestamp::now());
        self.manifold_curvature_after = Some(manifold_curvature_after);
    }

    /// Verify the integrity of a retrieved record.
    /// Returns Err if the hash does not match.
    pub fn verify_integrity(&self) -> Result<(), MemoryError> {
        let stored_hash = match &self.content_hash {
            Some(h) => h,
            None => return Ok(()), // not yet finalized — no hash to check
        };

        let response = self.hydra_response.as_deref().unwrap_or("");
        let computed = compute_hash(&self.principal_input, response);

        if &computed != stored_hash {
            return Err(MemoryError::IntegrityCheckFailed {
                id: self.id.clone(),
            });
        }

        Ok(())
    }

    /// Convert to a MemoryRecord for storage.
    pub fn to_memory_record(&self) -> MemoryRecord {
        let content = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
        let mut record = MemoryRecord::new(
            MemoryLayer::Verbatim,
            content,
            &self.session_id,
            &self.causal_root,
        );
        if let Some(hash) = &self.content_hash {
            record = record.with_hash(hash.clone());
        }
        record
    }
}

/// Compute SHA256 hash of (input + response).
fn compute_hash(input: &str, response: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher.update(b"||");
    hasher.update(response.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record() -> VerbatimRecord {
        VerbatimRecord::begin(
            "session-001",
            0,
            Surface::Tui,
            "build AgenticData v0.2.0",
            ContextSnapshot::default(),
            "const-identity",
        )
        .expect("should create record")
    }

    #[test]
    fn record_created_with_input() {
        let r = make_record();
        assert_eq!(r.principal_input, "build AgenticData v0.2.0");
        assert!(r.hydra_response.is_none());
        assert!(r.content_hash.is_none());
    }

    #[test]
    fn finalize_adds_hash() {
        let mut r = make_record();
        r.finalize("Task created and running", 0.1);
        assert!(r.content_hash.is_some());
        assert!(r.hydra_response.is_some());
    }

    #[test]
    fn integrity_check_passes_on_valid_record() {
        let mut r = make_record();
        r.finalize("response text", 0.1);
        assert!(r.verify_integrity().is_ok());
    }

    #[test]
    fn integrity_check_fails_on_tampered_response() {
        let mut r = make_record();
        r.finalize("original response", 0.1);
        // Tamper with the response
        r.hydra_response = Some("tampered response".to_string());
        assert!(matches!(
            r.verify_integrity(),
            Err(MemoryError::IntegrityCheckFailed { .. })
        ));
    }

    #[test]
    fn oversized_input_rejected() {
        let big_input = "x".repeat(MAX_VERBATIM_SIZE_BYTES + 1);
        let result = VerbatimRecord::begin(
            "s",
            0,
            Surface::Tui,
            big_input,
            ContextSnapshot::default(),
            "root",
        );
        assert!(result.is_err());
    }

    #[test]
    fn to_memory_record_is_verbatim_layer() {
        let r = make_record();
        let mr = r.to_memory_record();
        assert_eq!(mr.layer, MemoryLayer::Verbatim);
    }
}
