//! O13 Aesthetic genome — TOML-parsed design rules for visual quality evaluation.
//! Loads aesthetic entries from skills directories and evaluates content against rules.

use serde::Deserialize;

/// A single aesthetic rule entry from TOML.
#[derive(Debug, Clone)]
pub struct AestheticEntry {
    pub category: String,
    pub context: String,
    pub rules: Vec<String>,
    pub confidence: f64,
}

#[derive(Deserialize)]
struct AestheticFile {
    #[serde(default)]
    aesthetic: Vec<RawEntry>,
}

#[derive(Deserialize)]
struct RawEntry {
    category: String,
    #[serde(default)]
    context: String,
    #[serde(default)]
    rules: Vec<String>,
    #[serde(default = "default_confidence")]
    confidence: f64,
}

fn default_confidence() -> f64 { 0.8 }

/// Load all aesthetic genome entries from skills/design/ TOML files.
pub fn load_aesthetic_genome() -> Vec<AestheticEntry> {
    let base = dirs::home_dir().unwrap_or_default().join(".hydra/skills/design");
    let mut entries = Vec::new();
    if let Ok(dir) = std::fs::read_dir(&base) {
        for file in dir.flatten() {
            if file.path().extension().map(|e| e == "toml").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(file.path()) {
                    if let Ok(parsed) = toml::from_str::<AestheticFile>(&content) {
                        for raw in parsed.aesthetic {
                            entries.push(AestheticEntry {
                                category: raw.category, context: raw.context,
                                rules: raw.rules, confidence: raw.confidence,
                            });
                        }
                    }
                }
            }
        }
    }
    // Universal fallback rules (always available, EC-13.4)
    if entries.is_empty() {
        entries.push(AestheticEntry {
            category: "universal".into(), context: "any design".into(),
            rules: vec![
                "Maintain visual hierarchy — most important element is most prominent".into(),
                "Use consistent spacing — 8px grid recommended".into(),
                "Limit color palette to 4-5 colors maximum".into(),
                "Ensure sufficient contrast for readability".into(),
            ],
            confidence: 0.7,
        });
    }
    entries
}

/// Get rules for a specific aesthetic category.
pub fn rules_for_category(entries: &[AestheticEntry], category: &str) -> Vec<String> {
    let cat = category.to_lowercase();
    let mut rules: Vec<String> = entries.iter()
        .filter(|e| e.category.to_lowercase() == cat || e.category == "universal")
        .flat_map(|e| e.rules.clone())
        .collect();
    rules.dedup();
    rules
}

/// Evaluate content against aesthetic rules. Returns (score 0.0-1.0, issues list).
pub fn evaluate_against_rules(content: &str, rules: &[String]) -> (f64, Vec<String>) {
    if rules.is_empty() { return (0.7, vec!["No aesthetic rules available".into()]); }
    let lower = content.to_lowercase();
    let mut issues = Vec::new();
    let mut violations = 0;
    for rule in rules {
        let rule_lower = rule.to_lowercase();
        // Check if content explicitly violates known anti-patterns
        if rule_lower.contains("limit color") && lower.contains("10 colors") { violations += 1; issues.push(format!("Violates: {rule}")); }
        if rule_lower.contains("contrast") && lower.contains("low contrast") { violations += 1; issues.push(format!("Violates: {rule}")); }
        if rule_lower.contains("hierarchy") && lower.contains("no hierarchy") { violations += 1; issues.push(format!("Violates: {rule}")); }
    }
    let score = 1.0 - (violations as f64 / rules.len().max(1) as f64);
    (score.max(0.0), issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn universal_fallback_always_available() {
        let entries = load_aesthetic_genome();
        assert!(!entries.is_empty()); // At minimum universal rules
    }

    #[test]
    fn rules_for_category_filters() {
        let entries = vec![
            AestheticEntry { category: "color".into(), context: "web".into(),
                rules: vec!["max 4 colors".into()], confidence: 0.9 },
            AestheticEntry { category: "layout".into(), context: "web".into(),
                rules: vec!["use grid".into()], confidence: 0.9 },
        ];
        let color_rules = rules_for_category(&entries, "color");
        assert!(color_rules.iter().any(|r| r.contains("4 colors")));
        assert!(!color_rules.iter().any(|r| r.contains("grid")));
    }

    #[test]
    fn evaluate_clean_content_scores_high() {
        let rules = vec!["Maintain visual hierarchy".into(), "Use consistent spacing".into()];
        let (score, issues) = evaluate_against_rules("A well-designed page with clear hierarchy", &rules);
        assert!(score >= 0.8);
        assert!(issues.is_empty());
    }

    #[test]
    fn empty_rules_return_neutral() {
        let (score, issues) = evaluate_against_rules("anything", &[]);
        assert_eq!(score, 0.7); // EC-13.4 fallback
        assert!(!issues.is_empty());
    }
}
