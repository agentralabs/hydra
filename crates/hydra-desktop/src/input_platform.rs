//! Platform-specific implementations of the 6 atomic input operations.
//! macOS: osascript + cliclick. Linux: xdotool + xclip.

use crate::errors::DesktopError;
use crate::input_atoms::Target;
use std::process::Command;

// ── PRESS ──

pub fn press(target: &Target) -> Result<(), DesktopError> {
    if cfg!(target_os = "macos") { press_macos(target) }
    else if cfg!(target_os = "linux") { press_linux(target) }
    else { Err(DesktopError::UnsupportedPlatform("press".into())) }
}

fn press_macos(target: &Target) -> Result<(), DesktopError> {
    match target {
        Target::MouseLeft => run_cliclick("dd:."),
        Target::MouseRight => run_cliclick("rd:."),
        Target::MouseMiddle => run_cliclick("md:."),
        Target::Key(k) => run_osascript(&format!(
            r#"tell application "System Events" to key code {} using {{}}"#, macos_key_code(k))),
        Target::Modifier(m) => {
            // Hold modifier: use key down event
            let code = macos_modifier_code(m);
            run_osascript(&format!(r#"tell application "System Events" to key code {code}"#))
        }
    }
}

fn press_linux(target: &Target) -> Result<(), DesktopError> {
    match target {
        Target::MouseLeft => run_xdotool(&["mousedown", "1"]),
        Target::MouseRight => run_xdotool(&["mousedown", "3"]),
        Target::MouseMiddle => run_xdotool(&["mousedown", "2"]),
        Target::Key(k) => run_xdotool(&["keydown", &xdotool_key(k)]),
        Target::Modifier(m) => run_xdotool(&["keydown", &xdotool_modifier(m)]),
    }
}

// ── RELEASE ──

pub fn release(target: &Target) -> Result<(), DesktopError> {
    if cfg!(target_os = "macos") { release_macos(target) }
    else if cfg!(target_os = "linux") { release_linux(target) }
    else { Err(DesktopError::UnsupportedPlatform("release".into())) }
}

fn release_macos(target: &Target) -> Result<(), DesktopError> {
    match target {
        Target::MouseLeft => run_cliclick("du:."),
        Target::MouseRight => run_cliclick("ru:."),
        Target::MouseMiddle => run_cliclick("mu:."),
        Target::Key(k) => run_osascript(&format!(
            r#"tell application "System Events" to key code {} using {{}}"#, macos_key_code(k))),
        Target::Modifier(m) => {
            let code = macos_modifier_code(m);
            run_osascript(&format!(r#"tell application "System Events" to key code {code}"#))
        }
    }
}

fn release_linux(target: &Target) -> Result<(), DesktopError> {
    match target {
        Target::MouseLeft => run_xdotool(&["mouseup", "1"]),
        Target::MouseRight => run_xdotool(&["mouseup", "3"]),
        Target::MouseMiddle => run_xdotool(&["mouseup", "2"]),
        Target::Key(k) => run_xdotool(&["keyup", &xdotool_key(k)]),
        Target::Modifier(m) => run_xdotool(&["keyup", &xdotool_modifier(m)]),
    }
}

// ── MOVE ──

pub fn mouse_move(x: i32, y: i32) -> Result<(), DesktopError> {
    if cfg!(target_os = "macos") {
        // Use cliclick for move (more reliable during drag)
        run_cliclick(&format!("m:{x},{y}"))
    } else if cfg!(target_os = "linux") {
        run_xdotool(&["mousemove", &x.to_string(), &y.to_string()])
    } else {
        Err(DesktopError::UnsupportedPlatform("mouse_move".into()))
    }
}

// ── SCROLL WHEEL ──

pub fn scroll_wheel(dx: i32, dy: i32) -> Result<(), DesktopError> {
    if cfg!(target_os = "macos") {
        // cliclick scroll: negative = down, positive = up
        if dy != 0 { run_cliclick(&format!("w:{dy}"))? }
        if dx != 0 { run_cliclick(&format!("wh:{dx}"))? }
        Ok(())
    } else if cfg!(target_os = "linux") {
        // xdotool: button 4 = scroll up, button 5 = scroll down
        let (btn, count) = if dy > 0 { ("4", dy) } else { ("5", -dy) };
        for _ in 0..count.abs() {
            run_xdotool(&["click", btn])?;
        }
        Ok(())
    } else {
        Err(DesktopError::UnsupportedPlatform("scroll".into()))
    }
}

// ── CLIPBOARD WRITE ──

pub fn clipboard_write(text: &str) -> Result<(), DesktopError> {
    if cfg!(target_os = "macos") {
        // pbcopy via pipe
        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn().map_err(|e| DesktopError::InputFailed {
                action: "clipboard".into(), reason: format!("pbcopy: {e}"),
            })?;
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        Ok(())
    } else if cfg!(target_os = "linux") {
        let mut child = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn().map_err(|e| DesktopError::InputFailed {
                action: "clipboard".into(), reason: format!("xclip: {e}"),
            })?;
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        Ok(())
    } else {
        Err(DesktopError::UnsupportedPlatform("clipboard".into()))
    }
}

// ── Helpers ──

fn run_cliclick(args: &str) -> Result<(), DesktopError> {
    crate::deps::ensure_command("cliclick");
    let output = Command::new("cliclick").arg(args).output()
        .map_err(|e| DesktopError::InputFailed { action: "cliclick".into(), reason: format!("{e}") })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DesktopError::InputFailed { action: "cliclick".into(), reason: stderr.into() });
    }
    Ok(())
}

fn run_osascript(script: &str) -> Result<(), DesktopError> {
    let output = Command::new("osascript").arg("-e").arg(script).output()
        .map_err(|e| DesktopError::InputFailed { action: "osascript".into(), reason: format!("{e}") })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DesktopError::InputFailed { action: "osascript".into(), reason: stderr.into() });
    }
    Ok(())
}

fn run_xdotool(args: &[&str]) -> Result<(), DesktopError> {
    crate::deps::ensure_command("xdotool");
    let output = Command::new("xdotool").args(args).output()
        .map_err(|e| DesktopError::InputFailed { action: "xdotool".into(), reason: format!("{e}") })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DesktopError::InputFailed { action: "xdotool".into(), reason: stderr.into() });
    }
    Ok(())
}

fn macos_key_code(key: &str) -> &str {
    match key.to_lowercase().as_str() {
        "enter" | "return" => "36", "tab" => "48", "escape" | "esc" => "53",
        "space" => "49", "backspace" | "delete" => "51",
        "up" => "126", "down" => "125", "left" => "123", "right" => "124",
        "a" => "0", "s" => "1", "v" => "9", "c" => "8", "z" => "6", "x" => "7",
        _ => "36",
    }
}

fn macos_modifier_code(modifier: &str) -> &str {
    match modifier.to_lowercase().as_str() {
        "shift" => "56", "cmd" | "command" => "55",
        "ctrl" | "control" => "59", "alt" | "option" => "58",
        _ => "55",
    }
}

fn xdotool_key(key: &str) -> String {
    match key.to_lowercase().as_str() {
        "enter" | "return" => "Return".into(), "tab" => "Tab".into(),
        "escape" | "esc" => "Escape".into(), "space" => "space".into(),
        "backspace" | "delete" => "BackSpace".into(),
        "up" => "Up".into(), "down" => "Down".into(),
        "left" => "Left".into(), "right" => "Right".into(),
        other => other.into(),
    }
}

fn xdotool_modifier(modifier: &str) -> String {
    match modifier.to_lowercase().as_str() {
        "cmd" | "command" => "super".into(), "ctrl" | "control" => "ctrl".into(),
        "alt" | "option" => "alt".into(), "shift" => "shift".into(),
        other => other.into(),
    }
}
