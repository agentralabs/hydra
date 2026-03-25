//! Vault encryption — AES-256-GCM for credentials at rest.
//! Passphrase from HYDRA_VAULT_PASSPHRASE env var.
//! Backward compatible: if no passphrase set, vault files remain plaintext.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use sha2::{Digest, Sha256};

/// Encrypted data structure.
#[derive(Debug, Clone)]
pub struct EncryptedData {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// Vault crypto errors.
#[derive(Debug, thiserror::Error)]
pub enum VaultCryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("No vault passphrase set (HYDRA_VAULT_PASSPHRASE)")]
    NoPassphrase,
    #[error("I/O error: {0}")]
    Io(String),
}

/// Derive a 256-bit key from a passphrase using SHA256.
/// (Simple derivation — for production, consider argon2.)
fn derive_key(passphrase: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(passphrase.as_bytes());
    h.update(b"hydra-vault-key-derivation-v1");
    let result = h.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Get the vault passphrase from environment.
pub fn get_passphrase() -> Option<String> {
    std::env::var("HYDRA_VAULT_PASSPHRASE").ok()
}

/// Check if vault encryption is enabled.
pub fn is_encryption_enabled() -> bool {
    get_passphrase().is_some()
}

/// Encrypt plaintext bytes with the vault passphrase.
pub fn encrypt(plaintext: &[u8]) -> Result<EncryptedData, VaultCryptoError> {
    let passphrase = get_passphrase().ok_or(VaultCryptoError::NoPassphrase)?;
    let key_bytes = derive_key(&passphrase);
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Generate random 96-bit nonce
    let mut nonce_bytes = [0u8; 12];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| VaultCryptoError::EncryptionFailed(e.to_string()))?;

    Ok(EncryptedData {
        nonce: nonce_bytes.to_vec(),
        ciphertext,
    })
}

/// Decrypt ciphertext with the vault passphrase.
pub fn decrypt(data: &EncryptedData) -> Result<Vec<u8>, VaultCryptoError> {
    let passphrase = get_passphrase().ok_or(VaultCryptoError::NoPassphrase)?;
    let key_bytes = derive_key(&passphrase);
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    if data.nonce.len() != 12 {
        return Err(VaultCryptoError::DecryptionFailed(
            "Invalid nonce length".into(),
        ));
    }
    let nonce = Nonce::from_slice(&data.nonce);

    cipher
        .decrypt(nonce, data.ciphertext.as_ref())
        .map_err(|e| VaultCryptoError::DecryptionFailed(e.to_string()))
}

/// Encrypt a vault file in place. Writes nonce (12 bytes) + ciphertext.
pub fn encrypt_file(path: &std::path::Path) -> Result<(), VaultCryptoError> {
    if !is_encryption_enabled() {
        eprintln!("hydra: vault encryption skipped (HYDRA_VAULT_PASSPHRASE not set)");
        return Ok(());
    }

    let plaintext = std::fs::read(path).map_err(|e| VaultCryptoError::Io(e.to_string()))?;
    let encrypted = encrypt(&plaintext)?;

    // Write: 12 bytes nonce + rest is ciphertext
    let mut output = encrypted.nonce;
    output.extend_from_slice(&encrypted.ciphertext);

    let enc_path = path.with_extension("toml.enc");
    std::fs::write(&enc_path, &output).map_err(|e| VaultCryptoError::Io(e.to_string()))?;
    // SEC-1: Restrict vault file permissions to owner-only
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&enc_path, std::fs::Permissions::from_mode(0o600)); }

    // Remove plaintext file
    let _ = std::fs::remove_file(path);
    eprintln!("hydra: encrypted vault file → {}", enc_path.display());
    Ok(())
}

/// Decrypt a vault file. Reads nonce (12 bytes) + ciphertext.
pub fn decrypt_file(path: &std::path::Path) -> Result<String, VaultCryptoError> {
    let data = std::fs::read(path).map_err(|e| VaultCryptoError::Io(e.to_string()))?;
    if data.len() < 12 {
        return Err(VaultCryptoError::DecryptionFailed("File too short".into()));
    }

    let nonce = data[..12].to_vec();
    let ciphertext = data[12..].to_vec();

    let plaintext = decrypt(&EncryptedData { nonce, ciphertext })?;
    String::from_utf8(plaintext).map_err(|e| VaultCryptoError::DecryptionFailed(e.to_string()))
}

/// Read a vault TOML file, decrypting if needed.
/// Tries .toml.enc first, then .toml (backward compatible).
pub fn read_vault_file(service: &str) -> Result<String, VaultCryptoError> {
    let vault_dir = std::path::Path::new("vault");
    let enc_path = vault_dir.join(format!("{service}.toml.enc"));
    let plain_path = vault_dir.join(format!("{service}.toml"));

    if enc_path.exists() && is_encryption_enabled() {
        return decrypt_file(&enc_path);
    }
    if plain_path.exists() {
        return std::fs::read_to_string(&plain_path)
            .map_err(|e| VaultCryptoError::Io(e.to_string()));
    }
    Err(VaultCryptoError::Io(format!(
        "No vault file for service '{service}'"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("HYDRA_VAULT_PASSPHRASE", "test-passphrase-42") };
        let plaintext = b"api_key = \"sk-secret-key-12345\"";
        let encrypted = encrypt(plaintext).unwrap();
        assert_ne!(encrypted.ciphertext, plaintext);
        assert_eq!(encrypted.nonce.len(), 12);
        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
        unsafe { std::env::remove_var("HYDRA_VAULT_PASSPHRASE") };
    }

    #[test]
    fn no_passphrase_returns_error() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("HYDRA_VAULT_PASSPHRASE") };
        let result = encrypt(b"test");
        assert!(result.is_err());
    }

    #[test]
    fn wrong_passphrase_fails_decrypt() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("HYDRA_VAULT_PASSPHRASE", "correct-passphrase") };
        let encrypted = encrypt(b"secret data").unwrap();
        unsafe { std::env::set_var("HYDRA_VAULT_PASSPHRASE", "wrong-passphrase") };
        let result = decrypt(&encrypted);
        assert!(result.is_err());
        unsafe { std::env::remove_var("HYDRA_VAULT_PASSPHRASE") };
    }

    #[test]
    fn derive_key_deterministic() {
        let k1 = derive_key("test");
        let k2 = derive_key("test");
        assert_eq!(k1, k2);
        let k3 = derive_key("different");
        assert_ne!(k1, k3);
    }

    #[test]
    fn encryption_disabled_without_passphrase() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("HYDRA_VAULT_PASSPHRASE") };
        assert!(!is_encryption_enabled());
    }
}
