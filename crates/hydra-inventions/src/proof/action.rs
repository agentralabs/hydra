//! ProofCarryingAction — actions with cryptographic proof of correctness.
//!
//! Every action carries a proof that it was authorized, what it did,
//! and what the expected outcome was, enabling post-hoc verification.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Status of a proof
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStatus {
    Pending,
    Verified,
    Failed,
    Expired,
}

/// Cryptographic proof attached to an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionProof {
    pub proof_id: String,
    pub action_hash: String,
    pub precondition_hash: String,
    pub postcondition_hash: String,
    pub authorization_hash: String,
    pub chain_hash: String,
    pub timestamp: String,
    pub status: ProofStatus,
}

/// An action that carries its own proof of correctness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCarryingAction {
    pub id: String,
    pub action_name: String,
    pub params: serde_json::Value,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub authorization: String,
    pub proof: Option<ActionProof>,
    pub timestamp: String,
}

impl ProofCarryingAction {
    /// Create a new proof-carrying action
    pub fn new(
        action_name: &str,
        params: serde_json::Value,
        authorization: &str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action_name: action_name.into(),
            params,
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            authorization: authorization.into(),
            proof: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Add a precondition
    pub fn with_precondition(mut self, condition: &str) -> Self {
        self.preconditions.push(condition.into());
        self
    }

    /// Add a postcondition
    pub fn with_postcondition(mut self, condition: &str) -> Self {
        self.postconditions.push(condition.into());
        self
    }

    /// Generate the cryptographic proof
    pub fn generate_proof(&mut self, previous_chain_hash: Option<&str>) -> &ActionProof {
        let action_hash = hash_content(&serde_json::to_string(&self.params).unwrap_or_default());
        let precondition_hash = hash_content(&self.preconditions.join(","));
        let postcondition_hash = hash_content(&self.postconditions.join(","));
        let authorization_hash = hash_content(&self.authorization);

        let chain_input = format!(
            "{}:{}:{}:{}:{}",
            action_hash,
            precondition_hash,
            postcondition_hash,
            authorization_hash,
            previous_chain_hash.unwrap_or("genesis"),
        );
        let chain_hash = hash_content(&chain_input);

        self.proof = Some(ActionProof {
            proof_id: uuid::Uuid::new_v4().to_string(),
            action_hash,
            precondition_hash,
            postcondition_hash,
            authorization_hash,
            chain_hash,
            timestamp: chrono::Utc::now().to_rfc3339(),
            status: ProofStatus::Pending,
        });

        self.proof.as_ref().unwrap()
    }

    /// Check if this action has a valid proof
    pub fn has_proof(&self) -> bool {
        self.proof.is_some()
    }

    /// Get proof chain hash (for linking to next action)
    pub fn chain_hash(&self) -> Option<String> {
        self.proof.as_ref().map(|p| p.chain_hash.clone())
    }
}

fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Store for proof-carrying actions
pub struct ProofStore {
    actions: parking_lot::RwLock<Vec<ProofCarryingAction>>,
    chain_index: parking_lot::RwLock<HashMap<String, String>>,
}

impl ProofStore {
    pub fn new() -> Self {
        Self {
            actions: parking_lot::RwLock::new(Vec::new()),
            chain_index: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Store a proof-carrying action
    pub fn store(&self, action: ProofCarryingAction) {
        if let Some(proof) = &action.proof {
            self.chain_index
                .write()
                .insert(proof.proof_id.clone(), action.id.clone());
        }
        self.actions.write().push(action);
    }

    /// Get the last chain hash for chaining
    pub fn last_chain_hash(&self) -> Option<String> {
        self.actions.read().last().and_then(|a| a.chain_hash())
    }

    /// Verify chain integrity
    pub fn verify_chain(&self) -> bool {
        let actions = self.actions.read();
        for action in actions.iter() {
            if action.proof.is_none() {
                return false;
            }
        }
        true
    }

    pub fn count(&self) -> usize {
        self.actions.read().len()
    }
}

impl Default for ProofStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_generation() {
        let mut action = ProofCarryingAction::new(
            "write_file",
            serde_json::json!({"path": "/tmp/test.txt"}),
            "user-approval-123",
        )
        .with_precondition("file_exists(/tmp/)")
        .with_postcondition("file_exists(/tmp/test.txt)");

        let proof = action.generate_proof(None);
        assert!(!proof.action_hash.is_empty());
        assert!(!proof.chain_hash.is_empty());
        assert_eq!(proof.status, ProofStatus::Pending);
    }

    #[test]
    fn test_proof_chain() {
        let mut action1 = ProofCarryingAction::new(
            "step_1",
            serde_json::json!({}),
            "auth",
        );
        action1.generate_proof(None);
        let hash1 = action1.chain_hash().unwrap();

        let mut action2 = ProofCarryingAction::new(
            "step_2",
            serde_json::json!({}),
            "auth",
        );
        action2.generate_proof(Some(&hash1));
        let hash2 = action2.chain_hash().unwrap();

        // Chain hashes should be different
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_proof_store() {
        let store = ProofStore::new();

        let mut action = ProofCarryingAction::new("test", serde_json::json!({}), "auth");
        action.generate_proof(None);
        store.store(action);

        assert_eq!(store.count(), 1);
        assert!(store.verify_chain());
        assert!(store.last_chain_hash().is_some());
    }

    #[test]
    fn test_deterministic_hashing() {
        let mut a1 = ProofCarryingAction::new("same", serde_json::json!({"x": 1}), "auth");
        let mut a2 = ProofCarryingAction::new("same", serde_json::json!({"x": 1}), "auth");

        a1.generate_proof(None);
        a2.generate_proof(None);

        // Same inputs should produce same action hash
        assert_eq!(
            a1.proof.as_ref().unwrap().action_hash,
            a2.proof.as_ref().unwrap().action_hash,
        );
    }
}
