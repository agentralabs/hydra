//! Ghost cursor state — the UI view model for cursor position, trail, and visual state.

use serde::{Deserialize, Serialize};

use super::types::{CursorMode, CursorVisualState, TrailDot};

/// Current state of the ghost cursor (UI view model).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostCursorState {
    /// Current X position on screen.
    pub x: f64,
    /// Current Y position on screen.
    pub y: f64,
    /// Whether the cursor is currently visible.
    pub visible: bool,
    /// Current operating mode.
    pub mode: CursorMode,
    /// Action label shown next to cursor (e.g., "Opening Chrome").
    pub action_label: Option<String>,
    /// Visual state for animation.
    pub visual_state: CursorVisualState,
    /// Trail dots (most recent movement path).
    pub trail: Vec<TrailDot>,
    /// Maximum trail dots to keep.
    pub max_trail: usize,
    /// Whether trail rendering is enabled.
    pub trail_enabled: bool,
    /// Direction the cursor pupils should look (radians).
    pub pupil_angle: f64,
    /// Active session ID for recording.
    pub session_id: Option<String>,
    /// Whether currently recording actions.
    pub recording: bool,
    /// Replay speed multiplier (1.0 = normal, 2.0 = 2x, 0.5 = half).
    pub replay_speed: f64,
    /// Replay progress (0.0 to 1.0).
    pub replay_progress: f64,
    /// Whether the cursor is paused (user moved their mouse).
    pub paused: bool,
}

impl GhostCursorState {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            visible: false,
            mode: CursorMode::Visible,
            action_label: None,
            visual_state: CursorVisualState::Idle,
            trail: Vec::new(),
            max_trail: 50,
            trail_enabled: true,
            pupil_angle: 0.0,
            session_id: None,
            recording: false,
            replay_speed: 1.0,
            replay_progress: 0.0,
            paused: false,
        }
    }

    /// Move cursor to a new position, updating trail and pupil direction.
    pub fn move_to(&mut self, x: f64, y: f64, label: Option<String>) {
        if self.mode == CursorMode::Invisible {
            self.x = x;
            self.y = y;
            return;
        }

        // Calculate pupil direction
        let dx = x - self.x;
        let dy = y - self.y;
        if dx.abs() > 0.1 || dy.abs() > 0.1 {
            self.pupil_angle = dy.atan2(dx);
        }

        // Add trail dot at previous position
        if self.trail_enabled && self.visible {
            self.trail.push(TrailDot {
                x: self.x,
                y: self.y,
                age_ms: 0,
            });
            if self.trail.len() > self.max_trail {
                self.trail.remove(0);
            }
        }

        self.x = x;
        self.y = y;
        self.action_label = label;
        self.visual_state = CursorVisualState::Moving;
    }

    /// Trigger click animation.
    pub fn click(&mut self) {
        self.visual_state = CursorVisualState::Clicking;
    }

    /// Start typing animation.
    pub fn start_typing(&mut self) {
        self.visual_state = CursorVisualState::Typing;
    }

    /// Return to idle.
    pub fn idle(&mut self) {
        self.visual_state = CursorVisualState::Idle;
        self.action_label = None;
    }

    /// Show the cursor.
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the cursor and clear trail.
    pub fn hide(&mut self) {
        self.visible = false;
        self.action_label = None;
        self.trail.clear();
    }

    /// Pause cursor (user interaction detected).
    pub fn pause(&mut self) {
        self.paused = true;
        self.visual_state = CursorVisualState::Thinking;
        self.action_label = Some("Paused — your turn".into());
    }

    /// Resume after pause.
    pub fn resume(&mut self) {
        self.paused = false;
        self.visual_state = CursorVisualState::Idle;
        self.action_label = None;
    }

    /// Set the cursor mode.
    pub fn set_mode(&mut self, mode: CursorMode) {
        self.mode = mode;
        match mode {
            CursorMode::Invisible => self.hide(),
            CursorMode::Fast => {
                self.trail_enabled = false;
                self.action_label = None;
            }
            CursorMode::Visible => {
                self.trail_enabled = true;
            }
            CursorMode::Replay => {
                self.trail_enabled = true;
                self.replay_progress = 0.0;
            }
        }
    }

    /// Age all trail dots by `dt_ms` and remove expired ones (>1000ms).
    pub fn tick_trail(&mut self, dt_ms: u64) {
        for dot in &mut self.trail {
            dot.age_ms += dt_ms;
        }
        self.trail.retain(|d| d.age_ms < 1000);
    }

    /// CSS class for the cursor container.
    pub fn css_class(&self) -> String {
        let mut classes = vec!["ghost-cursor"];
        if !self.visible {
            classes.push("ghost-cursor-hidden");
        }
        match self.visual_state {
            CursorVisualState::Clicking => classes.push("ghost-cursor-clicking"),
            CursorVisualState::Typing => classes.push("ghost-cursor-typing"),
            CursorVisualState::Thinking => classes.push("ghost-cursor-thinking"),
            CursorVisualState::Moving => classes.push("ghost-cursor-moving"),
            CursorVisualState::Error => classes.push("ghost-cursor-error"),
            CursorVisualState::Idle => {}
        }
        if self.paused {
            classes.push("ghost-cursor-paused");
        }
        match self.mode {
            CursorMode::Fast => classes.push("ghost-cursor-fast"),
            CursorMode::Replay => classes.push("ghost-cursor-replay"),
            _ => {}
        }
        classes.join(" ")
    }

    /// CSS transform for cursor position.
    pub fn transform_style(&self) -> String {
        format!("left: {}px; top: {}px;", self.x, self.y)
    }

    /// Pupil offset for eye-tracking effect.
    pub fn pupil_offset(&self) -> (f64, f64) {
        let r = 0.8;
        (self.pupil_angle.cos() * r, self.pupil_angle.sin() * r)
    }
}
