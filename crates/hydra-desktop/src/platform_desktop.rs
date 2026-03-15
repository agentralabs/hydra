//! Desktop platform features — notifications, global hotkey.
//! These are desktop-only capabilities not available in TUI or VS Code.

/// Show an OS notification with title and body.
#[cfg(feature = "notifications")]
pub fn show_notification(title: &str, body: &str) {
    use notify_rust::Notification;
    if let Err(e) = Notification::new()
        .appname("Hydra")
        .summary(title)
        .body(body)
        .timeout(5000)
        .show()
    {
        eprintln!("[hydra-desktop] Notification failed: {}", e);
    }
}

#[cfg(not(feature = "notifications"))]
pub fn show_notification(title: &str, body: &str) {
    eprintln!("[hydra-desktop] Notification (no-op): {} — {}", title, body);
}

/// Show a notification for urgent awareness alerts.
pub fn notify_urgent(title: &str, detail: &str) {
    show_notification(&format!("⚠ {}", title), detail);
}

/// Show a notification for morning briefing.
pub fn notify_briefing(item_count: usize) {
    if item_count > 0 {
        show_notification(
            "Hydra Morning Briefing",
            &format!("{} items since your last session", item_count),
        );
    }
}

/// Register a global hotkey (Cmd+Shift+H on macOS, Ctrl+Shift+H elsewhere).
/// Returns true if registered successfully.
#[cfg(feature = "hotkey")]
pub fn register_global_hotkey() -> bool {
    use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};

    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[hydra-desktop] Hotkey manager failed: {}", e);
            return false;
        }
    };

    let hotkey = HotKey::new(
        Some(Modifiers::SUPER | Modifiers::SHIFT),
        Code::KeyH,
    );

    match manager.register(hotkey) {
        Ok(_) => {
            eprintln!("[hydra-desktop] Global hotkey registered: Cmd+Shift+H");
            // Leak the manager so it stays alive (it deregisters on drop)
            std::mem::forget(manager);
            true
        }
        Err(e) => {
            eprintln!("[hydra-desktop] Hotkey registration failed: {}", e);
            false
        }
    }
}

#[cfg(not(feature = "hotkey"))]
pub fn register_global_hotkey() -> bool {
    eprintln!("[hydra-desktop] Global hotkey not available (feature disabled)");
    false
}
