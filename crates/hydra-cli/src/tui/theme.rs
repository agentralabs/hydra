use ratatui::style::{Color, Modifier, Style};

// Hydra brand palette
pub const HYDRA_BLUE: Color = Color::Rgb(100, 149, 237); // Cornflower blue
pub const HYDRA_CYAN: Color = Color::Rgb(0, 210, 210);
pub const HYDRA_GREEN: Color = Color::Rgb(80, 200, 120);
pub const HYDRA_RED: Color = Color::Rgb(220, 80, 80);
pub const HYDRA_YELLOW: Color = Color::Rgb(240, 200, 80);
pub const HYDRA_ORANGE: Color = Color::Rgb(240, 160, 60);
pub const HYDRA_PURPLE: Color = Color::Rgb(160, 120, 220);
pub const HYDRA_DIM: Color = Color::DarkGray;
pub const HYDRA_BG: Color = Color::Black;
pub const HYDRA_FG: Color = Color::Reset;
pub const HYDRA_BORDER: Color = Color::DarkGray;
pub const HYDRA_BORDER_ACTIVE: Color = Color::Rgb(100, 149, 237);

pub fn _header() -> Style {
    Style::default().fg(HYDRA_BLUE).add_modifier(Modifier::BOLD)
}

pub fn _title() -> Style {
    Style::default()
        .fg(HYDRA_FG)
        .add_modifier(Modifier::BOLD)
}

pub fn dim() -> Style {
    Style::default().fg(HYDRA_DIM)
}

pub fn border() -> Style {
    Style::default().fg(HYDRA_BORDER)
}

pub fn border_active() -> Style {
    Style::default().fg(HYDRA_BORDER_ACTIVE)
}

pub fn status_ok() -> Style {
    Style::default().fg(HYDRA_GREEN)
}

pub fn status_warn() -> Style {
    Style::default().fg(HYDRA_YELLOW)
}

pub fn status_err() -> Style {
    Style::default().fg(HYDRA_RED)
}

pub fn user_msg() -> Style {
    Style::default().fg(HYDRA_FG)
}

pub fn hydra_msg() -> Style {
    Style::default().fg(HYDRA_CYAN)
}

pub fn prompt() -> Style {
    Style::default()
        .fg(HYDRA_BLUE)
        .add_modifier(Modifier::BOLD)
}

pub fn sidebar_label() -> Style {
    Style::default().fg(HYDRA_DIM)
}

pub fn sidebar_value() -> Style {
    Style::default().fg(HYDRA_FG)
}

pub fn phase_color(phase: &str) -> Style {
    match phase {
        "Perceive" => Style::default().fg(HYDRA_BLUE),
        "Think" => Style::default().fg(HYDRA_YELLOW),
        "Decide" => Style::default().fg(HYDRA_ORANGE),
        "Act" => Style::default().fg(HYDRA_GREEN),
        "Learn" => Style::default().fg(HYDRA_PURPLE),
        _ => Style::default().fg(HYDRA_DIM),
    }
}

pub fn _keyword() -> Style {
    Style::default()
        .fg(HYDRA_CYAN)
        .add_modifier(Modifier::BOLD)
}
