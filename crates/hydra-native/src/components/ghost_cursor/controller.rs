//! Cursor controller — session recording, interpolation helpers, OS automation, and SVG generation.

use serde::{Deserialize, Serialize};

use super::types::{CursorAction, CursorMode, ScrollDirection};

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
    svg.push_str(r##"<rect x="8" y="8" width="16" height="14" rx="3" fill="#6495ED"/>"##);
    svg.push_str(r##"<circle cx="13" cy="14" r="2" fill="white"/>"##);
    svg.push_str(r##"<circle cx="19" cy="14" r="2" fill="white"/>"##);
    svg.push_str(&format!("<circle cx=\"{lpx:.1}\" cy=\"{lpy:.1}\" r=\"1\" fill=\"#111\"/>"));
    svg.push_str(&format!("<circle cx=\"{rpx:.1}\" cy=\"{rpy:.1}\" r=\"1\" fill=\"#111\"/>"));
    svg.push_str(r##"<line x1="16" y1="8" x2="16" y2="3" stroke="#6495ED" stroke-width="2"/>"##);
    svg.push_str(r##"<circle cx="16" cy="2" r="2" fill="#6495ED"/>"##);
    svg.push_str(r##"<rect x="10" y="22" width="12" height="6" rx="2" fill="#6495ED" opacity="0.7"/>"##);
    svg.push_str(r##"<polygon points="8,28 8,32 12,30" fill="#6495ED"/>"##);
    svg.push_str("</svg>");
    svg
}
