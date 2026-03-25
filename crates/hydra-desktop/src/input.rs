//! InputSimulator — cross-platform mouse and keyboard simulation.
//! Uses shell commands (osascript on macOS, xdotool on Linux) for portability.
//! Human-like behavior: bezier curves, typing cadence, click jitter.

use crate::constants::{CLICK_JITTER_PX, MOUSE_STEP_DELAY_MS, TYPING_MAX_DELAY_MS, TYPING_MIN_DELAY_MS};
use crate::errors::DesktopError;
use rand::Rng;

/// Simulates mouse and keyboard input.
pub struct InputSimulator {
    current_x: f64,
    current_y: f64,
}

impl InputSimulator {
    pub fn new() -> Self {
        Self {
            current_x: 0.0,
            current_y: 0.0,
        }
    }

    /// Move mouse to (x, y) with human-like bezier curve.
    pub fn mouse_move(&mut self, x: f64, y: f64) -> Result<(), DesktopError> {
        let points = Self::bezier_curve(self.current_x, self.current_y, x, y, 15);

        for (px, py) in &points {
            Self::platform_mouse_move(*px as i32, *py as i32)?;
            std::thread::sleep(std::time::Duration::from_millis(MOUSE_STEP_DELAY_MS));
        }

        self.current_x = x;
        self.current_y = y;
        Ok(())
    }

    /// Click at current position.
    pub fn click(&self) -> Result<(), DesktopError> {
        let (jx, jy) = self.jitter(self.current_x, self.current_y);
        Self::platform_click(jx as i32, jy as i32)
    }

    /// Move to (x, y) and click.
    pub fn click_at(&mut self, x: f64, y: f64) -> Result<(), DesktopError> {
        self.mouse_move(x, y)?;
        self.click()
    }

    /// Double-click at current position.
    pub fn double_click(&self) -> Result<(), DesktopError> {
        Self::platform_double_click(self.current_x as i32, self.current_y as i32)
    }

    /// Right-click at current position.
    pub fn right_click(&self) -> Result<(), DesktopError> {
        Self::platform_right_click(self.current_x as i32, self.current_y as i32)
    }

