use ratatui::style::{Modifier, Style};

use super::palette::*;

// Frame styles
pub fn frame_border() -> Style {
    Style::default().fg(HYDRA_BLUE)
}

pub fn frame_title() -> Style {
    Style::default().fg(HYDRA_BLUE).add_modifier(Modifier::BOLD)
}

pub fn frame_username() -> Style {
    Style::default()
        .fg(HYDRA_CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn frame_model() -> Style {
    Style::default().fg(HYDRA_PURPLE)
}

pub fn frame_git_branch() -> Style {
    Style::default().fg(HYDRA_GREEN)
}

// Chat styles
pub fn user_message() -> Style {
    Style::default().fg(HYDRA_FG)
}

pub fn assistant_message() -> Style {
    Style::default().fg(HYDRA_FG)
}

pub fn system_message() -> Style {
    Style::default().fg(HYDRA_DIM)
}

// Input styles
pub fn input_prompt() -> Style {
    Style::default()
        .fg(HYDRA_CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn input_hint() -> Style {
    Style::default().fg(HYDRA_DIM)
}

pub fn input_border_active() -> Style {
    Style::default().fg(HYDRA_BLUE)
}

pub fn input_border_disabled() -> Style {
    Style::default().fg(HYDRA_BORDER)
}

// Tool result styles
pub fn tool_sister_name() -> Style {
    Style::default().fg(HYDRA_CYAN)
}

pub fn tool_duration() -> Style {
    Style::default().fg(HYDRA_DIM)
}

pub fn tool_connector() -> Style {
    Style::default().fg(HYDRA_DIM)
}

// Diff styles
pub fn diff_removed() -> Style {
    Style::default().bg(DIFF_RED_BG)
}

pub fn diff_added() -> Style {
    Style::default().bg(DIFF_GREEN_BG)
}

pub fn diff_line_number() -> Style {
    Style::default().fg(HYDRA_DIM)
}

// Status indicators
pub fn status_ok() -> Style {
    Style::default().fg(HYDRA_GREEN)
}

pub fn status_warn() -> Style {
    Style::default().fg(HYDRA_YELLOW)
}

pub fn status_err() -> Style {
    Style::default().fg(HYDRA_RED)
}

pub fn dim() -> Style {
    Style::default().fg(HYDRA_DIM)
}

pub fn bold() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

// Section headers
pub fn section_header() -> Style {
    Style::default()
        .fg(HYDRA_BLUE)
        .add_modifier(Modifier::BOLD)
}

// Briefing priority styles
pub fn briefing_urgent() -> Style {
    Style::default().fg(HYDRA_RED)
}

pub fn briefing_important() -> Style {
    Style::default().fg(HYDRA_YELLOW)
}

pub fn briefing_info() -> Style {
    Style::default().fg(HYDRA_DIM)
}

// Streaming
pub fn streaming_indicator() -> Style {
    Style::default().fg(HYDRA_PURPLE)
}
