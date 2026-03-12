//! Typography, spacing, and radius tokens for the Hydra design system.

/// Font size tokens.
pub struct Typography;

impl Typography {
    pub const HERO: u32 = 28;
    pub const TITLE: u32 = 20;
    pub const HEADLINE: u32 = 20;
    pub const SUBTITLE: u32 = 16;
    pub const BODY: u32 = 14;
    pub const SECONDARY: u32 = 14;
    pub const CAPTION: u32 = 13;
    pub const FONT_FAMILY: &'static str =
        "'Geist', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";
    pub const FONT_MONO: &'static str =
        "'JetBrains Mono', 'SF Mono', 'Fira Code', monospace";
}

/// Spacing tokens (in pixels).
pub struct Spacing;

impl Spacing {
    pub const TIGHT: u32 = 4;
    pub const NORMAL: u32 = 8;
    pub const RELAXED: u32 = 12;
    pub const SPACIOUS: u32 = 16;
    pub const GENEROUS: u32 = 24;
    pub const WIDE: u32 = 32;
    pub const EXTRA_WIDE: u32 = 48;
}

/// Border-radius tokens (in pixels).
pub struct Radius;

impl Radius {
    pub const SMALL: u32 = 6;
    pub const MEDIUM: u32 = 10;
    pub const LARGE: u32 = 16;
    pub const PILL: u32 = 9999;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typography_sizes_ascending() {
        assert!(Typography::CAPTION <= Typography::BODY);
        assert!(Typography::BODY <= Typography::SUBTITLE);
        assert!(Typography::SUBTITLE <= Typography::HEADLINE);
        assert!(Typography::HEADLINE <= Typography::HERO);
    }

    #[test]
    fn test_font_family_not_empty() {
        assert!(!Typography::FONT_FAMILY.is_empty());
        assert!(Typography::FONT_FAMILY.contains("sans-serif"));
    }

    #[test]
    fn test_spacing_ascending() {
        assert!(Spacing::TIGHT < Spacing::NORMAL);
        assert!(Spacing::NORMAL < Spacing::RELAXED);
        assert!(Spacing::RELAXED < Spacing::SPACIOUS);
        assert!(Spacing::SPACIOUS < Spacing::GENEROUS);
        assert!(Spacing::GENEROUS < Spacing::WIDE);
        assert!(Spacing::WIDE < Spacing::EXTRA_WIDE);
    }

    #[test]
    fn test_radius_ascending() {
        assert!(Radius::SMALL < Radius::MEDIUM);
        assert!(Radius::MEDIUM < Radius::LARGE);
        assert!(Radius::LARGE < Radius::PILL);
    }

    #[test]
    fn test_pill_radius_very_large() {
        assert!(Radius::PILL > 1000);
    }
}
