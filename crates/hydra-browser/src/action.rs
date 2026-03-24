//! BrowserAction — all possible browser interactions.
//! Each action is serializable for executor dispatch and audit receipts.

use serde::{Deserialize, Serialize};

/// Direction for scrolling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// A single browser action to execute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BrowserAction {
    /// Navigate to a URL.
    Navigate { url: String },
    /// Click an element by CSS selector.
    Click { selector: String },
    /// Type text into an element by CSS selector.
    Type { selector: String, text: String },
    /// Scroll the page.
    Scroll {
        direction: ScrollDirection,
        amount: u32,
    },
    /// Go back in history.
    Back,
    /// Go forward in history.
    Forward,
    /// Refresh the page.
    Refresh,
    /// Wait for a duration (milliseconds).
    Wait { ms: u64 },
    /// Select a value from a dropdown.
    Select { selector: String, value: String },
    /// Hover over an element.
    Hover { selector: String },
    /// Take a screenshot of the current page.
    Screenshot,
    /// Get the page HTML source.
    GetHtml,
    /// Get all clickable/interactive elements on the page.
    GetElements,
    /// Get the visible text content of the page.
    GetText,
}

impl BrowserAction {
    /// Human-readable label for this action.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Navigate { .. } => "navigate",
            Self::Click { .. } => "click",
            Self::Type { .. } => "type",
            Self::Scroll { .. } => "scroll",
            Self::Back => "back",
            Self::Forward => "forward",
            Self::Refresh => "refresh",
            Self::Wait { .. } => "wait",
            Self::Select { .. } => "select",
            Self::Hover { .. } => "hover",
            Self::Screenshot => "screenshot",
            Self::GetHtml => "get_html",
            Self::GetElements => "get_elements",
            Self::GetText => "get_text",
        }
    }

    /// Whether this action modifies page state (vs read-only).
    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            Self::Click { .. }
                | Self::Type { .. }
                | Self::Select { .. }
                | Self::Navigate { .. }
                | Self::Scroll { .. }
                | Self::Back
                | Self::Forward
                | Self::Refresh
        )
    }
}

/// Result of executing a browser action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Which action was executed.
    pub action: String,
    /// Whether it succeeded.
    pub success: bool,
    /// Output data (HTML, text, base64 screenshot, element list).
    pub data: String,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Error message if failed.
    pub error: Option<String>,
}

impl ActionResult {
    pub fn ok(action: &str, data: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            action: action.to_string(),
            success: true,
            data: data.into(),
            duration_ms,
            error: None,
        }
    }

    pub fn err(action: &str, error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            action: action.to_string(),
            success: false,
            data: String::new(),
            duration_ms,
            error: Some(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_serialization_roundtrip() {
        let action = BrowserAction::Type {
            selector: "#email".into(),
            text: "user@example.com".into(),
        };
        let json = serde_json::to_string(&action).unwrap();
        let back: BrowserAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn mutation_detection() {
        assert!(BrowserAction::Click {
            selector: "btn".into()
        }
        .is_mutation());
        assert!(!BrowserAction::Screenshot.is_mutation());
        assert!(!BrowserAction::GetText.is_mutation());
    }

    #[test]
    fn labels_are_descriptive() {
        assert_eq!(BrowserAction::Screenshot.label(), "screenshot");
        assert_eq!(
            BrowserAction::Navigate {
                url: "x".into()
            }
            .label(),
            "navigate"
        );
    }
}
