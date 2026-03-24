//! Accessibility tree parser — queries OS for UI element trees.
//! macOS: AppleScript via System Events. Linux: AT-SPI2 via gdbus.
//! Gives Hydra the same understanding of desktop apps that semantic-nav gives for web.

use crate::errors::DesktopError;

/// A single accessible UI element.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccessibleElement {
    pub role: String,
    pub title: String,
    pub value: Option<String>,
    pub position: (f64, f64),
    pub size: (f64, f64),
    pub is_enabled: bool,
}

/// The accessibility tree of the focused application.
#[derive(Debug, Clone)]
pub struct AccessibilityTree {
    pub app_name: String,
    pub elements: Vec<AccessibleElement>,
}

impl AccessibilityTree {
    /// Query the OS for the accessibility tree of the frontmost application.
    /// EC-2.1: Returns empty tree if app doesn't support accessibility.
    pub fn from_focused_app() -> Result<Self, DesktopError> {
        if cfg!(target_os = "macos") {
            Self::from_macos()
        } else if cfg!(target_os = "linux") {
            Self::from_linux()
        } else {
            Err(DesktopError::UnsupportedPlatform("accessibility".into()))
        }
    }

    /// Find element by title with fuzzy matching (EC-2.2: Levenshtein ≤ 2).
    pub fn find_by_title(&self, title: &str) -> Option<&AccessibleElement> {
        let lower = title.to_lowercase();
        // Exact match first
        if let Some(el) = self.elements.iter().find(|e| e.title.to_lowercase() == lower) {
            return Some(el);
        }
        // Fuzzy match (Levenshtein distance ≤ 2)
        self.elements.iter().find(|e| levenshtein(&e.title.to_lowercase(), &lower) <= 2)
    }

    /// Find elements by accessibility role.
    pub fn find_by_role(&self, role: &str) -> Vec<&AccessibleElement> {
        let lower = role.to_lowercase();
        self.elements.iter()
            .filter(|e| e.role.to_lowercase().contains(&lower))
            .collect()
    }

