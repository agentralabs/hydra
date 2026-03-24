//! Browser automation constants — all configurable defaults.

/// Default navigation timeout in milliseconds.
pub const NAVIGATION_TIMEOUT_MS: u64 = 30_000;

/// Default action timeout in milliseconds.
pub const ACTION_TIMEOUT_MS: u64 = 10_000;

/// Default viewport width.
pub const DEFAULT_VIEWPORT_WIDTH: u32 = 1920;

/// Default viewport height.
pub const DEFAULT_VIEWPORT_HEIGHT: u32 = 1080;

/// Cookie storage directory under ~/.hydra/browser/cookies/.
pub const COOKIE_DIR: &str = "browser/cookies";

/// Minimum delay between human-like actions (ms).
pub const HUMAN_MIN_DELAY_MS: u64 = 80;

/// Maximum delay between human-like actions (ms).
pub const HUMAN_MAX_DELAY_MS: u64 = 350;

/// Typing cadence minimum delay per character (ms).
pub const TYPING_MIN_MS: u64 = 30;

/// Typing cadence maximum delay per character (ms).
pub const TYPING_MAX_MS: u64 = 120;

/// Number of bezier interpolation points for mouse curves.
pub const BEZIER_POINTS: usize = 20;

/// Click position jitter radius in pixels.
pub const JITTER_RADIUS_PX: f64 = 3.0;

/// Maximum computer-use steps before giving up.
pub const MAX_COMPUTER_USE_STEPS: u32 = 25;

/// Vision budget — max calls per hour (0 = unlimited).
pub const VISION_BUDGET_PER_HOUR: u32 = 100;

/// CAPTCHA retry limit.
pub const CAPTCHA_MAX_RETRIES: u32 = 3;

/// Login retry limit.
pub const LOGIN_MAX_RETRIES: u32 = 3;

/// Session expiry check — if cookies older than this (seconds), re-validate.
pub const SESSION_STALE_SECONDS: i64 = 3600;
