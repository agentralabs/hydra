//! Text input component data.

use serde::{Deserialize, Serialize};

/// Props for the text input component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputProps {
    pub placeholder: String,
    pub disabled: bool,
    pub max_length: usize,
}

impl Default for InputProps {
    fn default() -> Self {
        Self {
            placeholder: "Ask Hydra anything...".into(),
            disabled: false,
            max_length: 10_000,
        }
    }
}

/// Input validation result
#[derive(Debug, Clone)]
pub struct InputValidation {
    pub valid: bool,
    pub trimmed: String,
    pub error: Option<String>,
}

/// Validate input before sending
pub fn validate_input(raw: &str, max_length: usize) -> InputValidation {
    let trimmed = raw.trim().to_string();

    if trimmed.is_empty() {
        return InputValidation {
            valid: false,
            trimmed,
            error: Some("Message cannot be empty".into()),
        };
    }

    if trimmed.len() > max_length {
        let len = trimmed.len();
        return InputValidation {
            valid: false,
            trimmed,
            error: Some(format!("Message too long ({} > {} chars)", len, max_length)),
        };
    }

    InputValidation {
        valid: true,
        trimmed,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_input() {
        let result = validate_input("Hello", 1000);
        assert!(result.valid);
        assert_eq!(result.trimmed, "Hello");
    }

    #[test]
    fn test_empty_input() {
        let result = validate_input("  ", 1000);
        assert!(!result.valid);
    }

    #[test]
    fn test_too_long() {
        let result = validate_input("a".repeat(101).as_str(), 100);
        assert!(!result.valid);
        assert!(result.error.unwrap().contains("too long"));
    }

    #[test]
    fn test_trims_whitespace() {
        let result = validate_input("  hello  ", 1000);
        assert!(result.valid);
        assert_eq!(result.trimmed, "hello");
    }
}
