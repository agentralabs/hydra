//! Layer 3: Convention Engine — universal UI conventions across all applications.
//!
//! Three tiers:
//! 1. Platform Universal — Cmd+S, Cmd+Z, Tab, Esc (work in every app)
//! 2. Category Universal — Cmd+F in all text editors, Cmd+T in all browsers
//! 3. App Specific — AutoCAD L=Line, Photoshop B=Brush (from AMM)

use std::collections::HashMap;

/// A UI convention (shortcut + expected behavior).
#[derive(Debug, Clone)]
pub struct Convention {
    pub intent: String,
    pub modifier: String,
    pub key: String,
    pub confidence: f64,
    pub tier: ConventionTier,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConventionTier { Platform, Category, App }

/// Universal convention database.
pub struct ConventionEngine {
    platform: Vec<Convention>,
    categories: HashMap<String, Vec<Convention>>,
}

impl ConventionEngine {
    /// Create with built-in platform and category conventions.
    pub fn new() -> Self {
        let platform = vec![
            conv("save", "cmd", "s", 1.0),
            conv("undo", "cmd", "z", 1.0),
            conv("redo", "cmd", "shift+z", 0.95),
            conv("copy", "cmd", "c", 1.0),
            conv("paste", "cmd", "v", 1.0),
            conv("cut", "cmd", "x", 1.0),
            conv("select_all", "cmd", "a", 1.0),
            conv("find", "cmd", "f", 0.98),
            conv("new", "cmd", "n", 0.95),
            conv("open", "cmd", "o", 0.95),
            conv("close", "cmd", "w", 0.95),
            conv("quit", "cmd", "q", 0.98),
            conv("print", "cmd", "p", 0.90),
            conv("preferences", "cmd", ",", 0.85),
            conv("next_field", "", "tab", 1.0),
            conv("confirm", "", "return", 1.0),
            conv("cancel", "", "escape", 0.95),
        ];

        let mut categories = HashMap::new();
        categories.insert("text_editor".into(), vec![
            conv("find_replace", "cmd", "shift+h", 0.8),
            conv("goto_line", "cmd", "g", 0.8),
            conv("comment", "cmd", "/", 0.85),
            conv("indent", "cmd", "]", 0.8),
            conv("dedent", "cmd", "[", 0.8),
        ]);
        categories.insert("browser".into(), vec![
            conv("new_tab", "cmd", "t", 0.95),
            conv("close_tab", "cmd", "w", 0.95),
            conv("reload", "cmd", "r", 0.95),
            conv("back", "cmd", "[", 0.9),
            conv("forward", "cmd", "]", 0.9),
            conv("address_bar", "cmd", "l", 0.9),
        ]);
        categories.insert("creative".into(), vec![
            conv("zoom_in", "cmd", "+", 0.85),
            conv("zoom_out", "cmd", "-", 0.85),
            conv("zoom_fit", "cmd", "0", 0.8),
            conv("export", "cmd", "shift+e", 0.7),
        ]);

        Self { platform, categories }
    }

    /// Resolve an intent to a convention. Checks: platform → category → None.
    pub fn resolve(&self, intent: &str, app_category: &str) -> Option<&Convention> {
        let lower = intent.to_lowercase();
        // Platform first (highest confidence)
        if let Some(c) = self.platform.iter().find(|c| c.intent == lower) {
            return Some(c);
        }
        // Category second
        if let Some(cat_convs) = self.categories.get(app_category) {
            if let Some(c) = cat_convs.iter().find(|c| c.intent == lower) {
                return Some(c);
            }
        }
        None
    }

    /// Get all conventions for a given category (platform + category-specific).
    pub fn all_for_category(&self, app_category: &str) -> Vec<&Convention> {
        let mut all: Vec<&Convention> = self.platform.iter().collect();
        if let Some(cat) = self.categories.get(app_category) {
            all.extend(cat.iter());
        }
        all
    }
}

impl Default for ConventionEngine {
    fn default() -> Self { Self::new() }
}

fn conv(intent: &str, modifier: &str, key: &str, confidence: f64) -> Convention {
    Convention {
        intent: intent.into(), modifier: modifier.into(),
        key: key.into(), confidence,
        tier: if modifier.is_empty() && matches!(key, "tab" | "return" | "escape") {
            ConventionTier::Platform
        } else { ConventionTier::Platform },
    }
}
