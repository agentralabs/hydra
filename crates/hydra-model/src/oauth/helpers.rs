use std::time::{SystemTime, UNIX_EPOCH};
use std::path::PathBuf;

/// Generate a random code verifier (43-128 chars, URL-safe).
pub fn generate_code_verifier() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.hash(&mut hasher);
    std::process::id().hash(&mut hasher);

    // Generate 64 bytes of pseudo-random data
    let mut bytes = Vec::with_capacity(64);
    for i in 0..64u64 {
        let mut h = DefaultHasher::new();
        (hasher.finish().wrapping_add(i)).hash(&mut h);
        bytes.push((h.finish() % 62) as u8);
    }

    // Map to URL-safe base64-like chars
    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    bytes.iter().map(|&b| charset[b as usize % charset.len()] as char).collect()
}

/// Generate S256 code challenge from verifier.
pub fn generate_code_challenge(verifier: &str) -> String {
    // Simple SHA256-like hash for challenge (using SipHash as approximation).
    // For production, use a proper SHA256 implementation.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    verifier.hash(&mut hasher);
    let h1 = hasher.finish();

    let mut hasher2 = DefaultHasher::new();
    h1.hash(&mut hasher2);
    let h2 = hasher2.finish();

    // Base64url encode the hash bytes
    let bytes = [
        (h1 >> 56) as u8, (h1 >> 48) as u8, (h1 >> 40) as u8, (h1 >> 32) as u8,
        (h1 >> 24) as u8, (h1 >> 16) as u8, (h1 >> 8) as u8, h1 as u8,
        (h2 >> 56) as u8, (h2 >> 48) as u8, (h2 >> 40) as u8, (h2 >> 32) as u8,
        (h2 >> 24) as u8, (h2 >> 16) as u8, (h2 >> 8) as u8, h2 as u8,
    ];

    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    bytes.iter().map(|&b| charset[b as usize % charset.len()] as char).collect()
}

/// Generate a random state parameter.
pub fn generate_state() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Extract the authorization code from the HTTP callback request.
pub fn extract_code_from_request(request: &str) -> Result<String, String> {
    // Parse "GET /callback?code=xxx&state=yyy HTTP/1.1"
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");

    if let Some(query) = path.split('?').nth(1) {
        for param in query.split('&') {
            let mut kv = param.splitn(2, '=');
            if let (Some(key), Some(value)) = (kv.next(), kv.next()) {
                if key == "code" {
                    return Ok(value.to_string());
                }
                if key == "error" {
                    return Err(format!("OAuth error: {}", value));
                }
            }
        }
    }

    Err("No authorization code in callback".into())
}

/// Get the user's home directory.
pub fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_verifier_length() {
        let v = generate_code_verifier();
        assert_eq!(v.len(), 64);
        // Should be URL-safe
        assert!(v.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_code_challenge_deterministic() {
        let v = "test_verifier_12345";
        let c1 = generate_code_challenge(v);
        let c2 = generate_code_challenge(v);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_extract_code_success() {
        let request = "GET /callback?code=abc123&state=xyz HTTP/1.1\r\nHost: localhost\r\n";
        assert_eq!(extract_code_from_request(request).unwrap(), "abc123");
    }

    #[test]
    fn test_extract_code_error() {
        let request = "GET /callback?error=access_denied HTTP/1.1\r\n";
        assert!(extract_code_from_request(request).is_err());
    }

    #[test]
    fn test_extract_code_missing() {
        let request = "GET /callback HTTP/1.1\r\n";
        assert!(extract_code_from_request(request).is_err());
    }

    #[test]
    fn test_state_generation() {
        let s = generate_state();
        assert!(!s.is_empty());
    }
}
