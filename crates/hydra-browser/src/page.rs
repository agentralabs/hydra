//! PageUnderstanding — extracts structure from web pages.
//! Identifies forms, links, interactive elements, and page type.

use serde::{Deserialize, Serialize};

/// A detected interactive element on a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageElement {
    pub index: usize,
    pub tag: String,
    pub element_type: Option<String>,
    pub text: String,
    pub name: Option<String>,
    pub id: Option<String>,
    pub href: Option<String>,
    pub placeholder: Option<String>,
}

/// A detected form on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedForm {
    pub form_type: FormType,
    pub fields: Vec<FormField>,
    pub submit_selector: Option<String>,
}

/// Classification of a form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FormType {
    Login,
    Registration,
    Search,
    Contact,
    Payment,
    Unknown,
}

/// A field within a form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub selector: String,
    pub field_type: String,
    pub name: Option<String>,
    pub placeholder: Option<String>,
    pub required: bool,
}

/// Classification of a web page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PageType {
    Login,
    Feed,
    Article,
    Dashboard,
    Form,
    Search,
    Error,
    Unknown,
}

/// Analyzes raw HTML to understand page structure.
pub struct PageAnalyzer;

impl PageAnalyzer {
    /// Detect forms in the HTML source.
    pub fn detect_forms(html: &str) -> Vec<DetectedForm> {
        let mut forms = Vec::new();
        let lower = html.to_lowercase();

        // Detect login forms
        if Self::has_login_indicators(&lower) {
            let mut fields = Vec::new();
            if let Some(sel) = Self::find_field_selector(html, &["email", "username", "user", "login"]) {
                fields.push(FormField {
                    selector: sel,
                    field_type: "text".into(),
                    name: Some("username".into()),
                    placeholder: None,
                    required: true,
                });
            }
            if let Some(sel) = Self::find_password_selector(html) {
                fields.push(FormField {
                    selector: sel,
                    field_type: "password".into(),
                    name: Some("password".into()),
                    placeholder: None,
                    required: true,
                });
            }
            if !fields.is_empty() {
                forms.push(DetectedForm {
                    form_type: FormType::Login,
                    fields,
                    submit_selector: Self::find_submit_selector(html),
                });
            }
        }

        // Detect search forms
        if lower.contains("type=\"search\"") || lower.contains("name=\"q\"") {
            if let Some(sel) = Self::find_field_selector(html, &["search", "q", "query"]) {
                forms.push(DetectedForm {
                    form_type: FormType::Search,
                    fields: vec![FormField {
                        selector: sel,
                        field_type: "search".into(),
                        name: Some("query".into()),
                        placeholder: None,
                        required: true,
                    }],
                    submit_selector: Self::find_submit_selector(html),
                });
            }
        }

        forms
    }

    /// Classify the page type from HTML content.
    pub fn classify_page(html: &str) -> PageType {
        let lower = html.to_lowercase();

        if Self::has_login_indicators(&lower) {
            return PageType::Login;
        }
        if lower.contains("class=\"feed\"")
            || lower.contains("class=\"timeline\"")
            || lower.contains("data-testid=\"tweet\"")
        {
            return PageType::Feed;
        }
        if lower.contains("<article") || lower.contains("class=\"article\"") {
            return PageType::Article;
        }
        if lower.contains("class=\"dashboard\"") || lower.contains("class=\"panel\"") {
            return PageType::Dashboard;
        }
        if lower.contains("class=\"error\"") || lower.contains("404") {
            return PageType::Error;
        }
        if lower.contains("type=\"search\"") {
            return PageType::Search;
        }

        PageType::Unknown
    }

    /// Extract readable text content (strip HTML tags).
    pub fn extract_text(html: &str) -> String {
        let mut result = String::with_capacity(html.len() / 3);
        let mut in_tag = false;
        let in_script = false;

        for c in html.chars() {
            if c == '<' {
                in_tag = true;
                continue;
            }
            if c == '>' {
                in_tag = false;
                continue;
            }
            if in_tag {
                // Check for script/style tags
                continue;
            }
            if !in_script {
                result.push(c);
            }
        }

        // Collapse whitespace
        let collapsed: String = result
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        collapsed
    }

    fn has_login_indicators(lower_html: &str) -> bool {
        lower_html.contains("type=\"password\"")
            && (lower_html.contains("login")
                || lower_html.contains("sign in")
                || lower_html.contains("log in")
                || lower_html.contains("signin"))
    }

    fn find_field_selector(html: &str, name_hints: &[&str]) -> Option<String> {
        let lower = html.to_lowercase();
        for hint in name_hints {
            if lower.contains(&format!("name=\"{hint}\"")) {
                return Some(format!("input[name=\"{hint}\"]"));
            }
            if lower.contains(&format!("id=\"{hint}\"")) {
                return Some(format!("#{hint}"));
            }
            if lower.contains(&format!("type=\"{hint}\"")) {
                return Some(format!("input[type=\"{hint}\"]"));
            }
        }
        None
    }

    fn find_password_selector(html: &str) -> Option<String> {
        if html.to_lowercase().contains("type=\"password\"") {
            Some("input[type=\"password\"]".to_string())
        } else {
            None
        }
    }

    fn find_submit_selector(html: &str) -> Option<String> {
        let lower = html.to_lowercase();
        if lower.contains("type=\"submit\"") {
            Some("input[type=\"submit\"], button[type=\"submit\"]".into())
        } else if lower.contains("<button") {
            Some("button".into())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_login_form() {
        let html = r#"<form><input name="email" type="text"><input type="password"><button type="submit">Login</button></form>"#;
        let forms = PageAnalyzer::detect_forms(html);
        assert!(!forms.is_empty());
        assert_eq!(forms[0].form_type, FormType::Login);
        assert_eq!(forms[0].fields.len(), 2);
    }

    #[test]
    fn classify_login_page() {
        let html = r#"<html><body><h1>Sign In</h1><input type="password"></body></html>"#;
        assert_eq!(PageAnalyzer::classify_page(html), PageType::Login);
    }

    #[test]
    fn classify_article_page() {
        let html = r#"<html><body><article>Some content</article></body></html>"#;
        assert_eq!(PageAnalyzer::classify_page(html), PageType::Article);
    }

    #[test]
    fn extract_text_strips_tags() {
        let html = "<html><body><p>Hello <b>world</b>!</p></body></html>";
        let text = PageAnalyzer::extract_text(html);
        assert!(text.contains("Hello world!"));
    }

    #[test]
    fn detect_search_form() {
        let html = r#"<form><input name="q" type="search"><button>Go</button></form>"#;
        let forms = PageAnalyzer::detect_forms(html);
        assert!(forms.iter().any(|f| f.form_type == FormType::Search));
    }
}
