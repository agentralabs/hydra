//! Theme system — dark and light themes with auto-detection.
//!
//! Brand colors (12 verb colors, 7 dot colors) stay the same in both themes.
//! Only UI chrome colors change between themes.
//! Auto-detects terminal background on startup via terminal-light crate.

use ratatui::style::Color;

use crate::constants;

/// Which theme is active.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeMode {
    Dark,
    Light,
}

/// All UI chrome colors for a theme. Brand colors live in constants.rs.
#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: ThemeMode,
    // Backgrounds
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_elevated: Color,
    // Text
    pub fg_primary: Color,
    pub fg_secondary: Color,
    pub fg_muted: Color,
    pub fg_ghost: Color,
    // Semantic
    pub user_message: Color,
    pub assistant_text: Color,
    pub system_notification: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    // Chrome
    pub accent: Color,
    pub alive: Color,
    pub separator: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub label: Color,
    pub label_green: Color,
    pub greeting: Color,
    // Welcome frame borders
    pub frame_top: Color,
    pub frame_bottom: Color,
    // Identity values
    pub identity_val: Color,
    pub identity_sub: Color,
    pub lyapunov_sub: Color,
    // Brand additions
    pub model_name: Color,
    pub git_branch: Color,
    pub dim: Color,
}

impl Theme {
    /// Dark theme — Hydra brand palette from Go reference.
    pub fn dark() -> Self {
        let (br, bg, bb) = constants::HYDRA_BLUE;
        let blue = Color::Rgb(br, bg, bb);
        let (cr, cg, cb) = constants::HYDRA_CYAN;
        let cyan = Color::Rgb(cr, cg, cb);
        let (gr, gg, gb) = constants::HYDRA_GREEN;
        let green = Color::Rgb(gr, gg, gb);
        let (rr, rg, rb) = constants::HYDRA_RED;
        let (yr, yg, yb) = constants::HYDRA_YELLOW;
        let (pr, pg, pb) = constants::HYDRA_PURPLE;
        let (dr, dg, db) = constants::HYDRA_DIM;
        Self {
            mode: ThemeMode::Dark,
            bg_primary: Color::Rgb(12, 12, 12),
            bg_secondary: Color::Rgb(17, 17, 17),
            bg_elevated: Color::Rgb(30, 30, 46),
            fg_primary: Color::Rgb(220, 220, 220),
            fg_secondary: Color::Rgb(100, 100, 100),
            fg_muted: Color::Rgb(dr, dg, db),
            fg_ghost: Color::Rgb(46, 46, 46),
            user_message: blue,
            assistant_text: Color::Rgb(220, 220, 220),
            system_notification: Color::Rgb(yr, yg, yb),
            error: Color::Rgb(rr, rg, rb),
            success: green,
            warning: Color::Rgb(yr, yg, yb),
            accent: blue,
            alive: green,
            separator: Color::Rgb(dr, dg, db),
            status_bar_bg: Color::Rgb(30, 30, 46),
            status_bar_fg: Color::Rgb(205, 214, 244),
            label: Color::Rgb(dr, dg, db),
            label_green: green,
            greeting: cyan,
            frame_top: blue,
            frame_bottom: blue,
            identity_val: blue,
            identity_sub: Color::Rgb(58, 110, 140),
            lyapunov_sub: Color::Rgb(45, 110, 68),
            model_name: Color::Rgb(pr, pg, pb),
            git_branch: green,
            dim: Color::Rgb(dr, dg, db),
        }
    }

    /// Light theme — inverted chrome, same brand colors.
    pub fn light() -> Self {
        let (br, bg, bb) = constants::HYDRA_BLUE;
        let blue = Color::Rgb(br, bg, bb);
        let (cr, cg, cb) = constants::HYDRA_CYAN;
        let (gr, gg, gb) = constants::HYDRA_GREEN;
        let green = Color::Rgb(gr, gg, gb);
        let (rr, rg, rb) = constants::HYDRA_RED;
        let (yr, yg, yb) = constants::HYDRA_YELLOW;
        let (pr, pg, pb) = constants::HYDRA_PURPLE;
        let (dr, dg, db) = constants::HYDRA_DIM;
        Self {
            mode: ThemeMode::Light,
            bg_primary: Color::Rgb(239, 241, 245),
            bg_secondary: Color::Rgb(230, 233, 239),
            bg_elevated: Color::Rgb(220, 224, 232),
            fg_primary: Color::Rgb(76, 79, 105),
            fg_secondary: Color::Rgb(108, 111, 133),
            fg_muted: Color::Rgb(dr, dg, db),
            fg_ghost: Color::Rgb(188, 192, 204),
            user_message: blue,
            assistant_text: Color::Rgb(30, 30, 30),
            system_notification: Color::Rgb(yr, yg, yb),
            error: Color::Rgb(rr, rg, rb),
            success: green,
            warning: Color::Rgb(yr, yg, yb),
            accent: blue,
            alive: green,
            separator: Color::Rgb(204, 208, 218),
            status_bar_bg: Color::Rgb(220, 224, 232),
            status_bar_fg: Color::Rgb(76, 79, 105),
            label: Color::Rgb(140, 143, 161),
            label_green: green,
            greeting: Color::Rgb(cr, cg, cb),
            frame_top: blue,
            frame_bottom: blue,
            identity_val: Color::Rgb(30, 102, 245),
            identity_sub: Color::Rgb(80, 130, 180),
            lyapunov_sub: Color::Rgb(40, 100, 60),
            model_name: Color::Rgb(pr, pg, pb),
            git_branch: green,
            dim: Color::Rgb(dr, dg, db),
        }
    }

