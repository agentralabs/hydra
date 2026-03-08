use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum HydraError {
    #[error("Could not understand the request: {0}")]
    CompilationError(String), // E001

    #[error("No action could be determined from the request")]
    NoActionDetected, // E002

    #[error("No protocols found to execute this action")]
    NoProtocolsFound, // E101

    #[error("All protocols failed for this action")]
    AllProtocolsFailed(String), // E102

    #[error("Deployment failed: {0}")]
    DeploymentFailed(String), // E201

    #[error("This action requires your approval before proceeding")]
    ApprovalRequired, // E202

    #[error("The operation timed out")]
    Timeout, // E203

    #[error("Sister '{0}' is not available")]
    SisterNotFound(String), // E301

    #[error("Sister '{0}' is not responding")]
    SisterUnreachable(String), // E302

    #[error("Permission denied: {0}")]
    PermissionDenied(String), // E401

    #[error("Configuration error: {0}")]
    ConfigError(String), // E501

    #[error("I/O error: {0}")]
    IoError(String), // E502

    #[error("Receipt chain is broken at sequence {0}")]
    ReceiptChainBroken(u64), // E601

    #[error("Token budget exceeded: needed {needed}, available {available}")]
    TokenBudgetExceeded { needed: u64, available: u64 }, // E701

    #[error("Session not found: {0}")]
    SessionNotFound(String), // E801

    #[error("Serialization error: {0}")]
    SerializationError(String), // E901

    #[error("Internal error: {0}")]
    Internal(String), // E999
}

impl HydraError {
    /// Human-friendly message following the 3-part template:
    /// "{What happened}. {Why it matters}. {What you can do}."
    pub fn user_message(&self) -> String {
        match self {
            Self::CompilationError(_) => {
                "I didn't quite understand that. The request may be too vague or unusual. Try rephrasing with more detail.".to_string()
            }
            Self::NoActionDetected => {
                "I'm not sure what action to take. I need a clearer instruction. Try something like 'create a file' or 'run the tests'.".to_string()
            }
            Self::NoProtocolsFound => {
                "I don't have a way to do that right now. The required capability isn't installed. Check if the needed sister modules are available.".to_string()
            }
            Self::AllProtocolsFailed(detail) => {
                format!("I tried several approaches but none worked. {detail}. Try a simpler approach or check the logs for details.")
            }
            Self::DeploymentFailed(detail) => {
                format!("Something went wrong during execution. {detail}. Check the output above for clues, or try running the steps manually.")
            }
            Self::ApprovalRequired => {
                "This action needs your approval before I proceed. It was flagged for review due to its risk level. Please approve or deny to continue.".to_string()
            }
            Self::Timeout => {
                "The operation took too long and was stopped. This might be due to network issues or a heavy workload. Try again, or increase the timeout in config.".to_string()
            }
            Self::SisterNotFound(name) => {
                format!("The {name} module isn't configured yet. It's needed for this operation. Install or configure the {name} sister module to enable this feature.")
            }
            Self::SisterUnreachable(name) => {
                format!("I can't reach the {name} module right now. It might be offline or unresponsive. Check if the {name} service is running.")
            }
            Self::PermissionDenied(detail) => {
                format!("I don't have permission to do that. {detail}. Run with elevated permissions or choose a different target.")
            }
            Self::ConfigError(detail) => {
                format!("There's a configuration issue. {detail}. Check ~/.hydra/config.toml for problems.")
            }
            Self::IoError(detail) => {
                format!("I had trouble reading or writing a file. {detail}. Check that the file exists and you have the right permissions.")
            }
            Self::ReceiptChainBroken(seq) => {
                format!("The audit trail has a gap at entry {seq}. This could indicate data corruption. Run 'hydra doctor --verify-data' to investigate.")
            }
            Self::TokenBudgetExceeded { needed, available } => {
                format!(
                    "I need {needed} tokens but only have {available}. The budget is too low for this operation. Wait for the budget to reset, or increase it in config."
                )
            }
            Self::SessionNotFound(_) => {
                "That session doesn't exist anymore. It may have expired or been cleaned up. Start a new session to continue.".to_string()
            }
            Self::SerializationError(_) => {
                "I had trouble processing some data. The format may be corrupted or incompatible. Try the operation again.".to_string()
            }
            Self::Internal(detail) => {
                format!("Something unexpected happened internally. {detail}. If this persists, please report the issue.")
            }
        }
    }

