//! Ghost Cursor component — the visible robot pointer that shows Hydra working.
//!
//! Renders a distinct robot cursor overlay that moves independently of the user's mouse,
//! showing exactly what Hydra is doing in real-time. Supports multiple modes (visible, fast,
//! invisible, replay) and persists all cursor actions for audit trail replay.

use serde::{Deserialize, Serialize};

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

// ═══════════════════════════════════════════════════════════
// CURSOR STATE
// ═══════════════════════════════════════════════════════════

/// A single trail dot left behind by cursor movement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailDot {
    pub x: f64,
    pub y: f64,
    pub age_ms: u64,
}

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

// ═══════════════════════════════════════════════════════════
// CURSOR CONTROLLER (Backend Logic)
// ═══════════════════════════════════════════════════════════

/// Recorded cursor event for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorEvent {
    /// Milliseconds since session start.
    pub timestamp_ms: u64,
    /// The action performed.
    pub action: CursorAction,
    /// Screen coordinates at time of action.
    pub x: f64,
    pub y: f64,
}

/// A complete cursor session recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorSession {
    pub id: String,
    pub task_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub events: Vec<CursorEvent>,
    pub mode: CursorMode,
    pub total_duration_ms: u64,
}

impl CursorSession {
    pub fn new(task_id: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
            events: Vec::new(),
            mode: CursorMode::Visible,
            total_duration_ms: 0,
        }
    }

    pub fn record(&mut self, action: CursorAction, x: f64, y: f64) {
        self.events.push(CursorEvent {
            timestamp_ms: self.total_duration_ms,
            action,
            x,
            y,
        });
    }

    pub fn finish(&mut self) {
        self.ended_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

// ═══════════════════════════════════════════════════════════
// INTERPOLATION HELPERS
// ═══════════════════════════════════════════════════════════

/// Generate interpolated positions for smooth cursor movement.
pub fn interpolate_arc(
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    steps: usize,
) -> Vec<(f64, f64)> {
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        // Smooth ease-in-out
        let ease = t * t * (3.0 - 2.0 * t);
        // Slight arc above the straight line
        let arc = (t * std::f64::consts::PI).sin() * 20.0;
        let x = start_x + (end_x - start_x) * ease;
        let y = start_y + (end_y - start_y) * ease - arc;
        points.push((x, y));
    }
    points
}

/// Duration for cursor movement based on distance and mode.
pub fn movement_duration_ms(distance: f64, mode: CursorMode) -> u64 {
    let base_ms = (distance * 1.5).min(800.0).max(100.0) as u64;
    match mode {
        CursorMode::Visible => base_ms,
        CursorMode::Fast => base_ms / 10,
        CursorMode::Invisible => 0,
        CursorMode::Replay => base_ms,
    }
}

/// Calculate distance between two screen points.
pub fn cursor_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
}

// ═══════════════════════════════════════════════════════════
// OS AUTOMATION (Platform-Specific)
// ═══════════════════════════════════════════════════════════

/// Platform-agnostic OS automation commands.
pub struct OsAutomation;

