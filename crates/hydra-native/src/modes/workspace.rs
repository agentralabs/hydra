//! Workspace mode — full-featured main window with sidebar.

use serde::{Deserialize, Serialize};

use crate::app::WindowConfig;

/// Full workspace mode with sidebar and content area.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMode {
    pub width: u32,
    pub height: u32,
    pub sidebar_visible: bool,
    pub sidebar_width: u32,
}

impl WorkspaceMode {
    pub fn new() -> Self {
        Self {
            width: 1200,
            height: 800,
            sidebar_visible: true,
            sidebar_width: 280,
        }
    }

    /// Window configuration for workspace mode.
    pub fn window_config() -> WindowConfig {
        let mode = Self::new();
        WindowConfig {
            title: "Hydra".into(),
            width: mode.width,
            height: mode.height,
            min_width: 800,
            min_height: 600,
            resizable: true,
            decorations: true,
            transparent: false,
        }
    }

    /// Toggle sidebar visibility.
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }
}

impl Default for WorkspaceMode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_defaults() {
        let m = WorkspaceMode::new();
        assert_eq!(m.width, 1200);
        assert_eq!(m.height, 800);
        assert!(m.sidebar_visible);
        assert_eq!(m.sidebar_width, 280);
    }

    #[test]
    fn test_window_config() {
        let wc = WorkspaceMode::window_config();
        assert_eq!(wc.title, "Hydra");
        assert_eq!(wc.width, 1200);
        assert_eq!(wc.height, 800);
        assert_eq!(wc.min_width, 800);
        assert_eq!(wc.min_height, 600);
        assert!(wc.resizable);
        assert!(wc.decorations);
    }

    #[test]
    fn test_toggle_sidebar() {
        let mut m = WorkspaceMode::new();
        assert!(m.sidebar_visible);
        m.toggle_sidebar();
        assert!(!m.sidebar_visible);
        m.toggle_sidebar();
        assert!(m.sidebar_visible);
    }
}