    /// Suggested action for recovery — every error now has one
    pub fn suggested_action(&self) -> Option<String> {
        Some(match self {
            Self::CompilationError(_) => {
                "Try rephrasing your request with more detail.".to_string()
            }
            Self::NoActionDetected => {
                "Try being more specific, e.g. 'create a file' or 'run the tests'.".to_string()
            }
            Self::NoProtocolsFound => {
                "Check if the required sister modules are installed.".to_string()
            }
            Self::AllProtocolsFailed(_) => {
                "Try a simpler approach or check logs for details.".to_string()
            }
            Self::DeploymentFailed(_) => {
                "Check the output for clues, or try running steps manually.".to_string()
            }
            Self::ApprovalRequired => "Review the action and approve or deny it.".to_string(),
            Self::Timeout => "Try again, or increase the timeout in config.".to_string(),
            Self::SisterNotFound(name) => {
                format!("Install or configure the {name} sister module.")
            }
            Self::SisterUnreachable(name) => {
                format!("Check if the {name} service is running.")
            }
            Self::PermissionDenied(_) => {
                "Run with elevated permissions or choose a different target.".to_string()
            }
            Self::TokenBudgetExceeded { .. } => {
                "Wait for the budget to reset, or increase it in config.".to_string()
            }
            Self::ConfigError(_) => "Check ~/.hydra/config.toml for issues.".to_string(),
            Self::IoError(_) => "Check file permissions and that the path exists.".to_string(),
            Self::ReceiptChainBroken(_) => {
                "Run 'hydra doctor --verify-data' to investigate.".to_string()
            }
            Self::SessionNotFound(_) => "Start a new session to continue.".to_string(),
            Self::SerializationError(_) => "Try the operation again.".to_string(),
            Self::Internal(_) => "If this persists, please report the issue.".to_string(),
        })
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Self::CompilationError(_) => "E001",
            Self::NoActionDetected => "E002",
            Self::NoProtocolsFound => "E101",
            Self::AllProtocolsFailed(_) => "E102",
            Self::DeploymentFailed(_) => "E201",
            Self::ApprovalRequired => "E202",
            Self::Timeout => "E203",
            Self::SisterNotFound(_) => "E301",
            Self::SisterUnreachable(_) => "E302",
            Self::PermissionDenied(_) => "E401",
            Self::ConfigError(_) => "E501",
            Self::IoError(_) => "E502",
            Self::ReceiptChainBroken(_) => "E601",
            Self::TokenBudgetExceeded { .. } => "E701",
            Self::SessionNotFound(_) => "E801",
            Self::SerializationError(_) => "E901",
            Self::Internal(_) => "E999",
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout | Self::SisterUnreachable(_) | Self::IoError(_)
        )
    }
}

// ── From<T> conversions for all relevant error types ──

impl From<std::io::Error> for HydraError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for HydraError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerializationError(e.to_string())
    }
}

impl From<String> for HydraError {
    fn from(e: String) -> Self {
        Self::Internal(e)
    }
}

impl From<&str> for HydraError {
    fn from(e: &str) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<uuid::Error> for HydraError {
    fn from(e: uuid::Error) -> Self {
        Self::SerializationError(e.to_string())
    }
}

impl From<toml::de::Error> for HydraError {
    fn from(e: toml::de::Error) -> Self {
        Self::ConfigError(e.to_string())
    }
}
