//! Ghost Cursor component — the visible robot pointer that shows Hydra working.
//!
//! Renders a distinct robot cursor overlay that moves independently of the user's mouse,
//! showing exactly what Hydra is doing in real-time. Supports multiple modes (visible, fast,
//! invisible, replay) and persists all cursor actions for audit trail replay.

pub mod types;
pub mod state;
pub mod controller;
mod tests;

// Re-export all public items at the module level for backwards compatibility.
pub use types::{
    CursorMode, CursorMove, CursorAction, MouseButton, ScrollDirection,
    CursorVisualState, TrailDot,
};
pub use state::GhostCursorState;
pub use controller::{
    CursorEvent, CursorSession, OsAutomation,
    interpolate_arc, movement_duration_ms, cursor_distance, cursor_svg,
};
