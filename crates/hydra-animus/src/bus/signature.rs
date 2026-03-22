//! Ed25519 signing and verification for Animus bus messages.

use crate::{constants::SIGNATURE_SIZE, errors::AnimusError};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

/// A signing keypair for bus message authentication.
pub struct BusSigningKey {
    signing_key: SigningKey,
}

impl BusSigningKey {
    /// Generate a new random signing key.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// The corresponding verifying key (public key).
    pub fn verifying_key(&self) -> BusVerifyingKey {
        BusVerifyingKey {
            verifying_key: self.signing_key.verifying_key(),
        }
    }

    /// Sign a message. Returns the signature bytes.
    pub fn sign(&self, message_id: &str, payload: &[u8]) -> Vec<u8> {
        let digest = message_digest(message_id, payload);
        let sig: Signature = self.signing_key.sign(&digest);
        sig.to_bytes().to_vec()
    }
}

/// A verifying (public) key for checking bus message authenticity.
#[derive(Clone)]
pub struct BusVerifyingKey {
    verifying_key: VerifyingKey,
}

impl BusVerifyingKey {
    /// Verify a signature over a message.
    pub fn verify(
        &self,
        message_id: &str,
        payload: &[u8],
        signature: &[u8],
    ) -> Result<(), AnimusError> {
        if signature.len() != SIGNATURE_SIZE {
            return Err(AnimusError::SignatureVerificationFailed {
                message_id: message_id.to_string(),
            });
        }

        let mut sig_bytes = [0u8; SIGNATURE_SIZE];
        sig_bytes.copy_from_slice(signature);
        let sig = Signature::from_bytes(&sig_bytes);

        let digest = message_digest(message_id, payload);
        self.verifying_key.verify(&digest, &sig).map_err(|_| {
            AnimusError::SignatureVerificationFailed {
                message_id: message_id.to_string(),
            }
        })
    }
}

/// Compute the digest signed/verified: SHA256(message_id || payload).
fn message_digest(message_id: &str, payload: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(message_id.as_bytes());
    hasher.update(b"||");
    hasher.update(payload);
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify() {
        let key = BusSigningKey::generate();
        let vk = key.verifying_key();
        let payload = b"hello animus";
        let sig = key.sign("msg-001", payload);
        assert!(vk.verify("msg-001", payload, &sig).is_ok());
    }

    #[test]
    fn wrong_payload_fails() {
        let key = BusSigningKey::generate();
        let vk = key.verifying_key();
        let sig = key.sign("msg-001", b"correct");
        assert!(vk.verify("msg-001", b"tampered", &sig).is_err());
    }

    #[test]
    fn wrong_message_id_fails() {
        let key = BusSigningKey::generate();
        let vk = key.verifying_key();
        let sig = key.sign("msg-001", b"payload");
        assert!(vk.verify("msg-002", b"payload", &sig).is_err());
    }

    #[test]
    fn empty_signature_rejected() {
        let key = BusSigningKey::generate();
        let vk = key.verifying_key();
        assert!(vk.verify("msg-001", b"payload", &[]).is_err());
    }
}
