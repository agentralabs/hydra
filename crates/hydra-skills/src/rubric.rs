//! Quality rubric — TOML-defined quality dimensions for the critic loop.
//! Each skill can define domain-specific quality criteria.

use serde::Deserialize;

/// A quality rubric defining evaluation dimensions.
#[derive(Debug, Clone)]
pub struct QualityRubric {
    pub dimensions: Vec<QualityDimension>,
    pub threshold: f64,
    pub max_revisions: u32,
}

impl QualityRubric {
    /// Normalize weights to sum to 1.0 (EC-5.4).
    pub fn normalize_weights(&mut self) {
        let total: f64 = self.dimensions.iter().map(|d| d.weight).sum();
        if total > 0.0 && (total - 1.0).abs() > 0.01 {
            eprintln!("hydra-rubric: weights sum to {total:.2}, normalizing to 1.0");
            for d in &mut self.dimensions { d.weight /= total; }
        }
    }
}

impl Default for QualityRubric {
    fn default() -> Self {
        Self {
            dimensions: vec![
                QualityDimension { name: "completeness".into(), weight: 0.4, check: CheckMethod::SemanticCheck { expected: "complete, all requirements met".into() } },
                QualityDimension { name: "correctness".into(), weight: 0.4, check: CheckMethod::SemanticCheck { expected: "no errors, works as expected".into() } },
                QualityDimension { name: "quality".into(), weight: 0.2, check: CheckMethod::SemanticCheck { expected: "clean, well-structured, professional".into() } },
            ],
            threshold: 8.0,
            max_revisions: 3,
        }
    }
}

/// A single quality dimension with scoring method.
#[derive(Debug, Clone)]
pub struct QualityDimension {
    pub name: String,
    pub weight: f64,
    pub check: CheckMethod,
}

/// How to evaluate a quality dimension.
#[derive(Debug, Clone)]
pub enum CheckMethod {
    Command { command: String },
    FilePattern { pattern: String },
    SemanticCheck { expected: String },
    GenomeRule { rule: String },
    /// O13: Aesthetic evaluation against design rules for a category.
    AestheticCheck { category: String },
}

/// Parse a quality rubric from TOML.
pub fn parse_rubric(toml_content: &str) -> Result<QualityRubric, String> {
    let parsed: RubricFile = toml::from_str(toml_content).map_err(|e| format!("Rubric TOML: {e}"))?;
    let dimensions: Vec<QualityDimension> = parsed.quality.dimensions.into_iter().map(|d| {
        let check = match d.check.check_type.as_str() {
            "command" => CheckMethod::Command { command: d.check.command.unwrap_or_default() },
            "file" => CheckMethod::FilePattern { pattern: d.check.pattern.unwrap_or_default() },
            "semantic" => CheckMethod::SemanticCheck { expected: d.check.expected.unwrap_or_default() },
            "genome" => CheckMethod::GenomeRule { rule: d.check.rule.unwrap_or_default() },
            "aesthetic" => CheckMethod::AestheticCheck { category: d.check.category.unwrap_or_default() },
            _ => CheckMethod::SemanticCheck { expected: d.name.clone() },
        };
        QualityDimension { name: d.name, weight: d.weight, check }
    }).collect();

    let mut rubric = QualityRubric {
        dimensions,
        threshold: parsed.quality.threshold.unwrap_or(8.0),
        max_revisions: parsed.quality.max_revisions.unwrap_or(3),
    };
    rubric.normalize_weights();
    Ok(rubric)
}

#[derive(Deserialize)]
struct RubricFile { quality: RawQuality }
#[derive(Deserialize)]
struct RawQuality {
    threshold: Option<f64>,
    max_revisions: Option<u32>,
    #[serde(default)]
    dimensions: Vec<RawDimension>,
}
#[derive(Deserialize)]
struct RawDimension { name: String, weight: f64, check: RawCheck }
#[derive(Deserialize)]
struct RawCheck {
    #[serde(rename = "type")]
    check_type: String,
    command: Option<String>,
    pattern: Option<String>,
    expected: Option<String>,
    rule: Option<String>,
    category: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_code_rubric() {
        let toml = r#"
        [quality]
        threshold = 8.0
        max_revisions = 3
        [[quality.dimensions]]
        name = "compiles"
        weight = 0.5
        check = { type = "command", command = "cargo build" }
        [[quality.dimensions]]
        name = "readable"
        weight = 0.5
        check = { type = "semantic", expected = "clean code" }
        "#;
        let rubric = parse_rubric(toml).unwrap();
        assert_eq!(rubric.dimensions.len(), 2);
        assert_eq!(rubric.threshold, 8.0);
    }

    #[test]
    fn normalize_weights() {
        let mut rubric = QualityRubric {
            dimensions: vec![
                QualityDimension { name: "a".into(), weight: 0.3, check: CheckMethod::SemanticCheck { expected: "".into() } },
                QualityDimension { name: "b".into(), weight: 0.3, check: CheckMethod::SemanticCheck { expected: "".into() } },
            ],
            threshold: 8.0, max_revisions: 3,
        };
        rubric.normalize_weights();
        let total: f64 = rubric.dimensions.iter().map(|d| d.weight).sum();
        assert!((total - 1.0).abs() < 0.01);
    }

    #[test]
    fn default_rubric_has_dimensions() {
        let rubric = QualityRubric::default();
        assert_eq!(rubric.dimensions.len(), 3);
        assert_eq!(rubric.threshold, 8.0);
    }
}
