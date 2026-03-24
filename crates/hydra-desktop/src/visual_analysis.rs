//! O13 Visual analysis — extract color and composition metrics from screenshots.
//! EC-13.1: Uses region-based sampling (not pixel-level) for gradient/anti-alias resilience.

use crate::errors::DesktopError;
use std::collections::HashMap;

/// Visual metrics extracted from a screenshot.
#[derive(Debug, Clone)]
pub struct VisualMetrics {
    pub dominant_colors: Vec<(u8, u8, u8)>,
    pub color_count: usize,
    pub brightness: f64,
}

/// Analyze a PNG screenshot for visual metrics.
/// Samples from a grid of regions (EC-13.1) rather than every pixel.
pub fn analyze_screenshot(png_bytes: &[u8]) -> Result<VisualMetrics, DesktopError> {
    let img = image::load_from_memory(png_bytes)
        .map_err(|e| DesktopError::ScreenshotFailed(format!("image decode: {e}")))?;
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    // Sample a 10x10 grid of regions (EC-13.1: region-based, not pixel-level)
    let grid = 10u32;
    let step_x = (w / grid).max(1);
    let step_y = (h / grid).max(1);
    let mut color_counts: HashMap<(u8, u8, u8), u32> = HashMap::new();
    let mut total_brightness = 0.0f64;
    let mut samples = 0u32;
    for gx in 0..grid {
        for gy in 0..grid {
            let px = (gx * step_x + step_x / 2).min(w - 1);
            let py = (gy * step_y + step_y / 2).min(h - 1);
            let pixel = rgb.get_pixel(px, py);
            // Quantize to 32-level buckets for clustering
            let r = (pixel[0] / 8) * 8;
            let g = (pixel[1] / 8) * 8;
            let b = (pixel[2] / 8) * 8;
            *color_counts.entry((r, g, b)).or_insert(0) += 1;
            total_brightness += (pixel[0] as f64 * 0.299 + pixel[1] as f64 * 0.587 + pixel[2] as f64 * 0.114) / 255.0;
            samples += 1;
        }
    }
    let mut sorted: Vec<_> = color_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let dominant: Vec<(u8, u8, u8)> = sorted.iter().take(5).map(|(c, _)| *c).collect();
    let brightness = if samples > 0 { total_brightness / samples as f64 } else { 0.5 };
    Ok(VisualMetrics { dominant_colors: dominant, color_count: sorted.len(), brightness })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_solid_color_image() {
        // Create a 10x10 red PNG in memory
        let mut img = image::RgbImage::new(10, 10);
        for pixel in img.pixels_mut() { *pixel = image::Rgb([255, 0, 0]); }
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let metrics = analyze_screenshot(buf.get_ref()).unwrap();
        assert_eq!(metrics.dominant_colors.len(), 1); // Single color
        assert!(metrics.brightness < 0.4); // Red is dark in luminance
    }

    #[test]
    fn invalid_bytes_returns_error() {
        assert!(analyze_screenshot(b"not a png").is_err());
    }
}
