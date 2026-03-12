use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendlyError {
    pub message: String,
    pub explanation: String,
    pub options: Vec<ErrorOption>,
    pub icon_state: String,
    pub can_undo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorOption {
    pub label: String,
    pub action: ErrorAction,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorAction {
    Retry,
    Dismiss,
    Undo,
    Remind,
    ContactSupport,
}

impl FriendlyError {
    /// Convert a technical error string into a friendly, human-readable error.
    /// NEVER exposes error codes, stack traces, or technical jargon.
    pub fn from_technical(error: &str) -> Self {
        let lower = error.to_lowercase();

        if lower.contains("connection refused")
            || lower.contains("connection reset")
            || lower.contains("network")
            || lower.contains("dns")
        {
            Self::connection_lost()
        } else if lower.contains("rate limit") || lower.contains("429") || lower.contains("too many") {
            Self::rate_limited()
        } else if lower.contains("timeout") || lower.contains("timed out") {
            Self::timeout()
        } else {
            Self::generic("Something didn't go as planned")
        }
    }

    pub fn connection_lost() -> Self {
        Self {
            message: "I'm having trouble connecting.".to_string(),
            explanation: "Let me try again in a moment.".to_string(),
            options: vec![
                ErrorOption {
                    label: "Try again".to_string(),
                    action: ErrorAction::Retry,
                    is_primary: true,
                },
                ErrorOption {
                    label: "Dismiss".to_string(),
                    action: ErrorAction::Dismiss,
                    is_primary: false,
                },
            ],
            icon_state: "dimmed".to_string(),
            can_undo: false,
        }
    }

    pub fn rate_limited() -> Self {
        Self {
            message: "I need to take a short break.".to_string(),
            explanation: "I'll be ready in a minute.".to_string(),
            options: vec![
                ErrorOption {
                    label: "Remind me".to_string(),
                    action: ErrorAction::Remind,
                    is_primary: true,
                },
                ErrorOption {
                    label: "Dismiss".to_string(),
                    action: ErrorAction::Dismiss,
                    is_primary: false,
                },
            ],
            icon_state: "resting".to_string(),
            can_undo: false,
        }
    }

    pub fn timeout() -> Self {
        Self {
            message: "That took longer than expected.".to_string(),
            explanation: "Want me to try again?".to_string(),
            options: vec![
                ErrorOption {
                    label: "Try again".to_string(),
                    action: ErrorAction::Retry,
                    is_primary: true,
                },
                ErrorOption {
                    label: "Dismiss".to_string(),
                    action: ErrorAction::Dismiss,
                    is_primary: false,
                },
            ],
            icon_state: "dimmed".to_string(),
            can_undo: false,
        }
    }

    pub fn generic(message: &str) -> Self {
        Self {
            message: message.to_string(),
            explanation: "I'll sort this out. No worries.".to_string(),
            options: vec![
                ErrorOption {
                    label: "Try again".to_string(),
                    action: ErrorAction::Retry,
                    is_primary: true,
                },
                ErrorOption {
                    label: "Dismiss".to_string(),
                    action: ErrorAction::Dismiss,
                    is_primary: false,
                },
                ErrorOption {
                    label: "Contact support".to_string(),
                    action: ErrorAction::ContactSupport,
                    is_primary: false,
                },
            ],
            icon_state: "dimmed".to_string(),
            can_undo: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_error_is_friendly() {
        let err = FriendlyError::from_technical("connection refused: tcp://127.0.0.1:8080");
        assert!(err.message.contains("trouble connecting"));
        assert!(!err.message.contains("refused"));
        assert!(!err.message.contains("tcp"));
        assert!(!err.message.contains("127.0.0.1"));
    }

    #[test]
    fn rate_limit_error_is_friendly() {
        let err = FriendlyError::from_technical("HTTP 429 rate limited");
        assert!(err.message.contains("short break"));
        assert!(!err.message.contains("429"));
        assert!(!err.message.contains("HTTP"));
    }

    #[test]
    fn timeout_error_is_friendly() {
        let err = FriendlyError::from_technical("request timed out after 30000ms");
        assert!(err.message.contains("longer than expected"));
        assert!(!err.message.contains("30000"));
        assert!(!err.message.contains("timed out"));
    }

    #[test]
    fn generic_error_is_friendly() {
        let err = FriendlyError::from_technical("NullPointerException at com.example.Main:42");
        assert!(!err.message.contains("NullPointer"));
        assert!(!err.message.contains("Exception"));
        assert!(!err.message.contains("42"));
    }

    #[test]
    fn all_errors_have_options() {
        let errors = vec![
            FriendlyError::connection_lost(),
            FriendlyError::rate_limited(),
            FriendlyError::timeout(),
            FriendlyError::generic("test"),
        ];
        for err in &errors {
            assert!(!err.options.is_empty(), "Error should have options");
            assert!(
                err.options.iter().any(|o| o.is_primary),
                "Error should have a primary option"
            );
        }
    }
}
