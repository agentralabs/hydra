//! Undo/redo wiring and toast notification state for the native UI.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Duration in milliseconds before an undo toast auto-expires.
const TOAST_EXPIRY_MS: u64 = 5_000;

/// Returns the current time as unix-epoch milliseconds.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// A transient toast notification offering an undo action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoToast {
    pub visible: bool,
    pub description: String,
    pub expires_at: Option<u64>,
}

impl UndoToast {
    /// Create a visible toast that expires in 5 seconds.
    pub fn show(description: &str) -> Self {
        Self {
            visible: true,
            description: description.to_owned(),
            expires_at: Some(now_millis() + TOAST_EXPIRY_MS),
        }
    }

    /// Create a visible toast with a specific expiry timestamp (for testing).
    pub fn show_with_expiry(description: &str, expires_at: u64) -> Self {
        Self {
            visible: true,
            description: description.to_owned(),
            expires_at: Some(expires_at),
        }
    }

    /// Hide the toast.
    pub fn dismiss(&mut self) {
        self.visible = false;
    }

    /// Whether the toast has passed its expiry time.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expiry) => now_millis() >= expiry,
            None => false,
        }
    }

    /// Check expiry against a provided timestamp (for deterministic testing).
    pub fn is_expired_at(&self, now: u64) -> bool {
        match self.expires_at {
            Some(expiry) => now >= expiry,
            None => false,
        }
    }
}

impl Default for UndoToast {
    fn default() -> Self {
        Self {
            visible: false,
            description: String::new(),
            expires_at: None,
        }
    }
}

/// Wiring state that connects undo/redo actions to the UI toast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoWiring {
    pub toast: UndoToast,
    pub can_undo: bool,
    pub can_redo: bool,
    pub last_action: Option<String>,
}

impl UndoWiring {
    /// Create default wiring with no pending actions.
    pub fn new() -> Self {
        Self {
            toast: UndoToast::default(),
            can_undo: false,
            can_redo: false,
            last_action: None,
        }
    }

    /// Record a file action: shows the undo toast and enables undo.
    pub fn on_file_action(&mut self, description: &str) {
        self.toast = UndoToast::show(description);
        self.can_undo = true;
        self.can_redo = false;
        self.last_action = Some(description.to_owned());
    }

    /// Perform an undo: hides the toast and swaps undo/redo availability.
    pub fn on_undo(&mut self) {
        self.toast.dismiss();
        self.can_undo = false;
        self.can_redo = true;
    }

    /// Perform a redo: hides the toast and swaps back.
    pub fn on_redo(&mut self) {
        self.toast.dismiss();
        self.can_undo = true;
        self.can_redo = false;
    }
}

impl Default for UndoWiring {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_show_is_visible() {
        let toast = UndoToast::show("Deleted file.rs");
        assert!(toast.visible);
        assert_eq!(toast.description, "Deleted file.rs");
        assert!(toast.expires_at.is_some());
    }

    #[test]
    fn test_toast_dismiss() {
        let mut toast = UndoToast::show("test");
        assert!(toast.visible);
        toast.dismiss();
        assert!(!toast.visible);
    }

    #[test]
    fn test_toast_expiry() {
        let toast = UndoToast::show_with_expiry("test", 1000);
        assert!(!toast.is_expired_at(500));
        assert!(!toast.is_expired_at(999));
        assert!(toast.is_expired_at(1000));
        assert!(toast.is_expired_at(2000));
    }

    #[test]
    fn test_toast_no_expiry_never_expires() {
        let toast = UndoToast::default();
        assert!(!toast.is_expired_at(u64::MAX));
    }

    #[test]
    fn test_wiring_on_file_action() {
        let mut wiring = UndoWiring::new();
        assert!(!wiring.can_undo);
        assert!(!wiring.can_redo);

        wiring.on_file_action("Renamed config.toml");
        assert!(wiring.can_undo);
        assert!(!wiring.can_redo);
        assert!(wiring.toast.visible);
        assert_eq!(wiring.last_action.as_deref(), Some("Renamed config.toml"));
    }

    #[test]
    fn test_wiring_undo_swaps_flags() {
        let mut wiring = UndoWiring::new();
        wiring.on_file_action("Delete main.rs");
        wiring.on_undo();
        assert!(!wiring.can_undo);
        assert!(wiring.can_redo);
        assert!(!wiring.toast.visible);
    }

    #[test]
    fn test_wiring_redo_swaps_back() {
        let mut wiring = UndoWiring::new();
        wiring.on_file_action("Delete main.rs");
        wiring.on_undo();
        wiring.on_redo();
        assert!(wiring.can_undo);
        assert!(!wiring.can_redo);
        assert!(!wiring.toast.visible);
    }

    #[test]
    fn test_wiring_new_action_resets_redo() {
        let mut wiring = UndoWiring::new();
        wiring.on_file_action("action 1");
        wiring.on_undo();
        assert!(wiring.can_redo);
        // New action should reset redo
        wiring.on_file_action("action 2");
        assert!(!wiring.can_redo);
        assert!(wiring.can_undo);
        assert_eq!(wiring.last_action.as_deref(), Some("action 2"));
    }
}