    /// Type text with human-like per-character delays.
    pub fn key_type(&self, text: &str) -> Result<(), DesktopError> {
        let mut rng = rand::thread_rng();
        for ch in text.chars() {
            Self::platform_key_char(ch)?;
            let delay = rng.gen_range(TYPING_MIN_DELAY_MS..=TYPING_MAX_DELAY_MS);
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
        Ok(())
    }

    /// Press a single key (Enter, Tab, Escape, etc.).
    pub fn key_press(&self, key: &str) -> Result<(), DesktopError> {
        Self::platform_key_press(key)
    }

    /// Press a key combination (e.g., "cmd+s", "ctrl+c").
    pub fn key_combo(&self, modifier: &str, key: &str) -> Result<(), DesktopError> {
        Self::platform_key_combo(modifier, key)
    }

    pub fn position(&self) -> (f64, f64) {
        (self.current_x, self.current_y)
    }

    /// Move to target using Fitts's Law timing + minimum-jerk trajectory.
    /// target_width: approximate width of the target element in pixels.
    /// This is physically indistinguishable from human motor control.
    pub fn move_to_target(&mut self, x: f64, y: f64, target_width: f64) -> Result<(), DesktopError> {
        let dx = x - self.current_x;
        let dy = y - self.current_y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance < 1.0 {
            self.current_x = x; self.current_y = y;
            return Ok(());
        }
        // Fitts's Law: movement time based on target distance/width
        let a = 50.0_f64;  // base time (ms)
        let b = 150.0_f64; // scaling factor
        let id = (distance / target_width.max(1.0) + 1.0).log2(); // index of difficulty
        let duration_ms = (a + b * id) as u64;
        let steps = (duration_ms / 5).max(5) as usize; // 5ms per step

        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            // Minimum-jerk trajectory: 5th order polynomial
            // x(t) = 10t³ - 15t⁴ + 6t⁵  (bell-shaped velocity profile)
            let frac = 10.0 * t.powi(3) - 15.0 * t.powi(4) + 6.0 * t.powi(5);
            let px = self.current_x + dx * frac;
            let py = self.current_y + dy * frac;
            Self::platform_mouse_move(px as i32, py as i32)?;
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        self.current_x = x;
        self.current_y = y;
        Ok(())
    }

    /// Click at target with Fitts's Law approach + coordinate validation.
    pub fn click_target(
        &mut self, x: f64, y: f64, target_width: f64,
        space: &crate::perception::CoordinateSpace,
    ) -> Result<(), DesktopError> {
        // Apply scale factor and validate bounds
        let (px, py) = space.to_physical(x, y);
        if !space.validate(px, py) {
            return Err(DesktopError::AppError {
                app: "input".into(),
                reason: format!("coordinates ({px:.0}, {py:.0}) out of screen bounds"),
            });
        }
        self.move_to_target(px, py, target_width)?;
        self.click()
    }

    fn jitter(&self, x: f64, y: f64) -> (f64, f64) {
        let mut rng = rand::thread_rng();
        let dx = rng.gen_range(-CLICK_JITTER_PX..=CLICK_JITTER_PX);
        let dy = rng.gen_range(-CLICK_JITTER_PX..=CLICK_JITTER_PX);
        (x + dx, y + dy)
    }

    /// Public test helper for bezier curve generation.
    pub fn bezier_test(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<(f64, f64)> {
        Self::bezier_curve(x0, y0, x1, y1, 15)
    }

    fn bezier_curve(x0: f64, y0: f64, x1: f64, y1: f64, n: usize) -> Vec<(f64, f64)> {
        let mut rng = rand::thread_rng();
        let cx = (x0 + x1) / 2.0 + rng.gen_range(-40.0..40.0);
        let cy = (y0 + y1) / 2.0 + rng.gen_range(-40.0..40.0);
        (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                let u = 1.0 - t;
                let x = u * u * x0 + 2.0 * u * t * cx + t * t * x1;
                let y = u * u * y0 + 2.0 * u * t * cy + t * t * y1;
                (x, y)
            })
            .collect()
    }

    // ── Platform-specific implementations ──

    fn platform_mouse_move(x: i32, y: i32) -> Result<(), DesktopError> {
        if cfg!(target_os = "macos") {
            Self::run_osascript(&format!(
                r#"tell application "System Events" to set position of mouse to {{{x}, {y}}}"#
            ))
        } else if cfg!(target_os = "linux") {
            Self::run_cmd("xdotool", &["mousemove", &x.to_string(), &y.to_string()])
        } else {
            Err(DesktopError::UnsupportedPlatform("mouse_move".into()))
        }
    }

    fn platform_click(x: i32, y: i32) -> Result<(), DesktopError> {
        if cfg!(target_os = "macos") {
            Self::run_osascript(&format!(
                r#"do shell script "cliclick c:{x},{y}""#
            )).or_else(|_| {
                // Fallback if cliclick not installed
                Self::run_osascript(&format!(
                    r#"tell application "System Events" to click at {{{x}, {y}}}"#
                ))
            })
        } else if cfg!(target_os = "linux") {
            Self::run_cmd("xdotool", &["mousemove", &x.to_string(), &y.to_string(), "click", "1"])
        } else {
            Err(DesktopError::UnsupportedPlatform("click".into()))
        }
    }

    fn platform_double_click(x: i32, y: i32) -> Result<(), DesktopError> {
        if cfg!(target_os = "linux") {
            Self::run_cmd("xdotool", &["mousemove", &x.to_string(), &y.to_string(), "click", "--repeat", "2", "1"])
        } else {
            // macOS: two clicks with short delay
            Self::platform_click(x, y)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            Self::platform_click(x, y)
        }
    }

    fn platform_right_click(x: i32, y: i32) -> Result<(), DesktopError> {
        if cfg!(target_os = "linux") {
            Self::run_cmd("xdotool", &["mousemove", &x.to_string(), &y.to_string(), "click", "3"])
        } else if cfg!(target_os = "macos") {
            Self::run_osascript(&format!(
                r#"do shell script "cliclick rc:{x},{y}""#
            ))
        } else {
            Err(DesktopError::UnsupportedPlatform("right_click".into()))
        }
    }

    fn platform_key_char(ch: char) -> Result<(), DesktopError> {
        if cfg!(target_os = "macos") {
            Self::run_osascript(&format!(
                r#"tell application "System Events" to keystroke "{ch}""#
            ))
        } else if cfg!(target_os = "linux") {
            Self::run_cmd("xdotool", &["type", "--clearmodifiers", &ch.to_string()])
        } else {
            Err(DesktopError::UnsupportedPlatform("key_char".into()))
        }
    }

    fn platform_key_press(key: &str) -> Result<(), DesktopError> {
        if cfg!(target_os = "macos") {
            let key_code = Self::macos_key_name(key);
            Self::run_osascript(&format!(
                r#"tell application "System Events" to key code {key_code}"#
            ))
        } else if cfg!(target_os = "linux") {
            let xdo_key = Self::xdotool_key_name(key);
            Self::run_cmd("xdotool", &["key", &xdo_key])
        } else {
            Err(DesktopError::UnsupportedPlatform("key_press".into()))
        }
    }

    fn platform_key_combo(modifier: &str, key: &str) -> Result<(), DesktopError> {
        if cfg!(target_os = "macos") {
            let mod_name = match modifier {
                "cmd" | "command" => "command down",
                "ctrl" | "control" => "control down",
                "alt" | "option" => "option down",
                "shift" => "shift down",
                _ => "command down",
            };
            Self::run_osascript(&format!(
                r#"tell application "System Events" to keystroke "{key}" using {{{mod_name}}}"#
            ))
        } else if cfg!(target_os = "linux") {
            let mod_name = match modifier {
                "cmd" | "command" => "super",
                "ctrl" | "control" => "ctrl",
                "alt" | "option" => "alt",
                "shift" => "shift",
                _ => "ctrl",
            };
            Self::run_cmd("xdotool", &["key", &format!("{mod_name}+{key}")])
        } else {
            Err(DesktopError::UnsupportedPlatform("key_combo".into()))
        }
    }

    fn run_osascript(script: &str) -> Result<(), DesktopError> {
        let output = std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| DesktopError::InputFailed {
                action: "osascript".into(),
                reason: e.to_string(),
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DesktopError::InputFailed {
                action: "osascript".into(),
                reason: stderr.to_string(),
            });
        }
        Ok(())
    }

