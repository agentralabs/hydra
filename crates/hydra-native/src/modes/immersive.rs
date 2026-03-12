//! Immersive mode — fullscreen, distraction-free.

use serde::{Deserialize, Serialize};

use crate::app::WindowConfig;

/// Fullscreen immersive mode for focused work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmersiveMode {
    pub fullscreen: bool,
    pub focus_mode: bool,
}

impl ImmersiveMode {
    pub fn new() -> Self {
        Self {
            fullscreen: true,
            focus_mode: true,
        }
    }

    /// Window configuration for immersive mode.
    pub fn window_config() -> WindowConfig {
        WindowConfig {
            title: "Hydra".into(),
            width: 1920,
            height: 1080,
            min_width: 800,
            min_height: 600,
            resizable: false,
            decorations: false,
            transparent: false,
        }
    }
}

impl Default for ImmersiveMode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_immersive_defaults() {
        let m = ImmersiveMode::new();
        assert!(m.fullscreen);
        assert!(m.focus_mode);
    }

    #[test]
    fn test_window_config() {
        let wc = ImmersiveMode::window_config();
        assert_eq!(wc.title, "Hydra");
        assert!(!wc.resizable);
        assert!(!wc.decorations);
    }

    #[test]
    fn test_serialization() {
        let m = ImmersiveMode::new();
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("true"));
        let back: ImmersiveMode = serde_json::from_str(&json).unwrap();
        assert!(back.fullscreen);
    }
}