    /// Return a color based on confidence level (0.0-1.0).
    pub fn confidence_color(confidence: f64) -> Color {
        if confidence > 0.85 {
            Color::Rgb(constants::HYDRA_GREEN.0, constants::HYDRA_GREEN.1, constants::HYDRA_GREEN.2)
        } else if confidence >= 0.50 {
            Color::Rgb(constants::HYDRA_YELLOW.0, constants::HYDRA_YELLOW.1, constants::HYDRA_YELLOW.2)
        } else {
            Color::Rgb(constants::HYDRA_RED.0, constants::HYDRA_RED.1, constants::HYDRA_RED.2)
        }
    }

    /// Return a color based on health percentage (0.0-100.0).
    pub fn health_color(pct: f64) -> Color {
        if pct >= 80.0 {
            Color::Rgb(constants::HYDRA_GREEN.0, constants::HYDRA_GREEN.1, constants::HYDRA_GREEN.2)
        } else if pct >= 50.0 {
            Color::Rgb(constants::HYDRA_YELLOW.0, constants::HYDRA_YELLOW.1, constants::HYDRA_YELLOW.2)
        } else {
            Color::Rgb(constants::HYDRA_RED.0, constants::HYDRA_RED.1, constants::HYDRA_RED.2)
        }
    }

    /// Return a color for a thinking phase name.
    pub fn phase_color(phase: &str) -> Color {
        match phase {
            "thinking" | "reasoning" => Color::Rgb(constants::HYDRA_PURPLE.0, constants::HYDRA_PURPLE.1, constants::HYDRA_PURPLE.2),
            "searching" | "reading" => Color::Rgb(constants::HYDRA_CYAN.0, constants::HYDRA_CYAN.1, constants::HYDRA_CYAN.2),
            "writing" | "generating" => Color::Rgb(constants::HYDRA_GREEN.0, constants::HYDRA_GREEN.1, constants::HYDRA_GREEN.2),
            "error" | "failed" => Color::Rgb(constants::HYDRA_RED.0, constants::HYDRA_RED.1, constants::HYDRA_RED.2),
            _ => Color::Rgb(constants::HYDRA_BLUE.0, constants::HYDRA_BLUE.1, constants::HYDRA_BLUE.2),
        }
    }

    /// Auto-detect theme from terminal background color.
    pub fn auto_detect() -> Self {
        match terminal_light::luma() {
            Ok(luma) if luma > 0.6 => Self::light(),
            _ => Self::dark(),
        }
    }

    /// Get theme by name.
    pub fn by_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            "auto" => Self::auto_detect(),
            _ => Self::dark(),
        }
    }

    /// Return the theme name.
    pub fn name(&self) -> &'static str {
        match self.mode {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        }
    }
}

/// Global theme holder — set once at startup, switchable via /theme command.
static THEME: std::sync::OnceLock<std::sync::RwLock<Theme>> = std::sync::OnceLock::new();

/// Initialize the global theme. Call once at startup.
pub fn init(theme: Theme) {
    let _ = THEME.set(std::sync::RwLock::new(theme));
}

/// Get the current theme. Panics if init() was not called.
pub fn current() -> Theme {
    THEME
        .get()
        .expect("theme not initialized — call theme::init() at startup")
        .read()
        .expect("theme lock poisoned")
        .clone()
}

/// Switch to a new theme at runtime.
pub fn switch(theme: Theme) {
    if let Some(lock) = THEME.get() {
        if let Ok(mut t) = lock.write() {
            *t = theme;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_has_dark_bg() {
        let t = Theme::dark();
        assert_eq!(t.mode, ThemeMode::Dark);
        assert_eq!(t.bg_primary, Color::Rgb(12, 12, 12));
    }

    #[test]
    fn light_theme_has_light_bg() {
        let t = Theme::light();
        assert_eq!(t.mode, ThemeMode::Light);
        assert_eq!(t.bg_primary, Color::Rgb(239, 241, 245));
    }

    #[test]
    fn by_name_returns_correct_theme() {
        assert_eq!(Theme::by_name("dark").mode, ThemeMode::Dark);
        assert_eq!(Theme::by_name("light").mode, ThemeMode::Light);
        assert_eq!(Theme::by_name("unknown").mode, ThemeMode::Dark);
    }

    #[test]
    fn confidence_color_ranges() {
        let high = Theme::confidence_color(0.9);
        let mid = Theme::confidence_color(0.6);
        let low = Theme::confidence_color(0.3);
        assert_ne!(high, mid);
        assert_ne!(mid, low);
    }
}