    fn run_cmd(cmd: &str, args: &[&str]) -> Result<(), DesktopError> {
        let output = std::process::Command::new(cmd)
            .args(args)
            .output()
            .map_err(|e| DesktopError::InputFailed {
                action: cmd.into(),
                reason: e.to_string(),
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DesktopError::InputFailed {
                action: cmd.into(),
                reason: stderr.to_string(),
            });
        }
        Ok(())
    }

    fn macos_key_name(key: &str) -> &str {
        match key.to_lowercase().as_str() {
            "enter" | "return" => "36",
            "tab" => "48",
            "escape" | "esc" => "53",
            "space" => "49",
            "backspace" | "delete" => "51",
            "up" => "126",
            "down" => "125",
            "left" => "123",
            "right" => "124",
            _ => "36", // default to enter
        }
    }

    fn xdotool_key_name(key: &str) -> String {
        match key.to_lowercase().as_str() {
            "enter" | "return" => "Return".into(),
            "tab" => "Tab".into(),
            "escape" | "esc" => "Escape".into(),
            "space" => "space".into(),
            "backspace" | "delete" => "BackSpace".into(),
            "up" => "Up".into(),
            "down" => "Down".into(),
            "left" => "Left".into(),
            "right" => "Right".into(),
            other => other.into(),
        }
    }

    // ── O33: Atomic Input Algebra — composed operations ──

