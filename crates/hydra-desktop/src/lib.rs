//! hydra-desktop — desktop automation for Hydra.
//!
//! Screen capture, input simulation, application management,
//! window orchestration, and clipboard monitoring.
//! Cross-platform: macOS (osascript/screencapture) and Linux (xdotool/wmctrl).

pub mod accessibility;
pub mod agent;
pub mod agent_amm;
pub mod app;

pub mod clipboard;
pub mod document;
pub mod constants;
pub mod errors;
pub mod input;
pub mod ocr;
pub mod orchestrator;
pub mod screen;
pub mod gesture;
pub mod observer;
pub mod presence;
pub mod visual_analysis;
pub mod webcam;
pub mod perception;
pub mod app_model;
pub mod verification;
pub mod state_graph;
pub mod deps;

// ── Re-exports ──

pub use app::{AppManager, WindowInfo};
pub use clipboard::{ClipboardContentType, ClipboardEvent, ClipboardMonitor};
pub use document::{DocumentContent, DocumentType};
pub use errors::DesktopError;
pub use input::InputSimulator;
pub use orchestrator::{MonitoredWindow, TileLayout, WindowOrchestrator};
pub use gesture::{Gesture, GestureClassifier, GestureCommand};
pub use observer::FileObserver;
pub use presence::{PresenceEngine, PresenceState};
pub use screen::{Rect, ScreenCapture, ScreenshotInfo};
pub use webcam::FrameDigest;
