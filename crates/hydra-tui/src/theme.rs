//! Theme system — dark and light themes with auto-detection.
//!
//! Brand colors (12 verb colors, 7 dot colors) stay the same in both themes.
//! Only UI chrome colors change between themes.
//! Auto-detects terminal background on startup via terminal-light crate.

use ratatui::style::Color;

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
}

impl Theme {
    /// Dark theme — Catppuccin Mocha inspired, matches spec COLOR_BG (12, 12, 12).
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            bg_primary: Color::Rgb(12, 12, 12),
            bg_secondary: Color::Rgb(17, 17, 17),
            bg_elevated: Color::Rgb(30, 30, 46),
            fg_primary: Color::Rgb(180, 180, 180),
            fg_secondary: Color::Rgb(100, 100, 100),
            fg_muted: Color::Rgb(68, 68, 68),
            fg_ghost: Color::Rgb(46, 46, 46),
            user_message: Color::Rgb(166, 227, 161),
            assistant_text: Color::Rgb(205, 214, 244),
            system_notification: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            accent: Color::Rgb(200, 169, 110),
            alive: Color::Rgb(61, 140, 94),
            separator: Color::Rgb(34, 34, 34),
            status_bar_bg: Color::Rgb(30, 30, 46),
            status_bar_fg: Color::Rgb(205, 214, 244),
            label: Color::Rgb(68, 68, 68),
            label_green: Color::Rgb(90, 122, 74),
            greeting: Color::Rgb(224, 200, 138),
            frame_top: Color::Rgb(122, 106, 74),
            frame_bottom: Color::Rgb(37, 58, 37),
            identity_val: Color::Rgb(106, 170, 212),
            identity_sub: Color::Rgb(58, 110, 140),
            lyapunov_sub: Color::Rgb(45, 110, 68),
        }
    }

    /// Light theme — Catppuccin Latte inspired.
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            bg_primary: Color::Rgb(239, 241, 245),
            bg_secondary: Color::Rgb(230, 233, 239),
            bg_elevated: Color::Rgb(220, 224, 232),
            fg_primary: Color::Rgb(76, 79, 105),
            fg_secondary: Color::Rgb(108, 111, 133),
            fg_muted: Color::Rgb(156, 160, 176),
            fg_ghost: Color::Rgb(188, 192, 204),
            user_message: Color::Rgb(64, 160, 43),
            assistant_text: Color::Rgb(76, 79, 105),
            system_notification: Color::Rgb(223, 142, 29),
            error: Color::Rgb(210, 15, 57),
            success: Color::Rgb(64, 160, 43),
            warning: Color::Rgb(223, 142, 29),
            accent: Color::Rgb(200, 169, 110),
            alive: Color::Rgb(61, 140, 94),
            separator: Color::Rgb(204, 208, 218),
            status_bar_bg: Color::Rgb(220, 224, 232),
            status_bar_fg: Color::Rgb(76, 79, 105),
            label: Color::Rgb(140, 143, 161),
            label_green: Color::Rgb(80, 120, 70),
            greeting: Color::Rgb(142, 120, 68),
            frame_top: Color::Rgb(142, 126, 94),
            frame_bottom: Color::Rgb(57, 78, 57),
            identity_val: Color::Rgb(30, 102, 245),
            identity_sub: Color::Rgb(80, 130, 180),
            lyapunov_sub: Color::Rgb(40, 100, 60),
        }
    }

    /// Auto-detect theme from terminal background color.
    /// Falls back to dark if detection fails.
    pub fn auto_detect() -> Self {
        // terminal-light crate returns luma (0.0 = black, 1.0 = white)
        // If luma > 0.6, the terminal has a light background
        match terminal_light::luma() {
            Ok(luma) if luma > 0.6 => Self::light(),
            _ => Self::dark(),
        }
    }

    /// Get theme by name.
    pub fn by_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
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
}
