//! Design system color palette.

/// Central color palette for the Hydra design system.
pub struct DesignColors;

impl DesignColors {
    // Brand — Hydra blue
    pub const ACCENT: &'static str = "#6495ED";
    pub const ACCENT_HOVER: &'static str = "#7BA8F0";

    // Semantic
    pub const SUCCESS: &'static str = "#3DD68C";
    pub const WARNING: &'static str = "#F5A623";
    pub const ERROR: &'static str = "#EF5350";
    pub const INFO: &'static str = "#4A9EFF";
    pub const PROCESSING: &'static str = "#A78BFA";

    // Dark backgrounds
    pub const BG_0: &'static str = "#0C0C0E";
    pub const BG_1: &'static str = "#141418";
    pub const BG_2: &'static str = "#1C1C22";
    pub const BG_3: &'static str = "#24242C";

    // Dark text
    pub const TEXT_0: &'static str = "#E8E8ED";
    pub const TEXT_1: &'static str = "#8E8E9A";
    pub const TEXT_2: &'static str = "#56566A";

    // Borders
    pub const BORDER: &'static str = "rgba(255,255,255,0.06)";
    pub const BORDER_FOCUS: &'static str = "rgba(100,149,237,0.4)";

    // Legacy aliases (kept for existing code references)
    pub const TRUST_BLUE: &'static str = "#4A9EFF";
    pub const WARM_WHITE: &'static str = "#FAFAFA";
    pub const SOFT_BLACK: &'static str = "#1A1A2E";
    pub const SUCCESS_GREEN: &'static str = "#3DD68C";
    pub const ATTENTION_ORANGE: &'static str = "#F5A623";
    pub const GENTLE_RED: &'static str = "#EF5350";
    pub const CALM_PURPLE: &'static str = "#A78BFA";
    pub const BG_PRIMARY: &'static str = "#0C0C0E";
    pub const BG_SECONDARY: &'static str = "#141418";
    pub const BG_GLASS: &'static str = "rgba(255,255,255,0.04)";
}

/// Returns true if the string looks like a valid hex color (#RGB, #RRGGBB, or #RRGGBBAA).
fn is_valid_hex(s: &str) -> bool {
    if !s.starts_with('#') {
        return false;
    }
    let hex = &s[1..];
    matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Returns true if the string looks like a valid rgba(...) color.
fn is_valid_rgba(s: &str) -> bool {
    s.starts_with("rgba(") && s.ends_with(')')
}

/// Returns true if a color string is a valid hex or rgba value.
pub fn is_valid_color(s: &str) -> bool {
    is_valid_hex(s) || is_valid_rgba(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_brand_colors_valid() {
        let colors = [
            DesignColors::ACCENT,
            DesignColors::ACCENT_HOVER,
            DesignColors::SUCCESS,
            DesignColors::WARNING,
            DesignColors::ERROR,
            DesignColors::INFO,
            DesignColors::PROCESSING,
        ];
        for color in &colors {
            assert!(is_valid_hex(color), "Invalid hex color: {}", color);
        }
    }

    #[test]
    fn test_all_background_colors_valid() {
        assert!(is_valid_hex(DesignColors::BG_0));
        assert!(is_valid_hex(DesignColors::BG_1));
        assert!(is_valid_hex(DesignColors::BG_2));
        assert!(is_valid_hex(DesignColors::BG_3));
        assert!(is_valid_hex(DesignColors::TEXT_0));
        assert!(is_valid_hex(DesignColors::TEXT_1));
        assert!(is_valid_hex(DesignColors::TEXT_2));
        assert!(is_valid_rgba(DesignColors::BORDER));
        assert!(is_valid_rgba(DesignColors::BORDER_FOCUS));
        assert!(is_valid_rgba(DesignColors::BG_GLASS));
    }

    #[test]
    fn test_is_valid_color_accepts_hex_and_rgba() {
        assert!(is_valid_color("#FF0000"));
        assert!(is_valid_color("#abc"));
        assert!(is_valid_color("rgba(0,0,0,0.5)"));
        assert!(!is_valid_color("blue"));
        assert!(!is_valid_color(""));
    }
}