    /// Find nearest element to a reference position (EC-2.3: multiple matches).
    pub fn find_nearest(&self, title: &str, ref_pos: (f64, f64)) -> Option<&AccessibleElement> {
        let lower = title.to_lowercase();
        let matches: Vec<&AccessibleElement> = self.elements.iter()
            .filter(|e| e.title.to_lowercase().contains(&lower) || levenshtein(&e.title.to_lowercase(), &lower) <= 2)
            .collect();
        matches.into_iter().min_by(|a, b| {
            let dist_a = distance(a.position, ref_pos);
            let dist_b = distance(b.position, ref_pos);
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Get the center coordinates of an element for clicking.
    pub fn element_center(el: &AccessibleElement) -> (f64, f64) {
        (el.position.0 + el.size.0 / 2.0, el.position.1 + el.size.1 / 2.0)
    }

    // ── macOS Implementation ──

    fn from_macos() -> Result<Self, DesktopError> {
        // Get frontmost app name
        let app_name = get_macos_frontmost_app()?;

        // Query UI elements via AppleScript
        let script = format!(
            r#"tell application "System Events"
    set frontApp to first process whose frontmost is true
    set appName to name of frontApp
    set elemList to ""
    try
        set uiElems to every UI element of front window of frontApp
        repeat with elem in uiElems
            try
                set elemRole to role of elem
                set elemTitle to ""
                try
                    set elemTitle to title of elem
                end try
                if elemTitle is missing value then set elemTitle to ""
                try
                    set elemTitle to description of elem
                end try
                set elemPos to position of elem
                set elemSize to size of elem
                set elemEnabled to enabled of elem
                set elemList to elemList & elemRole & "|||" & elemTitle & "|||" & (item 1 of elemPos) & "," & (item 2 of elemPos) & "|||" & (item 1 of elemSize) & "," & (item 2 of elemSize) & "|||" & elemEnabled & "
"
            end try
        end repeat
    end try
    return elemList
end tell"#
        );

        let output = std::process::Command::new("osascript").arg("-e").arg(&script)
            .output().map_err(|e| DesktopError::CaptureFailed(format!("osascript: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let elements = parse_macos_output(&stdout);

        eprintln!("hydra-a11y: {} elements from '{}'", elements.len(), app_name);
        Ok(AccessibilityTree { app_name, elements })
    }

    // ── Linux Implementation ──

    fn from_linux() -> Result<Self, DesktopError> {
        // Use xdotool to get active window, then basic property extraction
        let output = std::process::Command::new("xdotool").arg("getactivewindow").arg("getwindowname")
            .output().map_err(|e| DesktopError::CaptureFailed(format!("xdotool: {e}")))?;
        let app_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Linux a11y tree extraction is limited without AT-SPI2 bindings
        // Return empty tree — will cascade to Tier 2 (OCR)
        eprintln!("hydra-a11y: Linux — limited a11y, will cascade to OCR");
        Ok(AccessibilityTree { app_name, elements: Vec::new() })
    }
}

fn get_macos_frontmost_app() -> Result<String, DesktopError> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to get name of first process whose frontmost is true"#)
        .output()
        .map_err(|e| DesktopError::CaptureFailed(format!("osascript: {e}")))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_macos_output(output: &str) -> Vec<AccessibleElement> {
    let mut elements = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split("|||").collect();
        if parts.len() >= 5 {
            let role = parts[0].trim().to_string();
            let title = parts[1].trim().to_string();
            let pos = parse_coords(parts[2]);
            let size = parse_coords(parts[3]);
            let enabled = parts[4].trim() == "true";
            if !role.is_empty() {
                elements.push(AccessibleElement {
                    role, title, value: None,
                    position: pos, size, is_enabled: enabled,
                });
            }
        }
    }
    elements
}

fn parse_coords(s: &str) -> (f64, f64) {
    let parts: Vec<&str> = s.trim().split(',').collect();
    let x = parts.first().and_then(|p| p.trim().parse().ok()).unwrap_or(0.0);
    let y = parts.get(1).and_then(|p| p.trim().parse().ok()).unwrap_or(0.0);
    (x, y)
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Simple Levenshtein distance for fuzzy matching (EC-2.2).
fn levenshtein(a: &str, b: &str) -> usize {
    let (m, n) = (a.len(), b.len());
    if m == 0 { return n; }
    if n == 0 { return m; }
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];
    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_exact_match() {
        assert_eq!(levenshtein("submit", "submit"), 0);
    }

    #[test]
    fn levenshtein_close_match() {
        assert_eq!(levenshtein("submit", "subm1t"), 1);
        assert!(levenshtein("submit", "subn1it") <= 2);
    }

    #[test]
    fn levenshtein_distant() {
        assert!(levenshtein("submit", "cancel") > 3);
    }

    #[test]
    fn find_by_title_exact() {
        let tree = AccessibilityTree {
            app_name: "Test".into(),
            elements: vec![
                AccessibleElement { role: "AXButton".into(), title: "Submit".into(), value: None, position: (100.0, 200.0), size: (80.0, 30.0), is_enabled: true },
                AccessibleElement { role: "AXButton".into(), title: "Cancel".into(), value: None, position: (200.0, 200.0), size: (80.0, 30.0), is_enabled: true },
            ],
        };
        let found = tree.find_by_title("Submit");
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Submit");
    }

    #[test]
    fn find_by_title_fuzzy() {
        let tree = AccessibilityTree {
            app_name: "Test".into(),
            elements: vec![
                AccessibleElement { role: "AXButton".into(), title: "Subm1t".into(), value: None, position: (100.0, 200.0), size: (80.0, 30.0), is_enabled: true },
            ],
        };
        // "Submit" vs "Subm1t" — Levenshtein = 1
        let found = tree.find_by_title("Submit");
        assert!(found.is_some());
    }

    #[test]
    fn find_nearest_picks_closest() {
        let tree = AccessibilityTree {
            app_name: "Test".into(),
            elements: vec![
                AccessibleElement { role: "AXButton".into(), title: "Submit".into(), value: None, position: (100.0, 200.0), size: (80.0, 30.0), is_enabled: true },
                AccessibleElement { role: "AXButton".into(), title: "Submit".into(), value: None, position: (500.0, 200.0), size: (80.0, 30.0), is_enabled: true },
            ],
        };
        let found = tree.find_nearest("Submit", (120.0, 210.0));
        assert!(found.is_some());
        assert!((found.unwrap().position.0 - 100.0).abs() < 1.0); // closer to first
    }

    #[test]
    fn element_center_calculation() {
        let el = AccessibleElement { role: "AXButton".into(), title: "OK".into(), value: None, position: (100.0, 200.0), size: (80.0, 40.0), is_enabled: true };
        let (cx, cy) = AccessibilityTree::element_center(&el);
        assert!((cx - 140.0).abs() < 0.1);
        assert!((cy - 220.0).abs() < 0.1);
    }

    #[test]
    fn parse_coords_works() {
        assert_eq!(parse_coords("100, 200"), (100.0, 200.0));
        assert_eq!(parse_coords("0,0"), (0.0, 0.0));
    }
}
