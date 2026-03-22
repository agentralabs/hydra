//! TUI error types.

use thiserror::Error;

/// All errors that can occur within the hydra-tui.
#[derive(Debug, Error)]
pub enum TuiError {
    /// Terminal dimensions are below the minimum required.
    #[error("Terminal too small: {width}x{height} (minimum {min_width}x{min_height})")]
    TerminalTooSmall {
        /// Current width.
        width: u16,
        /// Current height.
        height: u16,
        /// Minimum width.
        min_width: u16,
        /// Minimum height.
        min_height: u16,
    },

    /// A rendering operation failed.
    #[error("Render error: {reason}")]
    RenderError {
        /// What went wrong.
        reason: String,
    },

    /// An input handling error occurred.
    #[error("Input error: {reason}")]
    InputError {
        /// What went wrong.
        reason: String,
    },

    /// An unknown command was entered.
    #[error("Unknown command: '{command}'")]
    UnknownCommand {
        /// The command that was not recognized.
        command: String,
    },

    /// Crossterm backend error.
    #[error("Terminal backend error: {0}")]
    Backend(#[from] std::io::Error),
}
