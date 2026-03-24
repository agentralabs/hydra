//! Desktop automation constants.

/// Default screenshot quality (PNG compression level 0-9).
pub const SCREENSHOT_COMPRESSION: u8 = 6;

/// Mouse movement minimum step delay (ms).
pub const MOUSE_STEP_DELAY_MS: u64 = 5;

/// Typing minimum delay per character (ms).
pub const TYPING_MIN_DELAY_MS: u64 = 25;

/// Typing maximum delay per character (ms).
pub const TYPING_MAX_DELAY_MS: u64 = 100;

/// Clipboard poll interval (ms).
pub const CLIPBOARD_POLL_MS: u64 = 500;

/// Maximum windows to enumerate.
pub const MAX_WINDOWS: usize = 100;

/// Click position jitter radius (pixels).
pub const CLICK_JITTER_PX: f64 = 2.0;
