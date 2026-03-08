//! DivergenceDetector — detect differences between main and shadow execution.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Type of divergence detected
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DivergenceType {
    /// Output values differ
    OutputMismatch { key: String },
    /// Shadow produced additional output
    ExtraOutput { key: String },
    /// Shadow missing expected output
    MissingOutput { key: String },
    /// Success/failure status differs
    StatusDivergence,
    /// Safety assessment differs
    SafetyDivergence,
}

/// A detected divergence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Divergence {
    pub divergence_type: DivergenceType,
    pub severity: DivergenceSeverity,
    pub description: String,
    pub main_value: Option<serde_json::Value>,
    pub shadow_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DivergenceSeverity {
    Info,
    Warning,
    Critical,
}

/// Detects divergences between main and shadow execution results
pub struct DivergenceDetector;

impl DivergenceDetector {
    /// Compare main and shadow outputs
    pub fn detect(
        main_outputs: &HashMap<String, serde_json::Value>,
        shadow_outputs: &HashMap<String, serde_json::Value>,
        main_success: bool,
        shadow_success: bool,
        main_safe: bool,
        shadow_safe: bool,
    ) -> Vec<Divergence> {
        let mut divergences = Vec::new();

        // Status divergence
        if main_success != shadow_success {
            divergences.push(Divergence {
                divergence_type: DivergenceType::StatusDivergence,
                severity: DivergenceSeverity::Critical,
                description: format!(
                    "Main: {}, Shadow: {}",
                    if main_success { "success" } else { "failure" },
                    if shadow_success { "success" } else { "failure" }
                ),
                main_value: Some(serde_json::json!(main_success)),
                shadow_value: Some(serde_json::json!(shadow_success)),
            });
        }

        // Safety divergence
        if main_safe != shadow_safe {
            divergences.push(Divergence {
                divergence_type: DivergenceType::SafetyDivergence,
                severity: DivergenceSeverity::Critical,
                description: "Safety assessment differs between main and shadow".into(),
                main_value: Some(serde_json::json!(main_safe)),
                shadow_value: Some(serde_json::json!(shadow_safe)),
            });
        }

        // Output comparison
        for (key, main_val) in main_outputs {
            match shadow_outputs.get(key) {
                Some(shadow_val) if shadow_val != main_val => {
                    divergences.push(Divergence {
                        divergence_type: DivergenceType::OutputMismatch { key: key.clone() },
                        severity: DivergenceSeverity::Warning,
                        description: format!("Output '{}' differs", key),
                        main_value: Some(main_val.clone()),
                        shadow_value: Some(shadow_val.clone()),
                    });
                }
                None => {
                    divergences.push(Divergence {
                        divergence_type: DivergenceType::MissingOutput { key: key.clone() },
                        severity: DivergenceSeverity::Info,
                        description: format!("Shadow missing output '{}'", key),
                        main_value: Some(main_val.clone()),
                        shadow_value: None,
                    });
                }
                _ => {} // Match
            }
        }

        for key in shadow_outputs.keys() {
            if !main_outputs.contains_key(key) {
                divergences.push(Divergence {
                    divergence_type: DivergenceType::ExtraOutput { key: key.clone() },
                    severity: DivergenceSeverity::Info,
                    description: format!("Shadow has extra output '{}'", key),
                    main_value: None,
                    shadow_value: shadow_outputs.get(key).cloned(),
                });
            }
        }

        divergences
    }

    /// Check if any divergence is critical
    pub fn has_critical(divergences: &[Divergence]) -> bool {
        divergences
            .iter()
            .any(|d| d.severity == DivergenceSeverity::Critical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divergence_detection() {
        let main = HashMap::from([
            ("a".into(), serde_json::json!(1)),
            ("b".into(), serde_json::json!(2)),
        ]);
        let shadow = HashMap::from([
            ("a".into(), serde_json::json!(1)),
            ("b".into(), serde_json::json!(99)), // different
            ("c".into(), serde_json::json!(3)),  // extra
        ]);

        let divergences = DivergenceDetector::detect(&main, &shadow, true, true, true, true);
        assert_eq!(divergences.len(), 2); // OutputMismatch + ExtraOutput
        assert!(!DivergenceDetector::has_critical(&divergences));
    }

    #[test]
    fn test_safety_divergence() {
        let empty = HashMap::new();
        let divergences = DivergenceDetector::detect(&empty, &empty, true, true, true, false);
        assert!(DivergenceDetector::has_critical(&divergences));
        assert!(divergences
            .iter()
            .any(|d| matches!(d.divergence_type, DivergenceType::SafetyDivergence)));
    }

    #[test]
    fn test_no_divergence() {
        let same = HashMap::from([("x".into(), serde_json::json!(42))]);
        let divergences = DivergenceDetector::detect(&same, &same, true, true, true, true);
        assert!(divergences.is_empty());
    }
}
