//! Ghost cursor type definitions — modes, actions, and enums.

use serde::{Deserialize, Serialize};

use super::state::GhostCursorState;

// ═══════════════════════════════════════════════════════════
// CURSOR MODE
// ═══════════════════════════════════════════════════════════

/// Operating mode for the ghost cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorMode {
    /// Full visibility — smooth animation, trail, action labels.
    Visible,
    /// 10x speed — minimal trail, no labels.
    Fast,
    /// No cursor shown — background execution only.
    Invisible,
    /// Replaying a recorded session at adjustable speed.
    Replay,
}

impl CursorMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Visible => "Visible",
            Self::Fast => "Fast",
            Self::Invisible => "Invisible",
            Self::Replay => "Replay",
        }
    }

    pub fn speed_multiplier(&self) -> f64 {
        match self {
            Self::Visible => 1.0,
            Self::Fast => 10.0,
            Self::Invisible => 0.0,
            Self::Replay => 1.0,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// CURSOR ACTION
// ═══════════════════════════════════════════════════════════

/// A structured cursor movement event used by the cognitive loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorMove {
    pub x: f64,
    pub y: f64,
    pub label: Option<String>,
    pub duration_ms: Option<u64>,
}

impl CursorMove {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y, label: None, duration_ms: None }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Apply this move to a GhostCursorState.
    pub fn apply(&self, state: &mut GhostCursorState) {
        state.move_to(self.x, self.y, self.label.clone());
    }
}

/// An action the ghost cursor can perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum CursorAction {
    /// Move cursor to screen coordinates.
    MoveTo {
        x: f64,
        y: f64,
        label: Option<String>,
    },
    /// Click at current position.
    Click {
        button: MouseButton,
    },
    /// Double-click at current position.
    DoubleClick,
    /// Type text at current position.
    TypeText {
        text: String,
    },
    /// Press a key combination.
    KeyPress {
        keys: String,
    },
    /// Scroll at current position.
    Scroll {
        direction: ScrollDirection,
        amount: i32,
    },
    /// Show the cursor.
    Show,
    /// Hide the cursor.
    Hide,
    /// Pause for a duration (ms).
    Wait {
        ms: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Visual state for cursor animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorVisualState {
    /// Normal resting state.
    Idle,
    /// Moving between positions.
    Moving,
    /// Click animation.
    Clicking,
    /// Typing animation (pulsing).
    Typing,
    /// Waiting / thinking.
    Thinking,
    /// Error state.
    Error,
}

/// A single trail dot left behind by cursor movement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailDot {
    pub x: f64,
    pub y: f64,
    pub age_ms: u64,
}
