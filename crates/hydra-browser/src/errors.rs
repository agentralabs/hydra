//! Browser error types — all failure modes for browser automation.

use thiserror::Error;

/// All browser operation errors.
#[derive(Debug, Error, Clone)]
pub enum BrowserError {
    /// Chrome/Chromium binary not found on this system.
    #[error("Chrome not found: {0}. Install Chrome or set HYDRA_CHROME_PATH")]
    ChromeNotFound(String),

    /// Failed to launch browser process.
    #[error("Browser launch failed: {0}")]
    LaunchFailed(String),

    /// Navigation to URL failed.
    #[error("Navigation failed for '{url}': {reason}")]
    NavigationFailed { url: String, reason: String },

    /// Page interaction failed (click, type, scroll).
    #[error("Action failed: {action} — {reason}")]
    ActionFailed { action: String, reason: String },

    /// Screenshot capture failed.
    #[error("Screenshot failed: {0}")]
    ScreenshotFailed(String),

    /// Session/cookie operation failed.
    #[error("Session error for '{domain}': {reason}")]
    SessionError { domain: String, reason: String },

    /// Login failed after all attempts.
    #[error("Login failed for '{domain}': {reason}")]
    LoginFailed { domain: String, reason: String },

    /// No credentials found in vault for domain.
    #[error("No credentials in vault for '{0}'")]
    NoCredentials(String),

    /// CAPTCHA could not be solved.
    #[error("CAPTCHA unsolvable on '{domain}': {reason}")]
    CaptchaUnsolvable { domain: String, reason: String },

    /// Vision provider returned an error.
    #[error("Vision error: {0}")]
    VisionError(String),

    /// Computer use agent exceeded step limit.
    #[error("Task exceeded {max_steps} steps without completion")]
    StepLimitExceeded { max_steps: u32 },

    /// Generic I/O error.
    #[error("I/O error: {0}")]
    Io(String),
}
