//! Core types for semantic affordance navigation.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// The role of a semantic element — inferred from tag, type, role attribute, ARIA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementRole {
    Button,
    Link,
    TextInput,
    PasswordInput,
    EmailInput,
    SearchInput,
    Textarea,
    Select,
    Checkbox,
    Radio,
    Tab,
    MenuItem,
    Dialog,
    Generic,
}

impl ElementRole {
    /// Infer role from HTML tag, type attribute, and role attribute.
    pub fn infer(tag: &str, input_type: Option<&str>, role_attr: Option<&str>) -> Self {
        if let Some(role) = role_attr {
            match role {
                "button" => return Self::Button,
                "link" => return Self::Link,
                "tab" => return Self::Tab,
                "menuitem" => return Self::MenuItem,
                "dialog" => return Self::Dialog,
                "checkbox" => return Self::Checkbox,
                "radio" => return Self::Radio,
                "textbox" | "searchbox" => return Self::TextInput,
                _ => {}
            }
        }
        match tag {
            "button" | "submit" => Self::Button,
            "a" => Self::Link,
            "select" => Self::Select,
            "textarea" => Self::Textarea,
            "input" => match input_type.unwrap_or("text") {
                "submit" | "button" | "reset" => Self::Button,
                "password" => Self::PasswordInput,
                "email" => Self::EmailInput,
                "search" => Self::SearchInput,
                "checkbox" => Self::Checkbox,
                "radio" => Self::Radio,
                _ => Self::TextInput,
            },
            _ => Self::Generic,
        }
    }

    pub fn is_actionable(&self) -> bool {
        !matches!(self, Self::Generic)
    }

    pub fn is_input(&self) -> bool {
        matches!(self, Self::TextInput | Self::PasswordInput | Self::EmailInput
            | Self::SearchInput | Self::Textarea)
    }
}

/// A parsed semantic element from the DOM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticElement {
    pub selector: String,
    pub tag: String,
    pub role: ElementRole,
    pub label: String,
    pub input_type: Option<String>,
    pub href: Option<String>,
    pub is_visible: bool,
    pub is_disabled: bool,
    pub parent_context: Option<String>,
    pub aria: HashMap<String, String>,
}

impl SemanticElement {
    /// All searchable text for this element (label + type + context).
    pub fn search_text(&self) -> String {
        let mut parts = vec![self.label.clone()];
        if let Some(t) = &self.input_type { parts.push(t.clone()); }
        if let Some(ctx) = &self.parent_context { parts.push(ctx.clone()); }
        if let Some(href) = &self.href { parts.push(href.clone()); }
        parts.join(" ").to_lowercase()
    }
}

/// A grouped form structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticForm {
    pub name: String,
    pub fields: Vec<SemanticElement>,
    pub submit_selector: Option<String>,
    pub form_type: FormIntent,
}

/// What a form is for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormIntent {
    Login,
    Search,
    Registration,
    Compose,
    DataEntry,
    Unknown,
}

/// A navigation link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavLink {
    pub selector: String,
    pub label: String,
    pub href: String,
    pub is_current: bool,
}

/// The page constitution — what the page allows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageConstitution {
    pub url: String,
    pub title: String,
    pub elements: Vec<SemanticElement>,
    pub forms: Vec<SemanticForm>,
    pub navigation: Vec<NavLink>,
    pub primary_action: Option<String>,
    pub search_input: Option<String>,
    pub guards: Vec<SemanticElement>,
    pub parsed_at: chrono::DateTime<chrono::Utc>,
}

/// A single planned step.
#[derive(Debug, Clone)]
pub struct PlannedStep {
    pub action: hydra_browser::BrowserAction,
    pub description: String,
    pub selector: String,
}

/// An execution plan derived from intent matching.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub steps: Vec<PlannedStep>,
    pub confidence: f64,
    pub strategy: String,
}

/// Result of a semantic navigation attempt.
#[derive(Debug)]
pub enum NavResult {
    /// Semantic nav handled the task successfully.
    Success,
    /// DOM not parseable or confidence too low — fall back to vision.
    Unparseable(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_button_from_tag() {
        assert_eq!(ElementRole::infer("button", None, None), ElementRole::Button);
    }

    #[test]
    fn infer_password_from_input_type() {
        assert_eq!(ElementRole::infer("input", Some("password"), None), ElementRole::PasswordInput);
    }

    #[test]
    fn infer_role_from_aria() {
        assert_eq!(ElementRole::infer("div", None, Some("button")), ElementRole::Button);
        assert_eq!(ElementRole::infer("span", None, Some("tab")), ElementRole::Tab);
    }

    #[test]
    fn search_text_combines_fields() {
        let el = SemanticElement {
            selector: "#test".into(), tag: "input".into(), role: ElementRole::EmailInput,
            label: "Email address".into(), input_type: Some("email".into()),
            href: None, is_visible: true, is_disabled: false,
            parent_context: Some("login form".into()), aria: HashMap::new(),
        };
        let text = el.search_text();
        assert!(text.contains("email"));
        assert!(text.contains("login form"));
    }
}
