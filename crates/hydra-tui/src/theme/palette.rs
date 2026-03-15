use ratatui::style::Color;

// Hydra brand palette — 7 semantic colors + utility
pub const HYDRA_BLUE: Color = Color::Rgb(100, 149, 237); // Primary accent, borders, headers
pub const HYDRA_CYAN: Color = Color::Rgb(0, 210, 210); // Username, keywords, sister working
pub const HYDRA_GREEN: Color = Color::Rgb(80, 200, 120); // Success, connected, healthy
pub const HYDRA_RED: Color = Color::Rgb(220, 80, 80); // Error, offline, critical
pub const HYDRA_YELLOW: Color = Color::Rgb(240, 200, 80); // Warning, partial, uncertain
pub const HYDRA_ORANGE: Color = Color::Rgb(240, 160, 60); // Decide phase, action needed
pub const HYDRA_PURPLE: Color = Color::Rgb(160, 120, 220); // Model name, Learn phase
pub const HYDRA_DIM: Color = Color::DarkGray; // Labels, paths, subtle text
pub const HYDRA_BG: Color = Color::Reset; // Terminal default
pub const HYDRA_FG: Color = Color::Reset; // Terminal default
pub const HYDRA_BORDER: Color = Color::DarkGray; // Inactive borders
pub const HYDRA_BORDER_ACTIVE: Color = Color::Rgb(100, 149, 237); // Active = HYDRA_BLUE

// Diff backgrounds
pub const DIFF_RED_BG: Color = Color::Rgb(80, 20, 20);
pub const DIFF_GREEN_BG: Color = Color::Rgb(20, 60, 20);

/// Dot color by activity category
pub fn dot_color(category: DotCategory) -> Color {
    match category {
        DotCategory::Thinking => HYDRA_DIM,
        DotCategory::SisterWorking => HYDRA_CYAN,
        DotCategory::Success => HYDRA_GREEN,
        DotCategory::Error => HYDRA_RED,
        DotCategory::Warning => HYDRA_YELLOW,
        DotCategory::Cognitive => HYDRA_PURPLE,
        DotCategory::Approval => HYDRA_ORANGE,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DotCategory {
    Thinking,
    SisterWorking,
    Success,
    Error,
    Warning,
    Cognitive,
    Approval,
}

/// Confidence bracket color for belief borders
pub fn confidence_color(confidence: f64) -> Color {
    if confidence > 0.85 {
        HYDRA_GREEN
    } else if confidence >= 0.5 {
        HYDRA_YELLOW
    } else {
        HYDRA_RED
    }
}

/// Health percentage color
pub fn health_color(pct: f64) -> Color {
    if pct >= 90.0 {
        HYDRA_GREEN
    } else if pct >= 50.0 {
        HYDRA_YELLOW
    } else {
        HYDRA_RED
    }
}

/// Cognitive phase color
pub fn phase_color(phase: &str) -> Color {
    match phase {
        "Perceive" => HYDRA_BLUE,
        "Think" => HYDRA_YELLOW,
        "Decide" => HYDRA_ORANGE,
        "Act" => HYDRA_GREEN,
        "Learn" => HYDRA_PURPLE,
        _ => HYDRA_DIM,
    }
}
