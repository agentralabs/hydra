//! ProofVerifier — verify proof-carrying actions.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::action::{ProofCarryingAction, ProofStatus};

/// Result of proof verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub action_id: String,
    pub valid: bool,
    pub checks_passed: usize,
    pub checks_total: usize,
    pub errors: Vec<String>,
}

/// Verifies cryptographic proofs on actions
pub struct ProofVerifier {
    verified: parking_lot::RwLock<Vec<VerificationResult>>,
}

impl ProofVerifier {
    pub fn new() -> Self {
        Self {
            verified: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Verify a proof-carrying action
    pub fn verify(&self, action: &mut ProofCarryingAction) -> VerificationResult {
        let mut errors = Vec::new();
        let mut checks_passed = 0;
        let checks_total = 4;

        let proof = match &action.proof {
            Some(p) => p.clone(),
            None => {
                let result = VerificationResult {
                    action_id: action.id.clone(),
                    valid: false,
                    checks_passed: 0,
                    checks_total,
                    errors: vec!["No proof attached".into()],
                };
                self.verified.write().push(result.clone());
                return result;
            }
        };

        // Check 1: Action hash matches
        let expected_action_hash =
            hash_content(&serde_json::to_string(&action.params).unwrap_or_default());
        if proof.action_hash == expected_action_hash {
            checks_passed += 1;
        } else {
            errors.push("Action hash mismatch".into());
        }

        // Check 2: Precondition hash matches
        let expected_pre_hash = hash_content(&action.preconditions.join(","));
        if proof.precondition_hash == expected_pre_hash {
            checks_passed += 1;
        } else {
            errors.push("Precondition hash mismatch".into());
        }

        // Check 3: Postcondition hash matches
        let expected_post_hash = hash_content(&action.postconditions.join(","));
        if proof.postcondition_hash == expected_post_hash {
            checks_passed += 1;
        } else {
            errors.push("Postcondition hash mismatch".into());
        }

        // Check 4: Authorization hash matches
        let expected_auth_hash = hash_content(&action.authorization);
        if proof.authorization_hash == expected_auth_hash {
            checks_passed += 1;
        } else {
            errors.push("Authorization hash mismatch".into());
        }

        let valid = checks_passed == checks_total;

        // Update proof status
        if let Some(p) = &mut action.proof {
            p.status = if valid {
                ProofStatus::Verified
            } else {
                ProofStatus::Failed
            };
        }

        let result = VerificationResult {
            action_id: action.id.clone(),
            valid,
            checks_passed,
            checks_total,
            errors,
        };

        self.verified.write().push(result.clone());
        result
    }

    /// Get verification statistics
    pub fn stats(&self) -> (usize, usize) {
        let verified = self.verified.read();
        let valid = verified.iter().filter(|v| v.valid).count();
        (valid, verified.len())
    }
}

impl Default for ProofVerifier {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_verification() {
        let verifier = ProofVerifier::new();
        let mut action = ProofCarryingAction::new("test", serde_json::json!({"x": 1}), "auth");
        action.generate_proof(None);

        let result = verifier.verify(&mut action);
        assert!(result.valid);
        assert_eq!(result.checks_passed, 4);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_tampered_action_fails() {
        let verifier = ProofVerifier::new();
        let mut action = ProofCarryingAction::new("test", serde_json::json!({"x": 1}), "auth");
        action.generate_proof(None);

        // Tamper with params after proof generation
        action.params = serde_json::json!({"x": 999});

        let result = verifier.verify(&mut action);
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_no_proof_fails() {
        let verifier = ProofVerifier::new();
        let mut action = ProofCarryingAction::new("test", serde_json::json!({}), "auth");

        let result = verifier.verify(&mut action);
        assert!(!result.valid);
        assert_eq!(result.checks_passed, 0);
    }

    #[test]
    fn test_verifier_stats() {
        let verifier = ProofVerifier::new();

        let mut good = ProofCarryingAction::new("good", serde_json::json!({}), "auth");
        good.generate_proof(None);
        verifier.verify(&mut good);

        let mut bad = ProofCarryingAction::new("bad", serde_json::json!({}), "auth");
        verifier.verify(&mut bad); // no proof

        let (valid, total) = verifier.stats();
        assert_eq!(valid, 1);
        assert_eq!(total, 2);
    }
}
