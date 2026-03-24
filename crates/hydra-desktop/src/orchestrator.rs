//! WindowOrchestrator — manages multiple windows and tabs simultaneously.

use crate::app::{AppManager, WindowInfo};
use crate::errors::DesktopError;
use serde::{Deserialize, Serialize};

/// Layout for window tiling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TileLayout {
    SideBySide,
    Stacked,
    Grid,
    Focus,
}

/// A monitored window with its last screenshot hash.
#[derive(Debug, Clone)]
pub struct MonitoredWindow {
    pub info: WindowInfo,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub has_notification: bool,
}

/// Orchestrates multiple windows for parallel task execution.
pub struct WindowOrchestrator {
    monitored: Vec<MonitoredWindow>,
}

impl WindowOrchestrator {
    pub fn new() -> Self {
        Self {
            monitored: Vec::new(),
        }
    }

    /// Start monitoring a window by title.
    pub fn monitor(&mut self, title: &str) -> Result<(), DesktopError> {
        let windows = AppManager::list_windows()?;
        let found = windows.into_iter().find(|w| {
            w.title.to_lowercase().contains(&title.to_lowercase())
        });

        if let Some(info) = found {
            self.monitored.push(MonitoredWindow {
                info,
                last_check: chrono::Utc::now(),
                has_notification: false,
            });
            eprintln!("hydra-desktop: monitoring window '{title}'");
            Ok(())
        } else {
            Err(DesktopError::WindowNotFound(title.into()))
        }
    }

    /// Stop monitoring a window.
    pub fn unmonitor(&mut self, title: &str) {
        self.monitored.retain(|w| {
            !w.info.title.to_lowercase().contains(&title.to_lowercase())
        });
    }

    /// Get list of monitored windows.
    pub fn monitored_windows(&self) -> &[MonitoredWindow] {
        &self.monitored
    }

    /// Check all monitored windows for notifications/changes.
    pub fn check_notifications(&mut self) -> Vec<String> {
        let notifications = Vec::new();
        for window in &mut self.monitored {
            // Simple heuristic: if window title changed, it may have a notification
            window.last_check = chrono::Utc::now();
            // In a real implementation, we'd screenshot and compare
        }
        notifications
    }

    /// Tile windows in a layout.
    pub fn tile(&self, layout: &TileLayout) -> Result<(), DesktopError> {
        let count = self.monitored.len();
        if count == 0 {
            return Ok(());
        }

        eprintln!("hydra-desktop: tiling {} windows ({:?})", count, layout);

        // Get screen dimensions (approximate)
        let screen_width = 1920u32;
        let screen_height = 1080u32;

        for (i, window) in self.monitored.iter().enumerate() {
            let (x, y, w, h) = match layout {
                TileLayout::SideBySide => {
                    let w = screen_width / count as u32;
                    (w * i as u32, 0, w, screen_height)
                }
                TileLayout::Stacked => {
                    let h = screen_height / count as u32;
                    (0, h * i as u32, screen_width, h)
                }
                TileLayout::Grid => {
                    let cols = (count as f64).sqrt().ceil() as u32;
                    let rows = (count as u32).div_ceil(cols);
                    let w = screen_width / cols;
                    let h = screen_height / rows;
                    let col = i as u32 % cols;
                    let row = i as u32 / cols;
                    (col * w, row * h, w, h)
                }
                TileLayout::Focus => {
                    if i == 0 {
                        (0, 0, screen_width, screen_height)
                    } else {
                        continue;
                    }
                }
            };

            // Apply via platform-specific window management
            if cfg!(target_os = "linux") {
                let _ = std::process::Command::new("wmctrl")
                    .args([
                        "-r",
                        &window.info.title,
                        "-e",
                        &format!("0,{x},{y},{w},{h}"),
                    ])
                    .output();
            }
            // macOS window management requires accessibility permissions
            // and is more complex — left as best-effort
        }

        Ok(())
    }
}

impl Default for WindowOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_orchestrator_empty() {
        let orch = WindowOrchestrator::new();
        assert!(orch.monitored_windows().is_empty());
    }

    #[test]
    fn tile_layout_serialization() {
        let layout = TileLayout::SideBySide;
        let json = serde_json::to_string(&layout).unwrap();
        let back: TileLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TileLayout::SideBySide);
    }
}