    /// Drag from (x1,y1) to (x2,y2) with smooth minimum-jerk movement.
    pub fn drag(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), DesktopError> {
        let atoms = crate::input_atoms::drag(x1, y1, x2, y2);
        let mut composer = crate::input_atoms::InputComposer::new();
        composer.execute(&atoms)?;
        self.current_x = x2; self.current_y = y2;
        Ok(())
    }

    /// Real wheel scroll at position (not arrow key stub).
    pub fn scroll_wheel(&mut self, x: f64, y: f64, dy: i32) -> Result<(), DesktopError> {
        let atoms = crate::input_atoms::scroll(x, y, dy);
        crate::input_atoms::InputComposer::new().execute(&atoms)
    }

    /// Click with modifier held (Shift+Click, Cmd+Click, Ctrl+Click).
    pub fn click_with_modifier(&mut self, x: f64, y: f64, modifier: &str) -> Result<(), DesktopError> {
        let atoms = crate::input_atoms::modifier_click(x, y, modifier);
        crate::input_atoms::InputComposer::new().execute(&atoms)
    }

    /// Drag with modifier held (Alt+Drag to duplicate, Shift+Drag to constrain).
    pub fn drag_with_modifier(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, modifier: &str) -> Result<(), DesktopError> {
        let atoms = crate::input_atoms::modifier_drag(x1, y1, x2, y2, modifier);
        crate::input_atoms::InputComposer::new().execute(&atoms)?;
        self.current_x = x2; self.current_y = y2;
        Ok(())
    }

    /// Write text to clipboard then paste (Cmd+V / Ctrl+V).
    pub fn paste_text(&mut self, text: &str) -> Result<(), DesktopError> {
        let atoms = crate::input_atoms::clipboard_paste(text);
        crate::input_atoms::InputComposer::new().execute(&atoms)
    }

    /// Wait until screen stops changing (renders complete, downloads finish).
    pub fn wait_for_stable(&self, timeout_ms: u64) -> Result<(), DesktopError> {
        let atoms = crate::input_atoms::wait_stable(timeout_ms);
        crate::input_atoms::InputComposer::new().execute(&atoms)
    }

    /// Wait until specific text appears on screen via OCR.
    pub fn wait_for_text(&self, text: &str, timeout_ms: u64) -> Result<(), DesktopError> {
        let atoms = crate::input_atoms::wait_text(text, timeout_ms);
        crate::input_atoms::InputComposer::new().execute(&atoms)
    }

    /// Multi-modifier key combo (Cmd+Shift+S, Ctrl+Alt+Del).
    pub fn key_combo_multi(&self, modifiers: &[&str], key: &str) -> Result<(), DesktopError> {
        let atoms = crate::input_atoms::multi_modifier_combo(modifiers, key);
        crate::input_atoms::InputComposer::new().execute(&atoms)
    }
}

impl Default for InputSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_curve_endpoints() {
        let points = InputSimulator::bezier_curve(0.0, 0.0, 100.0, 200.0, 10);
        assert_eq!(points.len(), 11);
        assert!((points[0].0).abs() < 0.01);
        assert!((points[0].1).abs() < 0.01);
        let last = points.last().unwrap();
        assert!((last.0 - 100.0).abs() < 0.01);
        assert!((last.1 - 200.0).abs() < 0.01);
    }

    #[test]
    fn initial_position_is_zero() {
        let sim = InputSimulator::new();
        assert_eq!(sim.position(), (0.0, 0.0));
    }

    #[test]
    fn key_mapping_macos() {
        assert_eq!(InputSimulator::macos_key_name("enter"), "36");
        assert_eq!(InputSimulator::macos_key_name("tab"), "48");
        assert_eq!(InputSimulator::macos_key_name("escape"), "53");
    }

    #[test]
    fn key_mapping_linux() {
        assert_eq!(InputSimulator::xdotool_key_name("enter"), "Return");
        assert_eq!(InputSimulator::xdotool_key_name("tab"), "Tab");
        assert_eq!(InputSimulator::xdotool_key_name("escape"), "Escape");
    }
}
