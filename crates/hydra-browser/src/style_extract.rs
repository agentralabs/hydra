//! O13 Style extraction — extract CSS styling information from web page HTML.
//! Used for style transfer: "Design it like Stripe" → extract Stripe's visual DNA.
//! EC-13.3: Extracts from whatever HTML is provided (caller sends homepage).

use std::collections::HashSet;

/// Extracted style profile from a web page.
#[derive(Debug, Clone, Default)]
pub struct StyleProfile {
    pub colors: Vec<String>,
    pub fonts: Vec<String>,
    pub font_sizes: Vec<String>,
    pub has_dark_theme: bool,
}

/// Extract style information from HTML (inline styles + style tags).
pub fn extract_from_html(html: &str) -> StyleProfile {
    let mut colors = HashSet::new();
    let mut fonts = HashSet::new();
    let mut font_sizes = HashSet::new();
    // Extract hex colors
    let mut i = 0;
    let bytes = html.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'#' && i + 6 < bytes.len() {
            let hex = &html[i + 1..i + 7];
            if hex.chars().all(|c| c.is_ascii_hexdigit()) {
                colors.insert(format!("#{hex}"));
                i += 7;
                continue;
            }
        }
        i += 1;
    }
    // Extract font-family values
    for segment in html.split("font-family") {
        if let Some(colon) = segment.find(':') {
            let after = &segment[colon + 1..];
            if let Some(end) = after.find(|c: char| c == ';' || c == '"' || c == '}') {
                let font = after[..end].trim().trim_matches('\'').trim_matches('"');
                if !font.is_empty() && font.len() < 60 {
                    fonts.insert(font.split(',').next().unwrap_or(font).trim().to_string());
                }
            }
        }
    }
    // Extract font-size values
    for segment in html.split("font-size") {
        if let Some(colon) = segment.find(':') {
            let after = &segment[colon + 1..];
            if let Some(end) = after.find(|c: char| c == ';' || c == '"' || c == '}') {
                let size = after[..end].trim();
                if size.contains("px") || size.contains("rem") || size.contains("em") {
                    font_sizes.insert(size.to_string());
                }
            }
        }
    }
    let has_dark = html.contains("dark") && (html.contains("#000") || html.contains("#111")
        || html.contains("#1a1a") || html.contains("background-color: #0") || html.contains("bg-dark"));
    StyleProfile {
        colors: colors.into_iter().collect(),
        fonts: fonts.into_iter().collect(),
        font_sizes: font_sizes.into_iter().collect(),
        has_dark_theme: has_dark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_hex_colors() {
        let html = r#"<div style="color: #ff5733; background: #2d3436;">test</div>"#;
        let profile = extract_from_html(html);
        assert!(profile.colors.len() >= 2);
    }

    #[test]
    fn extract_font_family() {
        let html = r#"<style>body { font-family: 'Inter', sans-serif; }</style>"#;
        let profile = extract_from_html(html);
        assert!(profile.fonts.iter().any(|f| f.contains("Inter")));
    }

    #[test]
    fn detect_dark_theme() {
        let html = r#"<body class="dark"><div style="background-color: #000000;">dark mode</div></body>"#;
        let profile = extract_from_html(html);
        assert!(profile.has_dark_theme);
    }

    #[test]
    fn empty_html_returns_empty_profile() {
        let profile = extract_from_html("");
        assert!(profile.colors.is_empty());
        assert!(profile.fonts.is_empty());
    }
}
