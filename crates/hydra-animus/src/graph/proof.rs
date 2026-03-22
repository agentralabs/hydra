//! Proof structures embedded in Prime graphs.
//! Proofs are claims with verifiable evidence.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a proof.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProofId(String);

impl ProofId {
    /// Generate a new unique proof ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    /// Returns the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ProofId {
    fn default() -> Self {
        Self::new()
    }
}

/// The status of a proof — has it been verified?
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProofStatus {
    /// Not yet verified.
    Pending,
    /// Verified and valid.
    Verified,
    /// Verification failed.
    Refuted { reason: String },
    /// Cannot be verified with available information.
    Undecidable,
}

/// A claim with supporting evidence, embedded in a Prime graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    /// Unique identifier.
    pub id: ProofId,
    /// The claim being proved (human-readable statement).
    pub claim: String,
    /// Evidence supporting the claim (serialized as JSON).
    pub evidence: Vec<serde_json::Value>,
    /// Current verification status.
    pub status: ProofStatus,
    /// Confidence in this proof (0.0-1.0).
    pub confidence: f64,
}

impl Proof {
    /// Create a new pending proof.
    pub fn new(claim: impl Into<String>) -> Self {
        Self {
            id: ProofId::new(),
            claim: claim.into(),
            evidence: Vec::new(),
            status: ProofStatus::Pending,
            confidence: 0.5,
        }
    }

    /// Add a piece of evidence to this proof.
    pub fn with_evidence(mut self, evidence: serde_json::Value) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Mark this proof as verified.
    pub fn verify(mut self, confidence: f64) -> Self {
        self.status = ProofStatus::Verified;
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Mark this proof as refuted.
    pub fn refute(mut self, reason: impl Into<String>) -> Self {
        self.status = ProofStatus::Refuted {
            reason: reason.into(),
        };
        self.confidence = 0.0;
        self
    }

    /// Returns true if verified.
    pub fn is_verified(&self) -> bool {
        matches!(self.status, ProofStatus::Verified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_starts_pending() {
        let p = Proof::new("the sky is blue");
        assert_eq!(p.status, ProofStatus::Pending);
        assert!(!p.is_verified());
    }

    #[test]
    fn proof_can_be_verified() {
        let p = Proof::new("claim").verify(0.95);
        assert!(p.is_verified());
        assert_eq!(p.confidence, 0.95);
    }

    #[test]
    fn proof_can_be_refuted() {
        let p = Proof::new("claim").refute("contradicting evidence");
        assert!(matches!(p.status, ProofStatus::Refuted { .. }));
        assert_eq!(p.confidence, 0.0);
    }
}
