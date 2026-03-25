//! Desktop automation error types.

use thiserror::Error;

/// All desktop operation errors.
#[derive(Debug, Error, Clone)]
pub enum DesktopError {
    /// Screen capture failed.
    #[error("Screen capture failed: {0}")]
    CaptureFailed(String),

    /// Input simulation failed.
    #[error("Input failed: {action} — {reason}")]
    InputFailed { action: String, reason: String },

    /// Application management failed.
    #[error("App error: {app} — {reason}")]
    AppError { app: String, reason: String },

    /// Window not found.
    #[error("Window not found: {0}")]
    WindowNotFound(String),

    /// Clipboard operation failed.
    #[error("Clipboard error: {0}")]
    ClipboardError(String),

    /// Platform not supported for this operation.
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    /// Vision analysis error.
    #[error("Vision error: {0}")]
    VisionError(String),

    /// Generic I/O error.
    #[error("I/O error: {0}")]
    Io(String),

    /// Camera/webcam operation failed (O19).
    #[error("Camera error: {0}")]
    CameraError(String),

    /// Permission denied for camera or screen access (O19).
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}
