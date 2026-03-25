//! O19 Webcam Capture — frame grabber via subprocess (ffmpeg/imagesnap).
//! Privacy: temp file deleted immediately after reading. Zero frames stored.
//! Camera off by default — must be explicitly enabled.

use std::time::Instant;

/// Check if a webcam capture tool is available on this system.
pub fn webcam_available() -> bool {
    if cfg!(target_os = "macos") {
        cmd_exists("imagesnap") || cmd_exists("ffmpeg")
    } else {
        std::path::Path::new("/dev/video0").exists() && cmd_exists("ffmpeg")
    }
}

fn cmd_exists(name: &str) -> bool {
    std::process::Command::new("which").arg(name).output()
        .map(|o| o.status.success()).unwrap_or(false)
}

/// Capture a single frame via subprocess. Returns raw RGB bytes.
/// Temp file is always deleted — even on error.
pub fn capture_frame() -> Result<Vec<u8>, crate::errors::DesktopError> {
    let tmp = std::env::temp_dir().join(format!("hydra-cam-{}.ppm",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().subsec_nanos()));
    let tmp_str = tmp.display().to_string();

    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("imagesnap").arg("-t").arg("0.1").arg(&tmp_str).output()
    } else {
        std::process::Command::new("ffmpeg")
            .args(["-f", "v4l2", "-i", "/dev/video0", "-vframes", "1", &tmp_str, "-y"])
            .output()
    };

    let bytes = match result {
        Ok(o) if o.status.success() => {
            std::fs::read(&tmp).map_err(|e| crate::errors::DesktopError::CameraError(format!("{e}")))
        }
        Ok(o) => Err(crate::errors::DesktopError::CameraError(
            String::from_utf8_lossy(&o.stderr).chars().take(100).collect())),
        Err(e) => Err(crate::errors::DesktopError::CameraError(format!("{e}"))),
    };
    let _ = std::fs::remove_file(&tmp); // Always delete temp
    bytes
}

/// Downsampled grayscale frame digest for motion comparison.
/// Only 80x60 = 4800 bytes — minimal memory footprint.
pub struct FrameDigest {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: Instant,
}

impl FrameDigest {
    /// Create a digest from raw RGB bytes by converting to grayscale and downsampling.
    pub fn from_rgb(w: u32, h: u32, rgb: &[u8]) -> Self {
        let target_w = 80u32;
        let target_h = 60u32;
        let mut pixels = Vec::with_capacity((target_w * target_h) as usize);
        for ty in 0..target_h {
            for tx in 0..target_w {
                let sx = (tx * w / target_w) as usize;
                let sy = (ty * h / target_h) as usize;
                let idx = (sy * w as usize + sx) * 3;
                if idx + 2 < rgb.len() {
                    let gray = (rgb[idx] as u16 + rgb[idx + 1] as u16 + rgb[idx + 2] as u16) / 3;
                    pixels.push(gray as u8);
                } else {
                    pixels.push(0);
                }
            }
        }
        Self { pixels, width: target_w, height: target_h, timestamp: Instant::now() }
    }

    /// Compute motion score between two digests (0.0 = identical, 1.0 = completely different).
    pub fn motion_score(&self, other: &FrameDigest) -> f64 {
        if self.pixels.len() != other.pixels.len() || self.pixels.is_empty() { return 0.0; }
        let sum: f64 = self.pixels.iter().zip(&other.pixels)
            .map(|(a, b)| (*a as f64 - *b as f64).abs())
            .sum();
        sum / (self.pixels.len() as f64 * 255.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_identical_zero() {
        let data = vec![128u8; 320 * 240 * 3];
        let a = FrameDigest::from_rgb(320, 240, &data);
        let b = FrameDigest::from_rgb(320, 240, &data);
        assert!(a.motion_score(&b) < 0.001);
    }

    #[test]
    fn digest_different_nonzero() {
        let a_data = vec![0u8; 320 * 240 * 3];
        let b_data = vec![255u8; 320 * 240 * 3];
        let a = FrameDigest::from_rgb(320, 240, &a_data);
        let b = FrameDigest::from_rgb(320, 240, &b_data);
        assert!(a.motion_score(&b) > 0.5);
    }

    #[test]
    fn downsample_dimensions() {
        let data = vec![100u8; 320 * 240 * 3];
        let d = FrameDigest::from_rgb(320, 240, &data);
        assert_eq!(d.width, 80);
        assert_eq!(d.height, 60);
        assert_eq!(d.pixels.len(), 4800);
    }
}
