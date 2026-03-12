//! Cross-platform helpers for shell commands (open, zip, log viewer).

use std::process::Command;

/// Open a file or directory in the OS file manager / default app.
pub fn open_path(path: &str) {
    #[cfg(target_os = "macos")]
    { let _ = Command::new("open").arg(path).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = Command::new("xdg-open").arg(path).spawn(); }
    #[cfg(target_os = "windows")]
    { let _ = Command::new("cmd").args(["/C", "start", "", path]).spawn(); }
}

/// Reveal a file in the OS file manager.
pub fn reveal_in_finder(path: &str) {
    #[cfg(target_os = "macos")]
    { let _ = Command::new("open").arg("-R").arg(path).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = Command::new("xdg-open").arg(std::path::Path::new(path).parent().unwrap_or(std::path::Path::new("."))).spawn(); }
    #[cfg(target_os = "windows")]
    { let _ = Command::new("explorer").arg(format!("/select,{}", path)).spawn(); }
}

/// Open a log file in a platform-appropriate log viewer.
pub fn open_log_viewer(log_path: &str) {
    #[cfg(target_os = "macos")]
    { let _ = Command::new("open").arg("-a").arg("Console").arg(log_path).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = Command::new("xdg-open").arg(log_path).spawn(); }
    #[cfg(target_os = "windows")]
    { let _ = Command::new("notepad").arg(log_path).spawn(); }
}

/// Open a file in a text editor.
pub fn open_in_editor(path: &str) {
    #[cfg(target_os = "macos")]
    { let _ = Command::new("open").arg("-e").arg(path).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = Command::new("xdg-open").arg(path).spawn(); }
    #[cfg(target_os = "windows")]
    { let _ = Command::new("notepad").arg(path).spawn(); }
}

/// Get the user's home directory, with proper cross-platform fallback.
pub fn home_dir() -> String {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string())
}

/// Create a zip archive from a source directory. Returns Ok(()) on success.
pub fn zip_directory(source: &str, dest_zip: &str) -> Result<(), String> {
    let output = Command::new("zip")
        .args(["-r", "-q", dest_zip, source])
        .output()
        .map_err(|e| format!("zip not found: {}. Install zip or use a file archiver.", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
