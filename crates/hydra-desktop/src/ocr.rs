//! Lightweight OCR — extract text regions with coordinates from screenshots.
//! macOS: Uses Vision framework via swift CLI. Fallback: tesseract.
//! Zero LLM tokens. Tier 2 of the vision cascade.

use crate::errors::DesktopError;

/// A region of text detected by OCR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OcrRegion {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub confidence: f64,
}

/// Run OCR on a screenshot PNG. Returns detected text regions.
pub fn ocr_screenshot(image_path: &str) -> Result<Vec<OcrRegion>, DesktopError> {
    if cfg!(target_os = "macos") {
        ocr_macos(image_path)
    } else {
        ocr_tesseract(image_path)
    }
}

/// Run OCR on the current screen (captures screenshot first).
pub fn ocr_current_screen() -> Result<Vec<OcrRegion>, DesktopError> {
    let (bytes, _info) = crate::screen::ScreenCapture::capture_full()?;
    ocr_from_bytes(&bytes)
}

/// Run OCR on pre-captured screenshot bytes (avoids redundant re-capture).
pub fn ocr_from_bytes(bytes: &[u8]) -> Result<Vec<OcrRegion>, DesktopError> {
    let tmp = format!("/tmp/hydra_ocr_{}.png", uuid::Uuid::new_v4());
    std::fs::write(&tmp, bytes)
        .map_err(|e| DesktopError::CaptureFailed(format!("Write OCR temp: {e}")))?;
    let result = ocr_screenshot(&tmp);
    let _ = std::fs::remove_file(&tmp);
    result
}

/// Find the best OCR match for a text query. Fuzzy matching.
pub fn find_best_match<'a>(query: &str, regions: &'a [OcrRegion]) -> Option<&'a OcrRegion> {
    let lower = query.to_lowercase();
    // Exact substring match first
    if let Some(r) = regions.iter().find(|r| r.text.to_lowercase().contains(&lower)) {
        return Some(r);
    }
    // Fuzzy: find closest match by word overlap
    regions.iter()
        .filter(|r| {
            let r_lower = r.text.to_lowercase();
            lower.split_whitespace().any(|w| r_lower.contains(w))
        })
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
}

/// macOS OCR using screencapture + simple text extraction heuristic.
fn ocr_macos(image_path: &str) -> Result<Vec<OcrRegion>, DesktopError> {
    // Try tesseract first (more reliable bounding boxes)
    if let Ok(regions) = ocr_tesseract(image_path) {
        if !regions.is_empty() { return Ok(regions); }
    }
    // EC-2.6: Dark mode retry — invert image and try again
    let inverted = format!("{}.inv.png", image_path);
    let inv_ok = std::process::Command::new("convert")
        .args([image_path, "-negate", &inverted]).output().map(|o| o.status.success()).unwrap_or(false);
    if inv_ok {
        if let Ok(regions) = ocr_tesseract(&inverted) {
            let _ = std::fs::remove_file(&inverted);
            if !regions.is_empty() { eprintln!("hydra-ocr: dark mode detected, inverted"); return Ok(regions); }
        }
        let _ = std::fs::remove_file(&inverted);
    }
    eprintln!("hydra-ocr: no OCR engine available, cascading to vision");
    Ok(Vec::new())
}

/// OCR via tesseract CLI (cross-platform). Auto-installs if missing.
fn ocr_tesseract(image_path: &str) -> Result<Vec<OcrRegion>, DesktopError> {
    // Self-sufficiency: ensure tesseract is installed
    crate::deps::ensure_command("tesseract");
    let output = std::process::Command::new("tesseract")
        .args([image_path, "stdout", "--tsv"])
        .output()
        .map_err(|e| DesktopError::CaptureFailed(format!("tesseract: {e}")))?;

    if !output.status.success() {
        return Err(DesktopError::CaptureFailed("tesseract failed".into()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let regions = parse_tesseract_tsv(&stdout);
    eprintln!("hydra-ocr: {} text regions via tesseract", regions.len());
    Ok(regions)
}

fn parse_tesseract_tsv(tsv: &str) -> Vec<OcrRegion> {
    let mut regions = Vec::new();
    for line in tsv.lines().skip(1) { // skip header
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 12 {
            let conf: f64 = cols[10].parse().unwrap_or(-1.0);
            let text = cols[11].trim().to_string();
            if conf > 30.0 && !text.is_empty() && text.len() > 1 {
                let x: f64 = cols[6].parse().unwrap_or(0.0);
                let y: f64 = cols[7].parse().unwrap_or(0.0);
                let w: f64 = cols[8].parse().unwrap_or(0.0);
                let h: f64 = cols[9].parse().unwrap_or(0.0);
                regions.push(OcrRegion { text, x, y, width: w, height: h, confidence: conf / 100.0 });
            }
        }
    }
    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tesseract_tsv_basic() {
        let tsv = "level\tpage\tblock\tpar\tline\tword\tleft\ttop\twidth\theight\tconf\ttext\n\
                   5\t1\t1\t1\t1\t1\t100\t200\t80\t30\t95.5\tSubmit\n\
                   5\t1\t1\t1\t1\t2\t200\t200\t80\t30\t88.0\tCancel";
        let regions = parse_tesseract_tsv(tsv);
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].text, "Submit");
        assert!((regions[0].x - 100.0).abs() < 0.1);
        assert!(regions[0].confidence > 0.9);
    }

    #[test]
    fn find_best_match_exact() {
        let regions = vec![
            OcrRegion { text: "Submit".into(), x: 100.0, y: 200.0, width: 80.0, height: 30.0, confidence: 0.95 },
            OcrRegion { text: "Cancel".into(), x: 200.0, y: 200.0, width: 80.0, height: 30.0, confidence: 0.90 },
        ];
        let found = find_best_match("Submit", &regions);
        assert!(found.is_some());
        assert_eq!(found.unwrap().text, "Submit");
    }

    #[test]
    fn find_best_match_fuzzy() {
        let regions = vec![
            OcrRegion { text: "Submit Form".into(), x: 100.0, y: 200.0, width: 80.0, height: 30.0, confidence: 0.95 },
        ];
        let found = find_best_match("submit", &regions);
        assert!(found.is_some());
    }

    #[test]
    fn find_best_match_none() {
        let regions = vec![
            OcrRegion { text: "Cancel".into(), x: 200.0, y: 200.0, width: 80.0, height: 30.0, confidence: 0.90 },
        ];
        assert!(find_best_match("Submit", &regions).is_none());
    }
}
