//! Assumption templates — domain-specific assumptions loaded from TOML.
//! Falls back to universal assumptions when no template matches.

use serde::Deserialize;

/// A template that maps triggers (keywords) to assumptions.
#[derive(Debug, Clone, Deserialize)]
pub struct AssumptionTemplate {
    pub trigger: String,
    pub assumptions: Vec<TemplateAssumption>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateAssumption {
    pub statement: String,
    pub severity: f64,
}

/// Load assumption templates from skill directories.
pub fn load_templates() -> Vec<AssumptionTemplate> {
    let mut templates = builtin_templates();
    // Load from ~/.hydra/skills/*/assumptions.toml
    let dir = dirs::home_dir().unwrap_or_default().join(".hydra/skills");
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path().join("assumptions.toml");
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(parsed) = toml::from_str::<TemplatesFile>(&content) {
                        templates.extend(parsed.assumption_template);
                    }
                }
            }
        }
    }
    templates
}

/// Match templates against a goal. Returns matching assumptions.
pub fn match_templates(goal: &str, templates: &[AssumptionTemplate]) -> Vec<TemplateAssumption> {
    let lower = goal.to_lowercase();
    let mut matched = Vec::new();
    for template in templates {
        let triggers: Vec<&str> = template.trigger.split('|').collect();
        if triggers.iter().any(|t| lower.contains(t.trim())) {
            matched.extend(template.assumptions.clone());
        }
    }
    matched
}

/// Universal fallback assumptions (EC-0.8).
pub fn universal_assumptions() -> Vec<TemplateAssumption> {
    vec![
        TemplateAssumption { statement: "This action is reversible".into(), severity: 0.8 },
        TemplateAssumption { statement: "No dependent systems will break".into(), severity: 0.7 },
        TemplateAssumption { statement: "A backup exists if needed".into(), severity: 0.75 },
    ]
}

#[derive(Deserialize)]
struct TemplatesFile {
    #[serde(default)]
    assumption_template: Vec<AssumptionTemplate>,
}

fn builtin_templates() -> Vec<AssumptionTemplate> {
    vec![
        AssumptionTemplate { trigger: "deploy|ship|push to prod|release".into(), assumptions: vec![
            TemplateAssumption { statement: "Tests have passed".into(), severity: 0.95 },
            TemplateAssumption { statement: "Rollback plan exists".into(), severity: 0.85 },
            TemplateAssumption { statement: "No breaking API changes".into(), severity: 0.90 },
            TemplateAssumption { statement: "Monitoring is configured".into(), severity: 0.80 },
        ]},
        AssumptionTemplate { trigger: "delete|remove|drop|destroy".into(), assumptions: vec![
            TemplateAssumption { statement: "Backup exists".into(), severity: 0.99 },
            TemplateAssumption { statement: "No dependent systems".into(), severity: 0.95 },
            TemplateAssumption { statement: "Action is reversible".into(), severity: 0.90 },
        ]},
        AssumptionTemplate { trigger: "send email|email client|reply to".into(), assumptions: vec![
            TemplateAssumption { statement: "Recipient is correct".into(), severity: 0.90 },
            TemplateAssumption { statement: "Tone matches relationship".into(), severity: 0.70 },
        ]},
        AssumptionTemplate { trigger: "install|npm install|pip install|cargo add".into(), assumptions: vec![
            TemplateAssumption { statement: "Package is from trusted source".into(), severity: 0.85 },
            TemplateAssumption { statement: "Version compatibility checked".into(), severity: 0.75 },
        ]},
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_deploy_template() {
        let templates = builtin_templates();
        let matched = match_templates("deploy to production", &templates);
        assert!(matched.len() >= 3);
        assert!(matched.iter().any(|a| a.statement.contains("Tests")));
    }

    #[test]
    fn match_delete_template() {
        let templates = builtin_templates();
        let matched = match_templates("delete the database", &templates);
        assert!(matched.iter().any(|a| a.statement.contains("Backup")));
    }

    #[test]
    fn no_match_returns_empty() {
        let templates = builtin_templates();
        let matched = match_templates("what is rust?", &templates);
        assert!(matched.is_empty());
    }

    #[test]
    fn universal_fallback_exists() {
        let uni = universal_assumptions();
        assert!(uni.len() >= 3);
    }
}
