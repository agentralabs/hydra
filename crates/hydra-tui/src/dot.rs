//! Dot kinds — the visual atoms of tool feedback in the stream.
//!
//! Each tool invocation is represented as a colored dot.
//! The 7 dot kinds map to 7 permanent colors.

use ratatui::style::Color;

use crate::constants;

/// The 7 kinds of dots that appear in the stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DotKind {
    /// Tool is currently executing.
    Active,
    /// Tool completed successfully.
    Success,
    /// Tool failed.
    Error,
    /// Narration / informational.
    Narration,
    /// Read operation (file, memory, etc.).
    Read,
    /// Cognitive operation (belief, prediction, etc.).
    Cognitive,
    /// Companion-initiated action.
    Companion,
}

impl DotKind {
    /// Return the ratatui color for this dot kind.
    pub fn color(&self) -> Color {
        let (r, g, b) = self.rgb();
        Color::Rgb(r, g, b)
    }

    /// Return the raw RGB tuple for this dot kind.
    pub fn rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::Active => constants::DOT_COLOR_ACTIVE,
            Self::Success => constants::DOT_COLOR_SUCCESS,
            Self::Error => constants::DOT_COLOR_ERROR,
            Self::Narration => constants::DOT_COLOR_NARRATION,
            Self::Read => constants::DOT_COLOR_READ,
            Self::Cognitive => constants::DOT_COLOR_COGNITIVE,
            Self::Companion => constants::DOT_COLOR_COMPANION,
        }
    }

    /// Return all dot kinds.
    pub fn all() -> &'static [DotKind] {
        &[
            Self::Active,
            Self::Success,
            Self::Error,
            Self::Narration,
            Self::Read,
            Self::Cognitive,
            Self::Companion,
        ]
    }

    /// Return the dot character for display.
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Active => "●",
            Self::Success => "✓",
            Self::Error => "✗",
            Self::Narration => "○",
            Self::Read => "◉",
            Self::Cognitive => "◆",
            Self::Companion => "◈",
        }
    }
}

impl std::fmt::Display for DotKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.symbol())
    }
}
