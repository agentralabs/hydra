//! AppManager — launch, focus, list, and close desktop applications.
//! macOS: osascript/AppleScript. Linux: wmctrl/xdotool.

use crate::errors::DesktopError;
use serde::{Deserialize, Serialize};

/// Information about an open window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app_name: String,
    pub is_focused: bool,
}

/// Manages desktop applications.
pub struct AppManager;

impl AppManager {
    /// Launch an application by name.
    pub fn launch(app_name: &str) -> Result<(), DesktopError> {
        eprintln!("hydra-desktop: launching '{app_name}'");
        if cfg!(target_os = "macos") {
            Self::run_cmd("open", &["-a", app_name])
        } else if cfg!(target_os = "linux") {
            // Try common launchers
            Self::run_cmd(app_name, &[]).or_else(|_| {
                Self::run_cmd("xdg-open", &[app_name])
            })
        } else {
            Err(DesktopError::UnsupportedPlatform("launch".into()))
        }
    }

    /// Bring a window to front by title (partial match).
    pub fn focus(title: &str) -> Result<(), DesktopError> {
        eprintln!("hydra-desktop: focusing window '{title}'");
        if cfg!(target_os = "macos") {
            let script = format!(
                r#"tell application "System Events"
                    set procs to every process whose visible is true
                    repeat with p in procs
                        set wins to every window of p
                        repeat with w in wins
                            if name of w contains "{title}" then
                                set frontmost of p to true
                                return
                            end if
                        end repeat
                    end repeat
                end tell"#
            );
            Self::run_osascript(&script)
        } else if cfg!(target_os = "linux") {
            Self::run_cmd("wmctrl", &["-a", title])
        } else {
            Err(DesktopError::UnsupportedPlatform("focus".into()))
        }
    }

    /// List all open windows.
    pub fn list_windows() -> Result<Vec<WindowInfo>, DesktopError> {
        if cfg!(target_os = "macos") {
            Self::list_windows_macos()
        } else if cfg!(target_os = "linux") {
            Self::list_windows_linux()
        } else {
            Err(DesktopError::UnsupportedPlatform("list_windows".into()))
        }
    }

    /// Close a window by title.
    pub fn close(title: &str) -> Result<(), DesktopError> {
        eprintln!("hydra-desktop: closing window '{title}'");
        if cfg!(target_os = "macos") {
            let script = format!(
                r#"tell application "System Events"
                    set procs to every process whose visible is true
                    repeat with p in procs
                        set wins to every window of p
                        repeat with w in wins
                            if name of w contains "{title}" then
                                click button 1 of w
                                return
                            end if
                        end repeat
                    end repeat
                end tell"#
            );
            Self::run_osascript(&script)
        } else if cfg!(target_os = "linux") {
            Self::run_cmd("wmctrl", &["-c", title])
        } else {
            Err(DesktopError::UnsupportedPlatform("close".into()))
        }
    }

    /// Check if an application is running.
    pub fn is_running(app_name: &str) -> bool {
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            let output = std::process::Command::new("pgrep")
                .arg("-i")
                .arg(app_name)
                .output();
            matches!(output, Ok(o) if o.status.success())
        } else {
            false
        }
    }

    fn list_windows_macos() -> Result<Vec<WindowInfo>, DesktopError> {
        let script = r#"
            set output to ""
            tell application "System Events"
                set procs to every process whose visible is true
                repeat with p in procs
                    set pname to name of p
                    set wins to every window of p
                    repeat with w in wins
                        set wname to name of w
                        set output to output & pname & "|||" & wname & linefeed
                    end repeat
                end repeat
            end tell
            return output
        "#;

        let output = std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| DesktopError::AppError {
                app: "list_windows".into(),
                reason: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let windows: Vec<WindowInfo> = stdout
            .lines()
            .filter(|l| l.contains("|||"))
            .enumerate()
            .map(|(i, line)| {
                let parts: Vec<&str> = line.splitn(2, "|||").collect();
                WindowInfo {
                    id: i.to_string(),
                    app_name: parts.first().unwrap_or(&"").to_string(),
                    title: parts.get(1).unwrap_or(&"").to_string(),
                    is_focused: i == 0,
                }
            })
            .collect();

        Ok(windows)
    }

    fn list_windows_linux() -> Result<Vec<WindowInfo>, DesktopError> {
        let output = std::process::Command::new("wmctrl")
            .arg("-l")
            .output()
            .map_err(|e| DesktopError::AppError {
                app: "wmctrl".into(),
                reason: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let windows: Vec<WindowInfo> = stdout
            .lines()
            .enumerate()
            .filter_map(|(i, line)| {
                let parts: Vec<&str> = line.splitn(4, ' ').collect();
                if parts.len() >= 4 {
                    Some(WindowInfo {
                        id: parts[0].to_string(),
                        app_name: parts.get(2).unwrap_or(&"").to_string(),
                        title: parts[3..].join(" "),
                        is_focused: i == 0,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(windows)
    }

    fn run_osascript(script: &str) -> Result<(), DesktopError> {
        let output = std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| DesktopError::AppError {
                app: "osascript".into(),
                reason: e.to_string(),
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DesktopError::AppError {
                app: "osascript".into(),
                reason: stderr.to_string(),
            });
        }
        Ok(())
    }

    fn run_cmd(cmd: &str, args: &[&str]) -> Result<(), DesktopError> {
        let output = std::process::Command::new(cmd)
            .args(args)
            .output()
            .map_err(|e| DesktopError::AppError {
                app: cmd.into(),
                reason: e.to_string(),
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DesktopError::AppError {
                app: cmd.into(),
                reason: stderr.to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_info_serialization() {
        let info = WindowInfo {
            id: "1".into(),
            title: "Terminal".into(),
            app_name: "iTerm2".into(),
            is_focused: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: WindowInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, "Terminal");
    }

    #[test]
    fn is_running_nonexistent() {
        assert!(!AppManager::is_running("hydra_nonexistent_app_xyz_12345"));
    }
}
