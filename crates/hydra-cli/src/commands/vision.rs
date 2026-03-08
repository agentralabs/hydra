//! Vision commands — interact with the Vision sister

use crate::output;

pub fn capture(target: Option<&str>) {
    let target = target.unwrap_or("screen");
    output::print_header("Vision Capture");
    output::print_info(&format!("Capturing: {}", target));
    output::print_kv("Status", "Vision sister not connected (offline mode)");
}

pub fn compare(a: &str, b: &str) {
    output::print_header("Vision Compare");
    output::print_info(&format!("Comparing: {} vs {}", a, b));
    output::print_kv("Status", "Vision sister not connected (offline mode)");
}

pub fn ocr(image_path: &str) {
    output::print_header("Vision OCR");
    output::print_info(&format!("Extracting text from: {}", image_path));
    output::print_kv("Status", "Vision sister not connected (offline mode)");
}

pub fn stats() {
    output::print_header("Vision Stats");
    output::print_kv("Captures", "0");
    output::print_kv("Observations", "0");
    output::print_kv("Status", "Vision sister not connected (offline mode)");
}