impl OsAutomation {
    /// Click at screen coordinates using OS-level automation.
    pub fn click_at(x: f64, y: f64) -> Option<std::process::Output> {
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("osascript")
                .arg("-e")
                .arg(format!(
                    "tell application \"System Events\" to click at {{{}, {}}}",
                    x as i64, y as i64
                ))
                .output()
                .ok()
        }
        #[cfg(target_os = "linux")]
        {
            // Try xdotool first, fall back to ydotool for Wayland
            std::process::Command::new("xdotool")
                .args(["mousemove", "--sync", &(x as i64).to_string(), &(y as i64).to_string()])
                .output()
                .ok();
            std::process::Command::new("xdotool")
                .args(["click", "1"])
                .output()
                .ok()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    /// Type text using OS-level keystroke injection.
    pub fn type_text(text: &str) -> Option<std::process::Output> {
        #[cfg(target_os = "macos")]
        {
            // Escape special characters for osascript
            let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
            std::process::Command::new("osascript")
                .arg("-e")
                .arg(format!(
                    "tell application \"System Events\" to keystroke \"{}\"",
                    escaped
                ))
                .output()
                .ok()
        }
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdotool")
                .args(["type", "--clearmodifiers", text])
                .output()
                .ok()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    /// Press a key combination (e.g., "Return", "cmd+c").
    pub fn key_press(keys: &str) -> Option<std::process::Output> {
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("osascript")
                .arg("-e")
                .arg(format!(
                    "tell application \"System Events\" to key code {} using {{}}",
                    match keys {
                        "Return" | "Enter" => "36",
                        "Tab" => "48",
                        "Escape" => "53",
                        "Space" => "49",
                        _ => "0",
                    }
                ))
                .output()
                .ok()
        }
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdotool")
                .args(["key", keys])
                .output()
                .ok()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    /// Scroll at current mouse position.
    pub fn scroll(direction: ScrollDirection, amount: i32) -> Option<std::process::Output> {
        #[cfg(target_os = "macos")]
        {
            let scroll_amount = match direction {
                ScrollDirection::Up => amount,
                ScrollDirection::Down => -amount,
                _ => 0,
            };
            std::process::Command::new("osascript")
                .arg("-e")
                .arg(format!(
                    "tell application \"System Events\" to scroll area 1 by {{0, {}}}",
                    scroll_amount
                ))
                .output()
                .ok()
        }
        #[cfg(target_os = "linux")]
        {
            let btn = match direction {
                ScrollDirection::Up => "4",
                ScrollDirection::Down => "5",
                ScrollDirection::Left => "6",
                ScrollDirection::Right => "7",
            };
            std::process::Command::new("xdotool")
                .args(["click", "--repeat", &amount.to_string(), btn])
                .output()
                .ok()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    /// Move the OS mouse cursor to position (for sync with ghost cursor).
    pub fn move_mouse(x: f64, y: f64) -> Option<std::process::Output> {
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("osascript")
                .arg("-e")
                .arg(format!(
                    "do shell script \"cliclick m:{},{}\"",
                    x as i64, y as i64
                ))
                .output()
                .ok()
        }
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdotool")
                .args(["mousemove", "--sync", &(x as i64).to_string(), &(y as i64).to_string()])
                .output()
                .ok()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }
}

// ═══════════════════════════════════════════════════════════
// SVG GENERATION
// ═══════════════════════════════════════════════════════════

/// Generate the robot cursor SVG string with dynamic pupil position.
pub fn cursor_svg(pupil_dx: f64, pupil_dy: f64) -> String {
    let lpx = 13.0 + pupil_dx;
    let lpy = 14.0 + pupil_dy;
    let rpx = 19.0 + pupil_dx;
    let rpy = 14.0 + pupil_dy;
    let mut svg = String::with_capacity(600);
    svg.push_str(r##"<svg width="32" height="32" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">"##);
    svg.push_str(r##"<rect x="8" y="8" width="16" height="14" rx="3" fill="#ea580c"/>"##);
    svg.push_str(r##"<circle cx="13" cy="14" r="2" fill="white"/>"##);
    svg.push_str(r##"<circle cx="19" cy="14" r="2" fill="white"/>"##);
    svg.push_str(&format!("<circle cx=\"{lpx:.1}\" cy=\"{lpy:.1}\" r=\"1\" fill=\"#111\"/>"));
    svg.push_str(&format!("<circle cx=\"{rpx:.1}\" cy=\"{rpy:.1}\" r=\"1\" fill=\"#111\"/>"));
    svg.push_str(r##"<line x1="16" y1="8" x2="16" y2="3" stroke="#ea580c" stroke-width="2"/>"##);
    svg.push_str(r##"<circle cx="16" cy="2" r="2" fill="#ea580c"/>"##);
    svg.push_str(r##"<rect x="10" y="22" width="12" height="6" rx="2" fill="#ea580c" opacity="0.7"/>"##);
    svg.push_str(r##"<polygon points="8,28 8,32 12,30" fill="#ea580c"/>"##);
    svg.push_str("</svg>");
    svg
}

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cursor_state() {
        let state = GhostCursorState::new();
        assert!(!state.visible);
        assert_eq!(state.mode, CursorMode::Visible);
        assert_eq!(state.x, 0.0);
        assert_eq!(state.y, 0.0);
        assert!(state.trail.is_empty());
    }

    #[test]
    fn test_move_to_updates_position() {
        let mut state = GhostCursorState::new();
        state.show();
        state.move_to(100.0, 200.0, Some("Opening Chrome".into()));
        assert_eq!(state.x, 100.0);
        assert_eq!(state.y, 200.0);
        assert_eq!(state.action_label, Some("Opening Chrome".into()));
        assert_eq!(state.visual_state, CursorVisualState::Moving);
    }

    #[test]
    fn test_trail_grows_on_move() {
        let mut state = GhostCursorState::new();
        state.show();
        state.move_to(10.0, 20.0, None);
        state.move_to(30.0, 40.0, None);
        state.move_to(50.0, 60.0, None);
        assert_eq!(state.trail.len(), 3); // origin + 2 previous positions
    }

    #[test]
    fn test_trail_capped_at_max() {
        let mut state = GhostCursorState::new();
        state.show();
        state.max_trail = 5;
        for i in 0..20 {
            state.move_to(i as f64, i as f64, None);
        }
        assert!(state.trail.len() <= 5);
    }

    #[test]
    fn test_invisible_mode_no_trail() {
        let mut state = GhostCursorState::new();
        state.set_mode(CursorMode::Invisible);
        state.move_to(100.0, 200.0, None);
        assert!(state.trail.is_empty());
        assert!(!state.visible);
    }

    #[test]
    fn test_click_visual_state() {
        let mut state = GhostCursorState::new();
        state.click();
        assert_eq!(state.visual_state, CursorVisualState::Clicking);
    }

    #[test]
    fn test_typing_visual_state() {
        let mut state = GhostCursorState::new();
        state.start_typing();
        assert_eq!(state.visual_state, CursorVisualState::Typing);
    }

    #[test]
    fn test_pause_and_resume() {
        let mut state = GhostCursorState::new();
        state.pause();
        assert!(state.paused);
        assert_eq!(state.visual_state, CursorVisualState::Thinking);
        state.resume();
        assert!(!state.paused);
        assert_eq!(state.visual_state, CursorVisualState::Idle);
    }

    #[test]
    fn test_css_class_generation() {
        let mut state = GhostCursorState::new();
        assert!(state.css_class().contains("ghost-cursor-hidden"));

        state.show();
        state.click();
        let cls = state.css_class();
        assert!(cls.contains("ghost-cursor-clicking"));
        assert!(!cls.contains("ghost-cursor-hidden"));
    }

    #[test]
    fn test_pupil_offset() {
        let mut state = GhostCursorState::new();
        state.move_to(100.0, 0.0, None); // Move right
        let (dx, dy) = state.pupil_offset();
        assert!(dx > 0.0); // Looking right
        assert!(dy.abs() < 0.5); // Not much vertical
    }

    #[test]
    fn test_interpolate_arc() {
        let points = interpolate_arc(0.0, 0.0, 100.0, 100.0, 10);
        assert_eq!(points.len(), 11);
        assert_eq!(points[0], (0.0, 0.0));
        // End point should be close to target (arc offset returns to 0 at end)
        let last = points.last().unwrap();
        assert!((last.0 - 100.0).abs() < 0.01);
        assert!((last.1 - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_movement_duration() {
        let d = movement_duration_ms(500.0, CursorMode::Visible);
        assert!(d >= 100 && d <= 800);

        let fast = movement_duration_ms(500.0, CursorMode::Fast);
        assert!(fast < d);

        let invisible = movement_duration_ms(500.0, CursorMode::Invisible);
        assert_eq!(invisible, 0);
    }

    #[test]
    fn test_cursor_distance() {
        let d = cursor_distance(0.0, 0.0, 3.0, 4.0);
        assert!((d - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_cursor_session_recording() {
        let mut session = CursorSession::new("task-1");
        assert_eq!(session.event_count(), 0);

        session.record(CursorAction::MoveTo { x: 100.0, y: 200.0, label: None }, 100.0, 200.0);
        session.record(CursorAction::Click { button: MouseButton::Left }, 100.0, 200.0);
        assert_eq!(session.event_count(), 2);

        session.finish();
        assert!(session.ended_at.is_some());
    }

    #[test]
    fn test_cursor_svg_generation() {
        let svg = cursor_svg(0.5, -0.3);
        assert!(svg.contains("svg"));
        assert!(svg.contains("ea580c"));
        assert!(svg.contains("13.5")); // left pupil shifted
    }

    #[test]
    fn test_tick_trail_removes_old() {
        let mut state = GhostCursorState::new();
        state.show();
        state.move_to(10.0, 10.0, None);
        state.move_to(20.0, 20.0, None);
        assert_eq!(state.trail.len(), 2);

        // Age past expiry
        state.tick_trail(1100);
        assert!(state.trail.is_empty());
    }

    #[test]
    fn test_mode_switching() {
        let mut state = GhostCursorState::new();
        state.show();
        assert!(state.trail_enabled);

        state.set_mode(CursorMode::Fast);
        assert!(!state.trail_enabled);

        state.set_mode(CursorMode::Visible);
        assert!(state.trail_enabled);

        state.set_mode(CursorMode::Replay);
        assert_eq!(state.replay_progress, 0.0);
    }

    #[test]
    fn test_cursor_action_serialization() {
        let action = CursorAction::MoveTo { x: 100.0, y: 200.0, label: Some("test".into()) };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("move_to"));
        let restored: CursorAction = serde_json::from_str(&json).unwrap();
        match restored {
            CursorAction::MoveTo { x, y, label } => {
                assert_eq!(x, 100.0);
                assert_eq!(y, 200.0);
                assert_eq!(label, Some("test".into()));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_cursor_mode_speed_multiplier() {
        assert_eq!(CursorMode::Visible.speed_multiplier(), 1.0);
        assert_eq!(CursorMode::Fast.speed_multiplier(), 10.0);
        assert_eq!(CursorMode::Invisible.speed_multiplier(), 0.0);
    }

    #[test]
    fn test_hide_clears_state() {
        let mut state = GhostCursorState::new();
        state.show();
        state.move_to(100.0, 200.0, Some("test".into()));
        state.hide();
        assert!(!state.visible);
        assert!(state.action_label.is_none());
        assert!(state.trail.is_empty());
    }

    #[test]
    fn test_transform_style() {
        let mut state = GhostCursorState::new();
        state.move_to(150.5, 300.0, None);
        let style = state.transform_style();
        assert!(style.contains("150.5"));
        assert!(style.contains("300"));
    }
}
