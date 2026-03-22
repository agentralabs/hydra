//! Persona blending — weighted combination of multiple personas.

use serde::{Deserialize, Serialize};

use crate::constants::{BLEND_WEIGHT_TOLERANCE, MAX_BLEND_PERSONAS};
use crate::errors::PersonaError;
use crate::persona::Persona;

/// A single component of a persona blend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlendComponent {
    /// The persona name.
    pub persona_name: String,
    /// Weight in the blend (0.0 to 1.0).
    pub weight: f64,
}

/// A weighted blend of personas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaBlend {
    /// The components of this blend.
    pub components: Vec<BlendComponent>,
}

impl PersonaBlend {
    /// Create a blend with a single persona at weight 1.0.
    pub fn single(persona_name: impl Into<String>) -> Self {
        Self {
            components: vec![BlendComponent {
                persona_name: persona_name.into(),
                weight: 1.0,
            }],
        }
    }

    /// Create a weighted blend from name-weight pairs.
    /// Validates that weights sum to 1.0 within tolerance and count is within limit.
    pub fn weighted(components: Vec<(String, f64)>) -> Result<Self, PersonaError> {
        if components.len() > MAX_BLEND_PERSONAS {
            return Err(PersonaError::BlendTooLarge {
                count: components.len(),
                max: MAX_BLEND_PERSONAS,
            });
        }

        let sum: f64 = components.iter().map(|(_, w)| w).sum();
        if (sum - 1.0).abs() > BLEND_WEIGHT_TOLERANCE {
            return Err(PersonaError::InvalidBlendWeights {
                sum,
                tolerance: BLEND_WEIGHT_TOLERANCE,
            });
        }

        let blend_components = components
            .into_iter()
            .map(|(name, weight)| BlendComponent {
                persona_name: name,
                weight,
            })
            .collect();

        Ok(Self {
            components: blend_components,
        })
    }

    /// Produce a blended voice description from the given personas.
    /// The personas slice must correspond to the components in order.
    pub fn blended_voice(&self, personas: &[&Persona]) -> BlendedVoice {
        let mut vocabulary = Vec::new();
        let mut priorities = Vec::new();
        let mut tone_parts = Vec::new();

        for (i, component) in self.components.iter().enumerate() {
            if let Some(persona) = personas.get(i) {
                // Add vocabulary weighted by component weight
                for word in &persona.vocabulary {
                    vocabulary.push((word.clone(), component.weight));
                }
                // Add priorities weighted by component weight
                for priority in &persona.priorities {
                    priorities.push((priority.clone(), component.weight));
                }
                tone_parts.push(format!("{:.0}% {}", component.weight * 100.0, persona.tone));
            }
        }

        // Sort by weight descending for both
        vocabulary.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        priorities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        BlendedVoice {
            vocabulary: vocabulary.into_iter().map(|(w, _)| w).collect(),
            priorities: priorities.into_iter().map(|(p, _)| p).collect(),
            tone: tone_parts.join(", "),
        }
    }

    /// Return the name of the dominant persona (highest weight).
    pub fn dominant(&self) -> Option<&str> {
        self.components
            .iter()
            .max_by(|a, b| {
                a.weight
                    .partial_cmp(&b.weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|c| c.persona_name.as_str())
    }
}

/// The result of blending multiple personas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlendedVoice {
    /// Combined vocabulary from all personas, sorted by weight.
    pub vocabulary: Vec<String>,
    /// Combined priorities from all personas, sorted by weight.
    pub priorities: Vec<String>,
    /// Combined tone description.
    pub tone: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_blend_has_weight_one() {
        let blend = PersonaBlend::single("core");
        assert_eq!(blend.components.len(), 1);
        assert!((blend.components[0].weight - 1.0).abs() < 1e-10);
    }

    #[test]
    fn weighted_blend_validates_sum() {
        let result = PersonaBlend::weighted(vec![("a".into(), 0.5), ("b".into(), 0.3)]);
        assert!(result.is_err());
    }

    #[test]
    fn weighted_blend_accepts_valid_weights() {
        let result = PersonaBlend::weighted(vec![("a".into(), 0.6), ("b".into(), 0.4)]);
        assert!(result.is_ok());
    }

    #[test]
    fn dominant_returns_highest_weight() {
        let blend = PersonaBlend::weighted(vec![("minor".into(), 0.3), ("major".into(), 0.7)])
            .expect("valid");
        assert_eq!(blend.dominant(), Some("major"));
    }

    #[test]
    fn blend_too_large_rejected() {
        let result = PersonaBlend::weighted(vec![
            ("a".into(), 0.2),
            ("b".into(), 0.2),
            ("c".into(), 0.2),
            ("d".into(), 0.2),
            ("e".into(), 0.2),
        ]);
        assert!(result.is_err());
    }
}
