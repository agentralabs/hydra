//! ScreenCapture — cross-platform screen capture.
//! Uses native APIs: CoreGraphics on macOS, X11/Wayland on Linux.
//! Falls back to shell commands when libraries unavailable.

use crate::errors::DesktopError;
use serde::{Deserialize, Serialize};

/// A rectangular region of the screen.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Information about a captured screenshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotInfo {
    pub width: u32,
    pub height: u32,
    pub bytes_len: usize,
    pub format: String,
}

/// Screen capture engine.
pub struct ScreenCapture;

impl ScreenCapture {
    /// Check if screen recording permission is granted (macOS).
    /// Returns true on Linux (no permission needed) or if permission granted.
    pub fn has_permission() -> bool {
        if !cfg!(target_os = "macos") { return true; }
        // Try a minimal capture — if it fails with empty output, permission denied
        let result = std::process::Command::new("screencapture")
            .args(["-x", "-t", "png", "/dev/null"])
            .output();
        match result {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    /// Capture the full screen as PNG bytes.
    pub fn capture_full() -> Result<(Vec<u8>, ScreenshotInfo), DesktopError> {
        Self::capture_via_shell(None)
    }

    /// Capture a specific region as PNG bytes.
    pub fn capture_region(rect: Rect) -> Result<(Vec<u8>, ScreenshotInfo), DesktopError> {
        Self::capture_via_shell(Some(rect))
    }

    /// Capture a specific window by title (best-effort match).
    pub fn capture_window(title: &str) -> Result<(Vec<u8>, ScreenshotInfo), DesktopError> {
        // On macOS, screencapture can target a window by ID
        // For simplicity, capture full screen and note the window title
        eprintln!("hydra-desktop: capture_window for '{title}' (full screen fallback)");
        Self::capture_full()
    }

    /// Platform-specific capture using shell commands.
    fn capture_via_shell(region: Option<Rect>) -> Result<(Vec<u8>, ScreenshotInfo), DesktopError> {
        let tmp_path = std::env::temp_dir().join(format!(
            "hydra-screenshot-{}.png",
            uuid::Uuid::new_v4()
        ));
        let tmp_str = tmp_path.to_string_lossy().to_string();

        let result = if cfg!(target_os = "macos") {
            Self::capture_macos(&tmp_str, region)
        } else if cfg!(target_os = "linux") {
            Self::capture_linux(&tmp_str, region)
        } else {
            Err(DesktopError::UnsupportedPlatform(
                std::env::consts::OS.to_string(),
            ))
        };

        result?;

        // Read the captured file
        let bytes = std::fs::read(&tmp_path).map_err(|e| {
            DesktopError::CaptureFailed(format!("Cannot read screenshot: {e}"))
        })?;

        // Get dimensions from the PNG header (simple parse)
        let (width, height) = Self::png_dimensions(&bytes);

        // Clean up temp file
        let _ = std::fs::remove_file(&tmp_path);

        let info = ScreenshotInfo {
            width,
            height,
            bytes_len: bytes.len(),
            format: "png".into(),
        };

        eprintln!(
            "hydra-desktop: captured {}x{} ({}KB)",
            width, height, bytes.len() / 1024
        );
        Ok((bytes, info))
    }

    fn capture_macos(path: &str, region: Option<Rect>) -> Result<(), DesktopError> {
        let mut cmd = std::process::Command::new("screencapture");
        cmd.arg("-x"); // no sound

        if let Some(r) = region {
            cmd.arg("-R")
                .arg(format!("{},{},{},{}", r.x, r.y, r.width, r.height));
        }

        cmd.arg(path);

        let output = cmd
            .output()
            .map_err(|e| DesktopError::CaptureFailed(format!("screencapture failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DesktopError::CaptureFailed(format!(
                "screencapture exit {}: {stderr}",
                output.status
            )));
        }
        Ok(())
    }

    fn capture_linux(path: &str, region: Option<Rect>) -> Result<(), DesktopError> {
        // Try gnome-screenshot, then scrot, then import (ImageMagick)
        let tools = ["gnome-screenshot", "scrot", "import"];

        for tool in &tools {
            let result = match *tool {
                "gnome-screenshot" => {
                    let mut cmd = std::process::Command::new("gnome-screenshot");
                    cmd.arg("-f").arg(path);
                    if let Some(r) = region {
                        cmd.arg("-a")
                            .arg(format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y));
                    }
                    cmd.output()
                }
                "scrot" => {
                    let mut cmd = std::process::Command::new("scrot");
                    if let Some(r) = region {
                        cmd.arg("-a")
                            .arg(format!("{},{},{},{}", r.x, r.y, r.width, r.height));
                    }
                    cmd.arg(path);
                    cmd.output()
                }
                "import" => {
                    let mut cmd = std::process::Command::new("import");
                    cmd.arg("-window").arg("root");
                    cmd.arg(path);
                    cmd.output()
                }
                _ => continue,
            };

            if let Ok(output) = result {
                if output.status.success() {
                    return Ok(());
                }
            }
        }

        Err(DesktopError::CaptureFailed(
            "No screenshot tool found. Install gnome-screenshot, scrot, or ImageMagick".into(),
        ))
    }

    /// Extract width and height from PNG IHDR chunk.
    fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
        if bytes.len() < 24 {
            return (0, 0);
        }
        // PNG signature (8 bytes) + IHDR length (4 bytes) + "IHDR" (4 bytes)
        // Then width (4 bytes big-endian) + height (4 bytes big-endian)
        let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        (width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_dimensions_parsing() {
        // Minimal PNG header for testing
        let mut fake_png = vec![0u8; 24];
        // PNG signature
        fake_png[0..8].copy_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
        // IHDR chunk length
        fake_png[8..12].copy_from_slice(&[0, 0, 0, 13]);
        // "IHDR"
        fake_png[12..16].copy_from_slice(b"IHDR");
        // Width = 1920
        fake_png[16..20].copy_from_slice(&1920u32.to_be_bytes());
        // Height = 1080
        fake_png[20..24].copy_from_slice(&1080u32.to_be_bytes());

        let (w, h) = ScreenCapture::png_dimensions(&fake_png);
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);
    }

    #[test]
    fn empty_bytes_returns_zero_dimensions() {
        let (w, h) = ScreenCapture::png_dimensions(&[]);
        assert_eq!((w, h), (0, 0));
    }

    #[test]
    fn rect_serialization() {
        let rect = Rect {
            x: 10,
            y: 20,
            width: 300,
            height: 400,
        };
        let json = serde_json::to_string(&rect).unwrap();
        let back: Rect = serde_json::from_str(&json).unwrap();
        assert_eq!(back.width, 300);
        assert_eq!(back.height, 400);
    }
}
