//! hydra-desktop — desktop automation for Hydra.
//!
//! Screen capture, input simulation, application management,
//! window orchestration, and clipboard monitoring.
//! Cross-platform: macOS (osascript/screencapture) and Linux (xdotool/wmctrl).

pub mod accessibility;
pub mod agent;
pub mod app;

pub mod clipboard;
pub mod constants;
pub mod errors;
pub mod input;
pub mod ocr;
pub mod orchestrator;
pub mod screen;
pub mod visual_analysis;

// ── Re-exports ──

pub use app::{AppManager, WindowInfo};
pub use clipboard::{ClipboardContentType, ClipboardEvent, ClipboardMonitor};
pub use errors::DesktopError;
pub use input::InputSimulator;
pub use orchestrator::{MonitoredWindow, TileLayout, WindowOrchestrator};
pub use screen::{Rect, ScreenCapture, ScreenshotInfo};
