//! Companion mode — small floating window.

use serde::{Deserialize, Serialize};

use crate::app::WindowConfig;

/// Small floating companion window for quick interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionMode {
    pub width: u32,
    pub height: u32,
    pub always_on_top: bool,
    pub draggable: bool,
}

impl CompanionMode {
    pub fn new() -> Self {
        Self {
            width: 350,
            height: 500,
            always_on_top: true,
            draggable: true,
        }
    }

    /// Window configuration for the companion floating window.
    pub fn window_config() -> WindowConfig {
        let mode = Self::new();
        WindowConfig {
            title: "Hydra Companion".into(),
            width: mode.width,
            height: mode.height,
            min_width: 300,
            min_height: 400,
            resizable: false,
            decorations: false,
            transparent: true,
        }
    }
}

impl Default for CompanionMode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_companion_defaults() {
        let m = CompanionMode::new();
        assert_eq!(m.width, 350);
        assert_eq!(m.height, 500);
        assert!(m.always_on_top);
        assert!(m.draggable);
    }

    #[test]
    fn test_window_config() {
        let wc = CompanionMode::window_config();
        assert_eq!(wc.title, "Hydra Companion");
        assert_eq!(wc.width, 350);
        assert_eq!(wc.height, 500);
        assert!(!wc.resizable);
        assert!(!wc.decorations);
        assert!(wc.transparent);
    }
}
