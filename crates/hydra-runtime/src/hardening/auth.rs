//! Server auth via AGENTIC_TOKEN environment variable.

/// Error from authentication operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    MissingHeader,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingToken => write!(f, "AGENTIC_TOKEN not set in environment"),
            Self::InvalidToken => write!(f, "Invalid authentication token"),
            Self::MissingHeader => write!(f, "Missing Authorization header"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Manages server authentication using the AGENTIC_TOKEN environment variable.
pub struct AuthManager {
    token: Option<String>,
}

impl AuthManager {
    /// Create a new AuthManager, reading AGENTIC_TOKEN from the environment.
    pub fn new() -> Self {
        Self {
            token: std::env::var("AGENTIC_TOKEN").ok(),
        }
    }

    /// Create an AuthManager with an explicit token (useful for testing).
    pub fn with_token(token: Option<String>) -> Self {
        Self { token }
    }

    /// Validate a token against the stored AGENTIC_TOKEN using constant-time comparison.
    pub fn validate_token(&self, token: &str) -> Result<(), AuthError> {
        let expected = self.token.as_deref().ok_or(AuthError::MissingToken)?;
        if constant_time_eq(expected.as_bytes(), token.as_bytes()) {
            Ok(())
        } else {
            Err(AuthError::InvalidToken)
        }
    }

    /// Check the Authorization header from a list of (name, value) pairs.
    /// Expects "Bearer <token>" format.
    pub fn require_auth(&self, headers: &[(String, String)]) -> Result<(), AuthError> {
        let auth_value = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
            .map(|(_, v)| v)
            .ok_or(AuthError::MissingHeader)?;

        let token = auth_value
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidToken)?;

        self.validate_token(token)
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_with_token_valid() {
        let auth = AuthManager::with_token(Some("secret123".into()));
        assert!(auth.validate_token("secret123").is_ok());
    }

    #[test]
    fn test_auth_with_token_invalid() {
        let auth = AuthManager::with_token(Some("secret123".into()));
        assert_eq!(auth.validate_token("wrong"), Err(AuthError::InvalidToken));
    }

    #[test]
    fn test_auth_no_token_set() {
        let auth = AuthManager::with_token(None);
        assert_eq!(auth.validate_token("anything"), Err(AuthError::MissingToken));
    }

    #[test]
    fn test_require_auth_valid_bearer() {
        let auth = AuthManager::with_token(Some("mytoken".into()));
        let headers = vec![("Authorization".into(), "Bearer mytoken".into())];
        assert!(auth.require_auth(&headers).is_ok());
    }

    #[test]
    fn test_require_auth_missing_header() {
        let auth = AuthManager::with_token(Some("mytoken".into()));
        let headers: Vec<(String, String)> = vec![];
        assert_eq!(auth.require_auth(&headers), Err(AuthError::MissingHeader));
    }

    #[test]
    fn test_require_auth_wrong_format() {
        let auth = AuthManager::with_token(Some("mytoken".into()));
        let headers = vec![("Authorization".into(), "Basic mytoken".into())];
        assert_eq!(auth.require_auth(&headers), Err(AuthError::InvalidToken));
    }

    #[test]
    fn test_require_auth_wrong_token() {
        let auth = AuthManager::with_token(Some("correct".into()));
        let headers = vec![("Authorization".into(), "Bearer wrong".into())];
        assert_eq!(auth.require_auth(&headers), Err(AuthError::InvalidToken));
    }

    #[test]
    fn test_require_auth_case_insensitive_header() {
        let auth = AuthManager::with_token(Some("mytoken".into()));
        let headers = vec![("authorization".into(), "Bearer mytoken".into())];
        assert!(auth.require_auth(&headers).is_ok());
    }

    #[test]
    fn test_constant_time_eq_equal() {
        assert!(constant_time_eq(b"hello", b"hello"));
    }

    #[test]
    fn test_constant_time_eq_not_equal() {
        assert!(!constant_time_eq(b"hello", b"world"));
    }

    #[test]
    fn test_constant_time_eq_different_lengths() {
        assert!(!constant_time_eq(b"short", b"longer_string"));
    }

    #[test]
    fn test_auth_error_display() {
        assert!(AuthError::MissingToken.to_string().contains("AGENTIC_TOKEN"));
        assert!(AuthError::InvalidToken.to_string().contains("Invalid"));
        assert!(AuthError::MissingHeader.to_string().contains("Missing"));
    }
}
