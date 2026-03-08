//! Input validation — no silent fallbacks, every rejection is explicit.

/// A validated intent string, guaranteed non-empty, no null bytes, within length limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedIntent {
    inner: String,
}

impl ValidatedIntent {
    /// Access the validated intent string.
    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

impl std::fmt::Display for ValidatedIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.inner)
    }
}

/// Validation error with no silent fallback — every case is explicit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    Empty,
    TooLong { len: usize, max: usize },
    ContainsNullBytes,
    InvalidFormat(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Input must not be empty"),
            Self::TooLong { len, max } => {
                write!(f, "Input too long: {len} chars exceeds maximum of {max}")
            }
            Self::ContainsNullBytes => write!(f, "Input must not contain null bytes"),
            Self::InvalidFormat(msg) => write!(f, "Invalid format: {msg}"),
        }
    }
}

impl std::error::Error for ValidationError {}

const MAX_INTENT_LEN: usize = 10_000;
const MAX_RUN_ID_LEN: usize = 64;

/// Validate an intent string. Rejects empty, too long (>10000 chars), or null-byte inputs.
pub fn validate_intent(input: &str) -> Result<ValidatedIntent, ValidationError> {
    if input.is_empty() {
        return Err(ValidationError::Empty);
    }
    if input.len() > MAX_INTENT_LEN {
        return Err(ValidationError::TooLong {
            len: input.len(),
            max: MAX_INTENT_LEN,
        });
    }
    if input.contains('\0') {
        return Err(ValidationError::ContainsNullBytes);
    }
    Ok(ValidatedIntent {
        inner: input.to_string(),
    })
}

/// Validate a run ID. Must be alphanumeric + hyphens only, max 64 chars.
pub fn validate_run_id(id: &str) -> Result<&str, ValidationError> {
    if id.is_empty() {
        return Err(ValidationError::Empty);
    }
    if id.len() > MAX_RUN_ID_LEN {
        return Err(ValidationError::TooLong {
            len: id.len(),
            max: MAX_RUN_ID_LEN,
        });
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(ValidationError::InvalidFormat(
            "run ID must contain only alphanumeric characters and hyphens".into(),
        ));
    }
    Ok(id)
}

/// Validate a config key. Must be dotted.path.format (segments of [a-zA-Z0-9_] joined by dots).
pub fn validate_config_key(key: &str) -> Result<&str, ValidationError> {
    if key.is_empty() {
        return Err(ValidationError::Empty);
    }
    let segments: Vec<&str> = key.split('.').collect();
    for segment in &segments {
        if segment.is_empty() {
            return Err(ValidationError::InvalidFormat(
                "config key must not have empty segments (double dots or leading/trailing dot)"
                    .into(),
            ));
        }
        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(ValidationError::InvalidFormat(
                "config key segments must contain only alphanumeric characters and underscores"
                    .into(),
            ));
        }
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    // validate_intent tests
    #[test]
    fn test_validate_intent_valid() {
        let result = validate_intent("Hello, what is the weather?");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "Hello, what is the weather?");
    }

    #[test]
    fn test_validate_intent_empty() {
        assert_eq!(validate_intent(""), Err(ValidationError::Empty));
    }

    #[test]
    fn test_validate_intent_too_long() {
        let long = "x".repeat(10_001);
        let result = validate_intent(&long);
        assert!(matches!(result, Err(ValidationError::TooLong { len: 10001, max: 10000 })));
    }

    #[test]
    fn test_validate_intent_exactly_max() {
        let exact = "x".repeat(10_000);
        assert!(validate_intent(&exact).is_ok());
    }

    #[test]
    fn test_validate_intent_null_bytes() {
        assert_eq!(validate_intent("hello\0world"), Err(ValidationError::ContainsNullBytes));
    }

    #[test]
    fn test_validated_intent_display() {
        let intent = validate_intent("test").unwrap();
        assert_eq!(format!("{}", intent), "test");
    }

    // validate_run_id tests
    #[test]
    fn test_validate_run_id_valid() {
        assert!(validate_run_id("abc-123").is_ok());
        assert!(validate_run_id("run-abc-def-456").is_ok());
    }

    #[test]
    fn test_validate_run_id_empty() {
        assert_eq!(validate_run_id(""), Err(ValidationError::Empty));
    }

    #[test]
    fn test_validate_run_id_too_long() {
        let long = "a".repeat(65);
        assert!(matches!(validate_run_id(&long), Err(ValidationError::TooLong { .. })));
    }

    #[test]
    fn test_validate_run_id_invalid_chars() {
        assert!(matches!(validate_run_id("run/id"), Err(ValidationError::InvalidFormat(_))));
        assert!(matches!(validate_run_id("run id"), Err(ValidationError::InvalidFormat(_))));
        assert!(matches!(validate_run_id("run_id"), Err(ValidationError::InvalidFormat(_))));
    }

    #[test]
    fn test_validate_run_id_alphanumeric_hyphen_only() {
        assert!(validate_run_id("abc123").is_ok());
        assert!(validate_run_id("a-b-c").is_ok());
        assert!(validate_run_id("ABC-123").is_ok());
    }

    // validate_config_key tests
    #[test]
    fn test_validate_config_key_valid() {
        assert!(validate_config_key("server.port").is_ok());
        assert!(validate_config_key("a.b.c").is_ok());
        assert!(validate_config_key("log_level").is_ok());
        assert!(validate_config_key("data_dir").is_ok());
    }

    #[test]
    fn test_validate_config_key_empty() {
        assert_eq!(validate_config_key(""), Err(ValidationError::Empty));
    }

    #[test]
    fn test_validate_config_key_double_dots() {
        assert!(matches!(validate_config_key("a..b"), Err(ValidationError::InvalidFormat(_))));
    }

    #[test]
    fn test_validate_config_key_leading_dot() {
        assert!(matches!(validate_config_key(".server"), Err(ValidationError::InvalidFormat(_))));
    }

    #[test]
    fn test_validate_config_key_trailing_dot() {
        assert!(matches!(validate_config_key("server."), Err(ValidationError::InvalidFormat(_))));
    }

    #[test]
    fn test_validate_config_key_invalid_chars() {
        assert!(matches!(validate_config_key("server-port"), Err(ValidationError::InvalidFormat(_))));
        assert!(matches!(validate_config_key("a/b"), Err(ValidationError::InvalidFormat(_))));
    }

    // ValidationError display tests
    #[test]
    fn test_validation_error_display() {
        assert_eq!(ValidationError::Empty.to_string(), "Input must not be empty");
        assert_eq!(ValidationError::ContainsNullBytes.to_string(), "Input must not contain null bytes");
        let too_long = ValidationError::TooLong { len: 100, max: 50 };
        assert!(too_long.to_string().contains("100"));
        assert!(too_long.to_string().contains("50"));
    }
}
