//! Layer 2: Application Mind Model (AMM) — structural understanding of any application.
//!
//! On first contact with an app, Hydra runs a discovery protocol:
//! walk accessibility tree, map menus, scan toolbars, probe shortcuts.
//! Stored as TOML in ~/.hydra/app_models/. Never needs vision for known apps.

use std::collections::HashMap;
use std::path::PathBuf;

/// A keyboard shortcut.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Shortcut {
    pub modifier: String,  // "cmd", "ctrl", "alt", ""
    pub key: String,       // "s", "z", "l"
}

/// A tool in a toolbar with its position.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolbarItem {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub icon_hash: u64,
}

/// Screen layout regions.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AppLayout {
    pub menu_bar_y: (f64, f64),
    pub toolbar_y: (f64, f64),
    pub canvas_x: (f64, f64),
    pub canvas_y: (f64, f64),
    pub status_y: (f64, f64),
}

/// Complete structural model of an application.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppModel {
    pub name: String,
    pub bundle_id: String,
    pub fingerprint: u64,
    pub menus: HashMap<String, Vec<String>>,
    pub shortcuts: HashMap<String, Shortcut>,
    pub toolbar: Vec<ToolbarItem>,
    pub layout: AppLayout,
    pub first_contact_done: bool,
    pub discovery_time_ms: u64,
}

impl AppModel {
    /// Load an existing app model from ~/.hydra/app_models/{name}.toml.
    pub fn load(app_name: &str) -> Option<Self> {
        let path = app_models_dir().join(format!("{}.toml", sanitize(app_name)));
        let content = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Save this model to disk.
    pub fn save(&self) {
        let dir = app_models_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.toml", sanitize(&self.name)));
        match toml::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    eprintln!("hydra-amm: save failed: {e}");
                } else {
                    eprintln!("hydra-amm: saved model for '{}' to {}", self.name, path.display());
                }
            }
            Err(e) => eprintln!("hydra-amm: serialize failed: {e}"),
        }
    }

    /// Run first contact protocol: discover the currently focused app.
    pub fn first_contact() -> Self {
        let start = std::time::Instant::now();
        eprintln!("hydra-amm: first contact protocol starting...");

        // Get focused app info
        let windows = crate::app::AppManager::list_windows().unwrap_or_default();
        let focused = windows.iter().find(|w| w.is_focused).or(windows.first());
        let name = focused.map(|w| w.app_name.clone()).unwrap_or_else(|| "unknown".into());
        let bundle_id = focused.map(|w| w.id.clone()).unwrap_or_default();

        let mut model = Self {
            name: name.clone(), bundle_id, fingerprint: 0,
            menus: HashMap::new(), shortcuts: HashMap::new(),
            toolbar: Vec::new(), layout: AppLayout::default(),
            first_contact_done: false, discovery_time_ms: 0,
        };

        // Phase 1: Accessibility tree — extract all elements
        match crate::accessibility::AccessibilityTree::from_focused_app() {
            Ok(tree) => {
                eprintln!("hydra-amm: a11y tree: {} elements", tree.elements.len());
                for el in &tree.elements {
                    // Build menu map from menu items
                    if el.role.contains("menu") || el.role.contains("Menu") {
                        let parent = el.title.split(" > ").next().unwrap_or(&el.title);
                        model.menus.entry(parent.to_string())
                            .or_default().push(el.title.clone());
                    }
                    // Record toolbar items by position
                    if el.role.contains("button") || el.role.contains("Button") {
                        if el.position.1 < 100.0 && !el.title.is_empty() {
                            model.toolbar.push(ToolbarItem {
                                name: el.title.clone(),
                                x: el.position.0, y: el.position.1,
                                icon_hash: 0,
                            });
                        }
                    }
                }
            }
            Err(e) => eprintln!("hydra-amm: a11y failed: {e}"),
        }

        // Phase 2: Probe common shortcuts
        let common = [
            ("save", "cmd", "s"), ("undo", "cmd", "z"), ("redo", "cmd", "shift+z"),
            ("copy", "cmd", "c"), ("paste", "cmd", "v"), ("cut", "cmd", "x"),
            ("find", "cmd", "f"), ("new", "cmd", "n"), ("open", "cmd", "o"),
            ("close", "cmd", "w"), ("quit", "cmd", "q"), ("print", "cmd", "p"),
            ("select_all", "cmd", "a"),
        ];
        for (intent, modifier, key) in common {
            model.shortcuts.insert(intent.into(), Shortcut {
                modifier: modifier.into(), key: key.into(),
            });
        }

        // Phase 3: Infer layout from element positions
        if !model.toolbar.is_empty() {
            let max_toolbar_y = model.toolbar.iter()
                .map(|t| t.y).fold(0.0_f64, f64::max);
            model.layout.menu_bar_y = (0.0, 25.0);
            model.layout.toolbar_y = (25.0, max_toolbar_y + 30.0);
        }

        model.fingerprint = hash_menus(&model.menus);
        model.first_contact_done = true;
        model.discovery_time_ms = start.elapsed().as_millis() as u64;
        eprintln!("hydra-amm: first contact complete for '{}' in {}ms ({} menus, {} tools, {} shortcuts)",
            name, model.discovery_time_ms, model.menus.len(),
            model.toolbar.len(), model.shortcuts.len());

        model.save();
        model
    }

    /// Look up a shortcut for a given intent.
    pub fn find_shortcut(&self, intent: &str) -> Option<&Shortcut> {
        let lower = intent.to_lowercase();
        self.shortcuts.get(&lower)
            .or_else(|| self.shortcuts.iter()
                .find(|(k, _)| lower.contains(k.as_str()))
                .map(|(_, v)| v))
    }

    /// Find a menu path for an intent (e.g., "export" → ["File", "Export"]).
    pub fn find_menu_path(&self, intent: &str) -> Option<Vec<String>> {
        let lower = intent.to_lowercase();
        for (menu, items) in &self.menus {
            for item in items {
                if item.to_lowercase().contains(&lower) {
                    return Some(vec![menu.clone(), item.clone()]);
                }
            }
        }
        None
    }

    /// Find a toolbar item by name.
    pub fn find_tool(&self, name: &str) -> Option<&ToolbarItem> {
        let lower = name.to_lowercase();
        self.toolbar.iter().find(|t| t.name.to_lowercase().contains(&lower))
    }

    /// Check if we have a model for this app already.
    pub fn exists(app_name: &str) -> bool {
        app_models_dir().join(format!("{}.toml", sanitize(app_name))).exists()
    }
}

fn app_models_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hydra/app_models")
}

fn sanitize(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>().to_lowercase()
}

fn hash_menus(menus: &HashMap<String, Vec<String>>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    let mut keys: Vec<_> = menus.keys().collect();
    keys.sort();
    for k in keys { k.hash(&mut h); menus[k].hash(&mut h); }
    h.finish()
}
