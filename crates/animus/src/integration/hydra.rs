//! AnimusEngine — bridge between Hydra cognitive loop and Animus Prime.

use crate::compiler::{self, CompileResult, Target};
use crate::prime::ast::*;
use crate::prime::parser::PrimeParser;
use crate::prime::validator::{PrimeValidator, ValidationResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cognitive phase mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitivePhase {
    Perceive,
    Think,
    Decide,
    Act,
    Learn,
}

/// Result from processing a phase through Animus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    pub phase: CognitivePhase,
    pub prime_ast: Option<serde_json::Value>,
    pub validation: Option<ValidationSummary>,
    pub compiled: HashMap<String, String>,
}

/// Summary of validation (serializable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl From<ValidationResult> for ValidationSummary {
    fn from(r: ValidationResult) -> Self {
        Self {
            valid: r.valid,
            errors: r.errors,
            warnings: r.warnings,
        }
    }
}

/// The Animus Engine — processes cognitive loop data through Prime
pub struct AnimusEngine {
    default_targets: Vec<Target>,
}

impl AnimusEngine {
    pub fn new() -> Self {
        Self {
            default_targets: vec![],
        }
    }

    /// Create with specific compilation targets
    pub fn with_targets(targets: Vec<Target>) -> Self {
        Self {
            default_targets: targets,
        }
    }

    /// Process LLM output (JSON) through the Animus pipeline:
    /// 1. Parse JSON → Prime AST
    /// 2. Validate
    /// 3. Optionally compile to targets
    pub fn process(
        &self,
        json: &serde_json::Value,
        phase: CognitivePhase,
    ) -> Result<PhaseResult, String> {
        // 1. Parse
        let ast = PrimeParser::parse(json)?;

        // 2. Validate
        let validation = PrimeValidator::validate(&ast);
        let summary: ValidationSummary = validation.into();

        // 3. Serialize AST
        let prime_value = serde_json::to_value(&ast).map_err(|e| e.to_string())?;

        // 4. Compile to targets
        let mut compiled = HashMap::new();
        for target in &self.default_targets {
            if let Ok(result) = compiler::compile(&ast, *target) {
                let key = format!("{:?}", target).to_lowercase();
                compiled.insert(key, result.code);
            }
        }

        Ok(PhaseResult {
            phase,
            prime_ast: Some(prime_value),
            validation: Some(summary),
            compiled,
        })
    }

    /// Compile a Prime AST node to a specific target
    pub fn compile_to(&self, node: &PrimeNode, target: Target) -> Result<CompileResult, String> {
        compiler::compile(node, target)
    }

    /// Parse and validate JSON into a Prime AST
    pub fn parse_and_validate(
        &self,
        json: &serde_json::Value,
    ) -> Result<(PrimeNode, ValidationSummary), String> {
        let ast = PrimeParser::parse(json)?;
        let validation = PrimeValidator::validate(&ast);
        Ok((ast, validation.into()))
    }

    /// Map a cognitive phase to recommended compilation targets
    pub fn recommended_targets(phase: CognitivePhase) -> Vec<Target> {
        match phase {
            CognitivePhase::Perceive => vec![], // Perception = understanding, no code needed
            CognitivePhase::Think => vec![],    // Reasoning = internal, no code needed
            CognitivePhase::Decide => vec![],   // Decision = plan, no code needed
            CognitivePhase::Act => vec![Target::JavaScript, Target::Python, Target::Rust],
            CognitivePhase::Learn => vec![Target::Sql], // Learning often involves storage
        }
    }
}

impl Default for AnimusEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_entity() {
        let engine = AnimusEngine::with_targets(vec![Target::JavaScript, Target::Python]);
        let json = serde_json::json!({
            "type": "entity",
            "name": "User",
            "fields": [
                { "name": "id", "type": "uuid" },
                { "name": "name", "type": "string" },
            ]
        });
        let result = engine.process(&json, CognitivePhase::Act).unwrap();
        assert_eq!(result.phase, CognitivePhase::Act);
        assert!(result.validation.as_ref().unwrap().valid);
        assert!(result.compiled.contains_key("javascript"));
        assert!(result.compiled.contains_key("python"));
    }

    #[test]
    fn test_parse_and_validate() {
        let engine = AnimusEngine::new();
        let json = serde_json::json!({
            "type": "create",
            "entity": "User",
            "data": { "name": "Alice" }
        });
        let (node, validation) = engine.parse_and_validate(&json).unwrap();
        assert_eq!(node.type_name(), "create");
        assert!(validation.valid);
    }

    #[test]
    fn test_recommended_targets() {
        let act_targets = AnimusEngine::recommended_targets(CognitivePhase::Act);
        assert_eq!(act_targets.len(), 3);
        let learn_targets = AnimusEngine::recommended_targets(CognitivePhase::Learn);
        assert_eq!(learn_targets.len(), 1);
    }
}
